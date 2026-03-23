use std::fs;
use std::path::Path;

use git2::Oid;
use orion_conf::{ToStructError, UvsConfFrom};
use serde::{Deserialize, Serialize};
use wp_error::run_error::{RunReason, RunResult};
use wp_log::{info_ctrl, warn_ctrl};

mod managed;
mod repo;
mod state;

use self::managed::{
    backup_managed_dirs, managed_dirs_differ, restore_managed_dirs, sync_managed_dirs,
};
use self::repo::{
    checkout_commit, fetch_remote_tags, prepare_remote_repo, resolve_default_target,
    resolve_tag_for_version,
};
pub use self::state::{
    acquire_project_remote_lock, capture_project_remote_snapshot,
    capture_runtime_artifact_snapshot, restore_project_remote_snapshot,
    restore_project_remote_update, restore_runtime_artifact_snapshot,
};
use self::state::{load_engine_config, load_state, persist_state, restore_project_remote_state};

const ENGINE_CONF_PATH: &str = "conf/wparse.toml";
const STATE_PATH: &str = ".run/project_remote_state.json";
const REMOTE_CACHE_PATH: &str = ".run/project_remote/remote";
const BACKUP_PATH: &str = ".run/project_remote/backup";
const BACKUP_MANIFEST_PATH: &str = ".run/project_remote/backup/manifest.json";
const LOCK_PATH: &str = ".run/project_remote.lock";
const MANAGED_DIRS: &[&str] = &["conf", "models", "topology", "connectors"];
const RULE_MAPPING_PATH: &str = ".run/rule_mapping.dat";
const AUTHORITY_DB_PATH: &str = ".run/authority.sqlite";

#[derive(Debug, Clone, Serialize)]
pub struct ProjectRemoteUpdateResult {
    pub requested_version: Option<String>,
    pub current_version: String,
    pub resolved_tag: String,
    pub from_revision: Option<String>,
    pub to_revision: String,
    pub changed: bool,
}

#[derive(Debug, Clone)]
pub struct ProjectRemoteSnapshot {
    state_file: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct ProjectRuntimeArtifactSnapshot {
    rule_mapping: Option<Vec<u8>>,
    authority_db: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct ProjectRemoteLockGuard {
    file: fs::File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectRemoteState {
    current_version: String,
    resolved_tag: String,
    revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupManifest {
    existing_dirs: Vec<String>,
}

struct ResolvedTag {
    tag: String,
    version: String,
    commit_id: Oid,
}

pub fn sync_project_remote<P: AsRef<Path>>(
    work_root: P,
    requested_version: Option<&str>,
) -> RunResult<ProjectRemoteUpdateResult> {
    let work_root = work_root.as_ref();
    let conf = load_engine_config(work_root)?;
    let remote_conf = conf.project_remote();
    if !remote_conf.enabled {
        return Err(conf_err(format!(
            "project_remote is disabled in {}",
            work_root.join(ENGINE_CONF_PATH).display()
        )));
    }
    if remote_conf.repo.trim().is_empty() {
        return Err(conf_err("project_remote.repo must not be empty"));
    }
    sync_project_remote_with_repo_inner(
        work_root,
        &remote_conf.repo,
        requested_version,
        Some(remote_conf.init_version.as_str()),
    )
}

pub fn sync_project_remote_from_repo<P: AsRef<Path>>(
    work_root: P,
    repo_url: &str,
    requested_version: Option<&str>,
) -> RunResult<ProjectRemoteUpdateResult> {
    let work_root = work_root.as_ref();
    if repo_url.trim().is_empty() {
        return Err(conf_err("project_remote.repo must not be empty"));
    }
    sync_project_remote_with_repo_inner(work_root, repo_url, requested_version, None)
}

pub fn current_project_version<P: AsRef<Path>>(work_root: P) -> RunResult<Option<String>> {
    Ok(load_state(work_root.as_ref())?.map(|state| state.current_version))
}

fn sync_project_remote_with_repo_inner(
    work_root: &Path,
    repo_url: &str,
    requested_version: Option<&str>,
    init_version: Option<&str>,
) -> RunResult<ProjectRemoteUpdateResult> {
    info_ctrl!(
        "project remote sync start work_root={} requested_version={} repo={}",
        work_root.display(),
        requested_version.unwrap_or("(auto)"),
        repo_url
    );

    let remote_root = work_root.join(REMOTE_CACHE_PATH);
    let repo = prepare_remote_repo(&remote_root, repo_url)?;
    fetch_remote_tags(&repo, repo_url)?;

    let previous_state = load_state(work_root)?;
    let resolved = match requested_version {
        Some(version) if !version.trim().is_empty() => {
            let target_version = version.trim().to_string();
            info_ctrl!(
                "project remote sync target resolved work_root={} requested_version={} target_version={} init_version={} state_exists={}",
                work_root.display(),
                requested_version.unwrap_or("(auto)"),
                target_version,
                init_version.unwrap_or("-"),
                previous_state.is_some()
            );
            resolve_tag_for_version(&repo, &target_version)?.ok_or_else(|| {
                conf_err(format!(
                    "requested version '{}' was not found",
                    target_version
                ))
            })?
        }
        _ => {
            let resolved = resolve_default_target(work_root, &repo, init_version.map(str::trim))?;
            info_ctrl!(
                "project remote sync target resolved work_root={} requested_version={} target_version={} init_version={} state_exists={}",
                work_root.display(),
                requested_version.unwrap_or("(auto)"),
                resolved.version,
                init_version.unwrap_or("-"),
                previous_state.is_some()
            );
            resolved
        }
    };
    info_ctrl!(
        "project remote sync tag resolved work_root={} requested_version={} current_version={} resolved_tag={} to_revision={}",
        work_root.display(),
        requested_version.unwrap_or("(auto)"),
        resolved.version,
        resolved.tag,
        resolved.commit_id
    );

    checkout_commit(&repo, resolved.commit_id, &resolved.tag)?;

    let changed = managed_dirs_differ(&remote_root, work_root)?;
    info_ctrl!(
        "project remote sync diff work_root={} requested_version={} changed={} from_revision={} to_revision={}",
        work_root.display(),
        requested_version.unwrap_or("(auto)"),
        changed,
        previous_state
            .as_ref()
            .map(|state| state.revision.as_str())
            .unwrap_or("-"),
        resolved.commit_id
    );
    if changed {
        info_ctrl!(
            "project remote sync backup managed dirs work_root={} dirs={}",
            work_root.display(),
            MANAGED_DIRS.join(",")
        );
        backup_managed_dirs(work_root)?;
    }

    let result = ProjectRemoteUpdateResult {
        requested_version: requested_version.map(str::to_string),
        current_version: resolved.version,
        resolved_tag: resolved.tag,
        from_revision: previous_state.as_ref().map(|state| state.revision.clone()),
        to_revision: oid_to_string(resolved.commit_id),
        changed,
    };
    let apply_result = (|| {
        if changed {
            info_ctrl!(
                "project remote sync apply managed dirs work_root={} remote_cache={}",
                work_root.display(),
                remote_root.display()
            );
            sync_managed_dirs(&remote_root, work_root)?;
        }
        persist_state(work_root, &result)?;
        Ok(())
    })();
    if let Err(err) = apply_result {
        warn_ctrl!(
            "project remote sync apply failed work_root={} requested_version={} current_version={} resolved_tag={} changed={} error={}",
            work_root.display(),
            requested_version.unwrap_or("(auto)"),
            result.current_version,
            result.resolved_tag,
            result.changed,
            err
        );
        rollback_partial_update(work_root, previous_state.as_ref(), changed).map_err(
            |rollback_err| conf_err(format!("{}; rollback failed: {}", err, rollback_err)),
        )?;
        warn_ctrl!(
            "project remote sync rollback done work_root={} requested_version={} current_version={} resolved_tag={} changed={}",
            work_root.display(),
            requested_version.unwrap_or("(auto)"),
            result.current_version,
            result.resolved_tag,
            changed
        );
        return Err(err);
    }
    info_ctrl!(
        "project remote sync done work_root={} requested_version={} current_version={} resolved_tag={} from_revision={} to_revision={} changed={}",
        work_root.display(),
        requested_version.unwrap_or("(auto)"),
        result.current_version,
        result.resolved_tag,
        result.from_revision.as_deref().unwrap_or("-"),
        result.to_revision,
        result.changed
    );
    Ok(result)
}

fn rollback_partial_update(
    work_root: &Path,
    previous_state: Option<&ProjectRemoteState>,
    changed: bool,
) -> RunResult<()> {
    if changed {
        restore_managed_dirs(work_root)?;
    }
    restore_project_remote_state(work_root, previous_state)
}

fn oid_to_string(oid: Oid) -> String {
    oid.to_string()
}

fn conf_err(message: impl Into<String>) -> wp_error::RunError {
    RunReason::from_conf().to_err().with_detail(message.into())
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests {
    use super::test_support::{
        create_empty_managed_dirs, create_remote_fixture, create_remote_fixture_without_tags,
        create_work_root, write_engine_conf_with_init_version, write_model_version,
        write_runtime_local_dirs,
    };
    use super::*;
    use std::fs;

    #[test]
    fn sync_project_remote_updates_to_requested_version_and_persists_state() {
        let fixture = create_remote_fixture();
        let work_root = create_work_root(&fixture);
        write_model_version(work_root.path(), "1.4.2");
        write_runtime_local_dirs(work_root.path());

        let result = sync_project_remote(work_root.path(), Some("1.4.3")).expect("sync remote");

        assert_eq!(result.requested_version.as_deref(), Some("1.4.3"));
        assert_eq!(result.current_version, "1.4.3");
        assert_eq!(result.resolved_tag, "v1.4.3");
        assert!(result.changed);
        assert_eq!(
            fs::read_to_string(work_root.path().join("models/version.txt")).expect("read version"),
            "1.4.3\n"
        );
        assert_eq!(
            fs::read_to_string(work_root.path().join("runtime/admin_api.token"))
                .expect("read token"),
            "token\n"
        );

        let state: serde_json::Value = serde_json::from_slice(
            &fs::read(work_root.path().join(STATE_PATH)).expect("read state file"),
        )
        .expect("parse state json");
        assert_eq!(state["current_version"], "1.4.3");
        assert_eq!(state["resolved_tag"], "v1.4.3");
        assert_eq!(state["revision"], result.to_revision);
    }

    #[test]
    fn sync_project_remote_uses_init_version_when_state_file_is_missing() {
        let fixture = create_remote_fixture();
        let work_root = create_work_root(&fixture);
        create_empty_managed_dirs(work_root.path());

        let result = sync_project_remote(work_root.path(), None).expect("sync remote");

        assert_eq!(result.requested_version, None);
        assert_eq!(result.current_version, "1.4.2");
        assert_eq!(result.resolved_tag, "v1.4.2");
    }

    #[test]
    fn sync_project_remote_uses_latest_release_when_state_file_exists() {
        let fixture = create_remote_fixture();
        let work_root = create_work_root(&fixture);
        write_model_version(work_root.path(), "1.4.2");
        persist_state(
            work_root.path(),
            &ProjectRemoteUpdateResult {
                requested_version: Some("1.4.2".to_string()),
                current_version: "1.4.2".to_string(),
                resolved_tag: "v1.4.2".to_string(),
                from_revision: None,
                to_revision: "old-revision".to_string(),
                changed: false,
            },
        )
        .expect("persist prior state");

        let result = sync_project_remote(work_root.path(), None).expect("sync remote");

        assert_eq!(result.requested_version, None);
        assert_eq!(result.current_version, "1.4.3");
        assert_eq!(result.resolved_tag, "v1.4.3");
    }

    #[test]
    fn sync_project_remote_falls_back_to_remote_head_when_no_release_tags_exist() {
        let fixture = create_remote_fixture_without_tags();
        let work_root = create_work_root(&fixture);
        write_engine_conf_with_init_version(work_root.path(), fixture.repo_url(), "");
        create_empty_managed_dirs(work_root.path());

        let result = sync_project_remote(work_root.path(), None).expect("sync remote");

        assert_eq!(result.requested_version, None);
        assert!(result.resolved_tag.starts_with("HEAD@"));
        assert_eq!(
            result.current_version,
            result.resolved_tag.trim_start_matches("HEAD@")
        );
        assert_eq!(
            fs::read_to_string(work_root.path().join("models/version.txt")).expect("read version"),
            "head\n"
        );
    }

    #[test]
    fn sync_project_remote_preserves_runtime_local_dirs() {
        let fixture = create_remote_fixture();
        let work_root = create_work_root(&fixture);
        write_model_version(work_root.path(), "1.4.2");
        write_runtime_local_dirs(work_root.path());

        sync_project_remote(work_root.path(), Some("1.4.3")).expect("sync remote");

        assert_eq!(
            fs::read_to_string(work_root.path().join("runtime/admin_api.token"))
                .expect("read token"),
            "token\n"
        );
        assert_eq!(
            fs::read_to_string(work_root.path().join("data/local.dat")).expect("read local data"),
            "local\n"
        );
    }

    #[test]
    fn sync_project_remote_initializes_non_git_work_root() {
        let fixture = create_remote_fixture();
        let work_root = create_work_root(&fixture);
        write_runtime_local_dirs(work_root.path());

        let result =
            sync_project_remote(work_root.path(), Some("1.4.2")).expect("sync should initialize");

        assert_eq!(result.current_version, "1.4.2");
        assert_eq!(result.resolved_tag, "v1.4.2");
        assert!(!work_root.path().join(".git").exists());
        assert!(work_root
            .path()
            .join(REMOTE_CACHE_PATH)
            .join(".git")
            .exists());
        assert_eq!(
            fs::read_to_string(work_root.path().join("models/version.txt")).expect("read version"),
            "1.4.2\n"
        );
    }

    #[test]
    fn restore_project_remote_snapshot_restores_managed_dirs_only() {
        let fixture = create_remote_fixture();
        let work_root = create_work_root(&fixture);
        write_model_version(work_root.path(), "1.4.2");
        write_runtime_local_dirs(work_root.path());

        let snapshot = capture_project_remote_snapshot(work_root.path()).expect("capture snapshot");
        sync_project_remote(work_root.path(), Some("1.4.3")).expect("sync remote");
        restore_project_remote_snapshot(work_root.path(), &snapshot).expect("restore snapshot");

        assert_eq!(
            fs::read_to_string(work_root.path().join("models/version.txt")).expect("read version"),
            "1.4.2\n"
        );
        assert_eq!(
            fs::read_to_string(work_root.path().join("runtime/admin_api.token"))
                .expect("read token"),
            "token\n"
        );
    }

    #[test]
    fn restore_project_remote_snapshot_without_backup_manifest_restores_state_only() {
        let fixture = create_remote_fixture();
        let work_root = create_work_root(&fixture);
        write_model_version(work_root.path(), "1.4.2");

        let snapshot = capture_project_remote_snapshot(work_root.path()).expect("capture snapshot");
        let result = sync_project_remote(work_root.path(), Some("1.4.2")).expect("sync remote");
        assert!(!result.changed);
        assert!(work_root.path().join(STATE_PATH).exists());

        restore_project_remote_snapshot(work_root.path(), &snapshot).expect("restore snapshot");

        assert!(!work_root.path().join(STATE_PATH).exists());
        assert_eq!(
            fs::read_to_string(work_root.path().join("models/version.txt")).expect("read version"),
            "1.4.2\n"
        );
    }

    #[test]
    fn restore_project_remote_update_skips_stale_backup_when_update_did_not_change_dirs() {
        let fixture = create_remote_fixture();
        let work_root = create_work_root(&fixture);
        write_model_version(work_root.path(), "1.4.2");

        sync_project_remote(work_root.path(), Some("1.4.3")).expect("sync to latest");
        assert_eq!(
            fs::read_to_string(work_root.path().join("models/version.txt")).expect("read version"),
            "1.4.3\n"
        );

        let snapshot = capture_project_remote_snapshot(work_root.path()).expect("capture snapshot");
        let result = sync_project_remote(work_root.path(), Some("1.4.3")).expect("sync unchanged");
        assert!(!result.changed);

        restore_project_remote_update(work_root.path(), &snapshot, result.changed)
            .expect("restore snapshot");

        assert_eq!(
            fs::read_to_string(work_root.path().join("models/version.txt")).expect("read version"),
            "1.4.3\n"
        );
    }

    #[test]
    fn sync_project_remote_rolls_back_when_persist_state_fails() {
        let fixture = create_remote_fixture();
        let work_root = create_work_root(&fixture);
        write_model_version(work_root.path(), "1.4.2");
        fs::create_dir_all(work_root.path().join(".run")).expect("create run dir");
        fs::write(
            work_root.path().join(STATE_PATH),
            r#"{
  "current_version": "1.4.2",
  "resolved_tag": "v1.4.2",
  "revision": "old-revision"
}"#,
        )
        .expect("write prior state");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut perms = fs::metadata(work_root.path().join(STATE_PATH))
                .expect("stat state file")
                .permissions();
            perms.set_mode(0o400);
            fs::set_permissions(work_root.path().join(STATE_PATH), perms)
                .expect("chmod state file");
        }

        let err =
            sync_project_remote(work_root.path(), Some("1.4.3")).expect_err("sync should fail");
        assert!(
            err.to_string().contains("write"),
            "unexpected error: {}",
            err
        );
        assert_eq!(
            fs::read_to_string(work_root.path().join("models/version.txt")).expect("read version"),
            "1.4.2\n"
        );
        let state: serde_json::Value = serde_json::from_slice(
            &fs::read(work_root.path().join(STATE_PATH)).expect("read state file"),
        )
        .expect("parse state json");
        assert_eq!(state["current_version"], "1.4.2");
    }

    #[test]
    fn acquire_project_remote_lock_rejects_second_holder() {
        let work_root = tempfile::tempdir().expect("tempdir");

        let _first = acquire_project_remote_lock(work_root.path()).expect("acquire first lock");
        let err =
            acquire_project_remote_lock(work_root.path()).expect_err("second lock should fail");
        assert!(
            err.to_string()
                .contains("project remote update already in progress"),
            "unexpected lock error: {}",
            err
        );
    }
}
