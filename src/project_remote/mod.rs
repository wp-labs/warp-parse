use std::fs;
use std::path::Path;

use git2::Oid;
use orion_conf::{ToStructError, UvsConfFrom};
use orion_error::WrapStructErrorAs;
use orion_variate::EnvDict;
use serde::{Deserialize, Serialize};
use wp_config::engine::{ProjectRemoteConf, RepoGroupConf};
use wp_error::run_error::{RunReason, RunResult};
use wp_log::{info_ctrl, warn_ctrl};

mod managed;
mod repo;
mod state;

use self::managed::{
    backup_managed_dirs, managed_dirs_differ, managed_dirs_for, restore_managed_dirs,
    sync_managed_dirs,
};
use self::repo::{
    checkout_commit, fetch_remote_tags, prepare_remote_repo, resolve_default_target,
    resolve_tag_for_version,
};
pub use self::state::{
    acquire_project_remote_lock, capture_project_remote_snapshot,
    capture_project_remote_snapshot_with_group, capture_runtime_artifact_snapshot,
    restore_project_remote_snapshot, restore_project_remote_update,
    restore_runtime_artifact_snapshot,
};
use self::state::{
    load_engine_config, load_state, persist_group_state, persist_state,
    restore_project_remote_state,
};

const ENGINE_CONF_PATH: &str = "conf/wparse.toml";
const STATE_PATH: &str = ".run/project_remote_state.json";
const REMOTE_CACHE_PATH: &str = ".run/project_remote/remote";
const REMOTE_CACHE_PATH_MODELS: &str = ".run/project_remote/remote-models";
const REMOTE_CACHE_PATH_INFRA: &str = ".run/project_remote/remote-infra";
const BACKUP_PATH: &str = ".run/project_remote/backup";
const BACKUP_MANIFEST_PATH: &str = ".run/project_remote/backup/manifest.json";
const LOCK_PATH: &str = ".run/project_remote.lock";
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectRemoteSnapshot {
    state_file: Option<Vec<u8>>,
    pub group: Option<RemoteGroup>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteGroup {
    Models,
    Infra,
}

impl std::str::FromStr for RemoteGroup {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "models" => Ok(RemoteGroup::Models),
            "infra" => Ok(RemoteGroup::Infra),
            other => Err(format!(
                "invalid group '{}': expected 'models' or 'infra'",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GroupState {
    #[serde(rename = "version")]
    current_version: String,
    #[serde(rename = "tag")]
    resolved_tag: String,
    revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum ProjectRemoteState {
    Single {
        current_version: String,
        resolved_tag: String,
        revision: String,
    },
    Dual {
        models: Option<GroupState>,
        infra: Option<GroupState>,
    },
}

impl ProjectRemoteState {
    fn single_version(&self) -> Option<&str> {
        match self {
            ProjectRemoteState::Single {
                current_version, ..
            } => Some(current_version.as_str()),
            ProjectRemoteState::Dual { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupManifest {
    existing_dirs: Vec<String>,
}

pub(crate) enum ProjectRemoteMode {
    Single {
        repo: String,
        init_version: String,
    },
    Dual {
        models: RepoGroupConf,
        infra: RepoGroupConf,
    },
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
    let dict = crate::load_sec_dict()?;
    sync_project_remote_with_dict(work_root, requested_version, &dict)
}

pub fn sync_project_remote_with_dict<P: AsRef<Path>>(
    work_root: P,
    requested_version: Option<&str>,
    dict: &EnvDict,
) -> RunResult<ProjectRemoteUpdateResult> {
    let work_root = work_root.as_ref();
    let conf = load_engine_config(work_root, dict)?;
    let remote_conf = conf.project_remote();
    if !remote_conf.enabled {
        return Err(project_remote_disabled_err(
            work_root.join(ENGINE_CONF_PATH).display().to_string(),
        ));
    }
    let mode = resolve_project_remote_mode(remote_conf)?;
    match mode {
        ProjectRemoteMode::Single { repo, init_version } => sync_project_remote_with_repo_inner(
            work_root,
            &repo,
            requested_version,
            Some(init_version.as_str()),
            None,
        ),
        ProjectRemoteMode::Dual { .. } => Err(project_remote_dual_requires_group_err()),
    }
}

pub fn sync_project_remote_group_with_dict<P: AsRef<Path>>(
    work_root: P,
    group: RemoteGroup,
    requested_version: Option<&str>,
    dict: &EnvDict,
) -> RunResult<ProjectRemoteUpdateResult> {
    let work_root = work_root.as_ref();
    let conf = load_engine_config(work_root, dict)?;
    let remote_conf = conf.project_remote();
    if !remote_conf.enabled {
        return Err(project_remote_disabled_err(
            work_root.join(ENGINE_CONF_PATH).display().to_string(),
        ));
    }
    let mode = resolve_project_remote_mode(remote_conf)?;
    match mode {
        ProjectRemoteMode::Dual { models, infra } => {
            let group_conf = match group {
                RemoteGroup::Models => &models,
                RemoteGroup::Infra => &infra,
            };
            sync_project_remote_with_repo_inner(
                work_root,
                &group_conf.repo,
                requested_version,
                Some(group_conf.init_version.as_str()),
                Some(group),
            )
        }
        ProjectRemoteMode::Single { .. } => Err(project_remote_single_no_group_err()),
    }
}

pub fn sync_project_remote_from_repo<P: AsRef<Path>>(
    work_root: P,
    repo_url: &str,
    requested_version: Option<&str>,
) -> RunResult<ProjectRemoteUpdateResult> {
    let work_root = work_root.as_ref();
    if repo_url.trim().is_empty() {
        return Err(project_remote_repo_required_err());
    }
    sync_project_remote_with_repo_inner(work_root, repo_url, requested_version, None, None)
}

pub fn current_project_version<P: AsRef<Path>>(work_root: P) -> RunResult<Option<String>> {
    Ok(
        load_state(work_root.as_ref())?
            .and_then(|state| state.single_version().map(str::to_string)),
    )
}

pub fn current_project_group_versions<P: AsRef<Path>>(
    work_root: P,
) -> RunResult<Option<serde_json::Value>> {
    let state = load_state(work_root.as_ref())?;
    match state {
        Some(ProjectRemoteState::Dual { models, infra }) => {
            let mut map = serde_json::Map::new();
            if let Some(m) = models {
                map.insert(
                    "models".to_string(),
                    serde_json::json!({
                        "version": m.current_version,
                        "tag": m.resolved_tag,
                    }),
                );
            }
            if let Some(i) = infra {
                map.insert(
                    "infra".to_string(),
                    serde_json::json!({
                        "version": i.current_version,
                        "tag": i.resolved_tag,
                    }),
                );
            }
            Ok(Some(serde_json::Value::Object(map)))
        }
        _ => Ok(None),
    }
}

pub(crate) fn resolve_project_remote_mode(
    conf: &ProjectRemoteConf,
) -> RunResult<ProjectRemoteMode> {
    let has_single = !conf.repo.trim().is_empty();
    let has_models = conf.models.is_some();
    let has_infra = conf.infra.is_some();

    match (has_single, has_models, has_infra) {
        (true, false, false) => Ok(ProjectRemoteMode::Single {
            repo: conf.repo.clone(),
            init_version: conf.init_version.clone(),
        }),
        (false, true, true) => {
            let models = conf.models.as_ref().unwrap();
            let infra = conf.infra.as_ref().unwrap();
            if models.repo.trim().is_empty() {
                return Err(project_remote_repo_required_err_for("models"));
            }
            if infra.repo.trim().is_empty() {
                return Err(project_remote_repo_required_err_for("infra"));
            }
            Ok(ProjectRemoteMode::Dual {
                models: models.clone(),
                infra: infra.clone(),
            })
        }
        (false, true, false) => Err(project_remote_dual_partial_err("infra")),
        (false, false, true) => Err(project_remote_dual_partial_err("models")),
        _ => Err(project_remote_ambiguous_mode_err()),
    }
}

fn remote_cache_path_for(group: Option<RemoteGroup>) -> &'static str {
    match group {
        Some(RemoteGroup::Models) => REMOTE_CACHE_PATH_MODELS,
        Some(RemoteGroup::Infra) => REMOTE_CACHE_PATH_INFRA,
        None => REMOTE_CACHE_PATH,
    }
}

fn sync_project_remote_with_repo_inner(
    work_root: &Path,
    repo_url: &str,
    requested_version: Option<&str>,
    init_version: Option<&str>,
    group: Option<RemoteGroup>,
) -> RunResult<ProjectRemoteUpdateResult> {
    let dirs = managed_dirs_for(group);
    let group_label = group.map(|g| match g {
        RemoteGroup::Models => "models",
        RemoteGroup::Infra => "infra",
    });
    info_ctrl!(
        "project remote sync start work_root={} requested_version={} repo={} group={}",
        work_root.display(),
        requested_version.unwrap_or("(auto)"),
        repo_url,
        group_label.unwrap_or("-")
    );

    let remote_root = work_root.join(remote_cache_path_for(group));
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
            resolve_tag_for_version(&repo, &target_version)?
                .ok_or_else(|| requested_version_not_found_err(&target_version))?
        }
        _ => {
            let resolved =
                resolve_default_target(work_root, &repo, init_version.map(str::trim), group)?;
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

    let changed = managed_dirs_differ(&remote_root, work_root, dirs)?;
    let from_revision = previous_state.as_ref().and_then(|ps| match ps {
        ProjectRemoteState::Single { revision, .. } => Some(revision.as_str()),
        ProjectRemoteState::Dual { models, infra } => match group {
            Some(RemoteGroup::Models) => models.as_ref().map(|m| m.revision.as_str()),
            Some(RemoteGroup::Infra) => infra.as_ref().map(|i| i.revision.as_str()),
            None => None,
        },
    });
    info_ctrl!(
        "project remote sync diff work_root={} requested_version={} changed={} from_revision={} to_revision={}",
        work_root.display(),
        requested_version.unwrap_or("(auto)"),
        changed,
        from_revision.unwrap_or("-"),
        resolved.commit_id
    );
    if changed {
        info_ctrl!(
            "project remote sync backup managed dirs work_root={} dirs={}",
            work_root.display(),
            dirs.join(",")
        );
        backup_managed_dirs(work_root, dirs)?;
    }

    let result = ProjectRemoteUpdateResult {
        requested_version: requested_version.map(str::to_string),
        current_version: resolved.version,
        resolved_tag: resolved.tag,
        from_revision: from_revision.map(str::to_string),
        to_revision: oid_to_string(resolved.commit_id),
        changed,
        group: group_label.map(str::to_string),
    };
    let apply_result = (|| {
        if changed {
            info_ctrl!(
                "project remote sync apply managed dirs work_root={} remote_cache={}",
                work_root.display(),
                remote_root.display()
            );
            sync_managed_dirs(&remote_root, work_root, dirs)?;
        }
        match group {
            Some(g) => persist_group_state(work_root, g, &result)?,
            None => persist_state(work_root, &result)?,
        }
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
        rollback_partial_update(work_root, previous_state.as_ref(), changed, dirs).map_err(
            |rollback_err| {
                rollback_err.wrap_as(RunReason::from_conf(), format!("{}; rollback failed", err))
            },
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
    dirs: &[&str],
) -> RunResult<()> {
    if changed {
        restore_managed_dirs(work_root, dirs)?;
    }
    restore_project_remote_state(work_root, previous_state)
}

fn oid_to_string(oid: Oid) -> String {
    oid.to_string()
}

fn project_remote_disabled_err(path: impl Into<String>) -> wp_error::RunError {
    RunReason::from_conf()
        .to_err()
        .with_detail(format!("project_remote is disabled in {}", path.into()))
}

fn project_remote_repo_required_err() -> wp_error::RunError {
    RunReason::from_conf()
        .to_err()
        .with_detail("project_remote.repo must not be empty")
}

fn project_remote_repo_required_err_for(group: &str) -> wp_error::RunError {
    RunReason::from_conf()
        .to_err()
        .with_detail(format!("project_remote.{}.repo must not be empty", group))
}

fn project_remote_dual_partial_err(missing: &str) -> wp_error::RunError {
    RunReason::from_conf()
        .to_err()
        .with_detail(format!(
            "dual-repo mode requires both [project_remote.models] and [project_remote.infra]; missing '{}'",
            missing
        ))
}

fn project_remote_ambiguous_mode_err() -> wp_error::RunError {
    RunReason::from_conf()
        .to_err()
        .with_detail(
            "ambiguous project_remote config: use either 'repo' (single-repo) or both 'models' + 'infra' (dual-repo), not a mix",
        )
}

fn project_remote_dual_requires_group_err() -> wp_error::RunError {
    RunReason::from_conf().to_err().with_detail(
        "dual-repo mode requires --group (models|infra); use sync_project_remote_group_with_dict",
    )
}

fn project_remote_single_no_group_err() -> wp_error::RunError {
    RunReason::from_conf()
        .to_err()
        .with_detail("single-repo mode does not support --group; use sync_project_remote_with_dict")
}

fn requested_version_not_found_err(version: &str) -> wp_error::RunError {
    RunReason::from_conf()
        .to_err()
        .with_detail(format!("requested version '{}' was not found", version))
}

fn conf_err_source<E>(message: impl Into<String>, source: E) -> wp_error::RunError
where
    E: std::error::Error + Send + Sync + 'static,
{
    RunReason::from_conf()
        .to_err()
        .with_detail(message.into())
        .with_std_source(source)
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests {
    use super::test_support::{
        create_dual_work_root, create_empty_managed_dirs, create_infra_remote_fixture,
        create_models_remote_fixture, create_remote_fixture, create_remote_fixture_without_tags,
        create_work_root, write_engine_conf_with_init_version, write_model_version,
        write_runtime_local_dirs,
    };
    use super::*;
    use std::fs;
    use tempfile::tempdir;

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
                group: None,
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
        // Write a Dual state file. When sync_project_remote (single-repo) tries
        // to persist, persist_state will refuse to downgrade Dual → Single,
        // triggering the rollback path.
        fs::write(
            work_root.path().join(STATE_PATH),
            r#"{"models":{"version":"1.4.2","tag":"v1.4.2","revision":"old-revision"},"infra":{"version":"1.0.0","tag":"v1.0.0","revision":"infra-rev"}}"#,
        )
        .expect("write dual state");

        let err =
            sync_project_remote(work_root.path(), Some("1.4.3")).expect_err("sync should fail");
        assert!(
            err.to_string()
                .contains("cannot persist single-repo state over dual-repo state"),
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
        assert_eq!(state["models"]["version"], "1.4.2");
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

    // ============ Dual-Repo Tests ============

    #[test]
    fn dual_sync_models_only_updates_models_dir() {
        let models_remote = create_models_remote_fixture();
        let infra_remote = create_infra_remote_fixture();
        let work_root = create_dual_work_root(&models_remote, &infra_remote);
        create_empty_managed_dirs(work_root.path());

        let result = sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Models,
            Some("1.4.3"),
            &EnvDict::default(),
        )
        .expect("sync models");

        assert_eq!(result.current_version, "1.4.3");
        assert_eq!(result.resolved_tag, "v1.4.3");
        assert_eq!(result.group.as_deref(), Some("models"));
        assert!(result.changed);
        assert_eq!(
            fs::read_to_string(work_root.path().join("models/version.txt")).expect("read version"),
            "1.4.3\n"
        );
        // infra dirs should be untouched
        assert!(!work_root.path().join("conf/infra.toml").exists());
    }

    #[test]
    fn dual_sync_infra_only_updates_infra_dirs() {
        let models_remote = create_models_remote_fixture();
        let infra_remote = create_infra_remote_fixture();
        let work_root = create_dual_work_root(&models_remote, &infra_remote);
        create_empty_managed_dirs(work_root.path());

        let result = sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Infra,
            Some("1.1.0"),
            &EnvDict::default(),
        )
        .expect("sync infra");

        assert_eq!(result.current_version, "1.1.0");
        assert_eq!(result.resolved_tag, "v1.1.0");
        assert_eq!(result.group.as_deref(), Some("infra"));
        assert!(result.changed);
        assert_eq!(
            fs::read_to_string(work_root.path().join("conf/infra.toml")).expect("read infra"),
            "[infra]\nversion = \"1.1.0\"\n"
        );
        // models dir should be untouched (still empty from create_empty_managed_dirs)
        assert!(!work_root.path().join("models/version.txt").exists());
    }

    #[test]
    fn dual_sync_uses_init_version_when_no_state_and_no_requested_version() {
        let models_remote = create_models_remote_fixture();
        let infra_remote = create_infra_remote_fixture();
        let work_root = create_dual_work_root(&models_remote, &infra_remote);
        create_empty_managed_dirs(work_root.path());

        let result = sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Models,
            None,
            &EnvDict::default(),
        )
        .expect("sync models with init_version");

        assert_eq!(result.current_version, "1.4.2");
        assert_eq!(result.resolved_tag, "v1.4.2");
    }

    #[test]
    fn dual_sync_rollback_preserves_other_group_state() {
        let models_remote = create_models_remote_fixture();
        let infra_remote = create_infra_remote_fixture();
        let work_root = create_dual_work_root(&models_remote, &infra_remote);
        create_empty_managed_dirs(work_root.path());

        // Sync models to an older version first, creating initial dual state entry
        sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Models,
            Some("1.4.2"),
            &EnvDict::default(),
        )
        .expect("sync models v1.4.2");

        // Manually inject infra state to simulate a previously-synced infra
        persist_group_state(
            work_root.path(),
            RemoteGroup::Infra,
            &ProjectRemoteUpdateResult {
                requested_version: Some("1.0.0".to_string()),
                current_version: "1.0.0".to_string(),
                resolved_tag: "v1.0.0".to_string(),
                from_revision: None,
                to_revision: "infra-rev".to_string(),
                changed: false,
                group: Some("infra".to_string()),
            },
        )
        .expect("inject infra state");

        // Sync models to newer version
        sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Models,
            Some("1.4.3"),
            &EnvDict::default(),
        )
        .expect("sync models v1.4.3");

        // Verify both groups are present and independent
        let state = load_state(work_root.path())
            .expect("load state")
            .expect("state exists");
        match state {
            ProjectRemoteState::Dual { models, infra } => {
                let models = models.expect("models synced");
                let infra = infra.expect("infra synced");
                assert_eq!(models.current_version, "1.4.3");
                assert_eq!(infra.current_version, "1.0.0");
            }
            _ => panic!("expected Dual state"),
        }
    }

    #[test]
    fn dual_sync_persists_group_versions_independently() {
        let models_remote = create_models_remote_fixture();
        let infra_remote = create_infra_remote_fixture();
        let work_root = create_dual_work_root(&models_remote, &infra_remote);
        create_empty_managed_dirs(work_root.path());

        sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Models,
            Some("1.4.2"),
            &EnvDict::default(),
        )
        .expect("sync models");
        sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Infra,
            Some("1.0.0"),
            &EnvDict::default(),
        )
        .expect("sync infra");

        let state_json: serde_json::Value = serde_json::from_slice(
            &fs::read(work_root.path().join(STATE_PATH)).expect("read state"),
        )
        .expect("parse state");
        assert_eq!(state_json["models"]["version"], "1.4.2");
        assert_eq!(state_json["models"]["tag"], "v1.4.2");
        assert_eq!(state_json["infra"]["version"], "1.0.0");
        assert_eq!(state_json["infra"]["tag"], "v1.0.0");
    }

    #[test]
    fn dual_sync_single_repo_with_group_errors() {
        let fixture = create_remote_fixture();
        let work_root = create_work_root(&fixture);

        let err = sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Models,
            None,
            &EnvDict::default(),
        )
        .expect_err("should reject group on single repo");

        assert!(
            err.to_string().contains("single-repo"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn dual_sync_dual_repo_without_group_errors() {
        let models_remote = create_models_remote_fixture();
        let infra_remote = create_infra_remote_fixture();
        let work_root = create_dual_work_root(&models_remote, &infra_remote);

        let err = sync_project_remote_with_dict(work_root.path(), None, &EnvDict::default())
            .expect_err("should require group on dual repo");

        assert!(
            err.to_string().contains("--group"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn state_backward_compat_reads_old_single_format() {
        let work_root = tempfile::tempdir().expect("tempdir");
        let run_dir = work_root.path().join(".run");
        fs::create_dir_all(&run_dir).expect("create .run");
        fs::write(
            work_root.path().join(STATE_PATH),
            r#"{"current_version":"1.4.2","resolved_tag":"v1.4.2","revision":"abc123"}"#,
        )
        .expect("write old state");

        let state = load_state(work_root.path())
            .expect("load state")
            .expect("state exists");
        match state {
            ProjectRemoteState::Single {
                current_version,
                resolved_tag,
                revision,
            } => {
                assert_eq!(current_version, "1.4.2");
                assert_eq!(resolved_tag, "v1.4.2");
                assert_eq!(revision, "abc123");
            }
            _ => panic!("expected Single state, got Dual"),
        }

        let version = current_project_version(work_root.path()).expect("read version");
        assert_eq!(version, Some("1.4.2".to_string()));
    }

    #[test]
    fn state_dual_format_roundtrip() {
        let models_remote = create_models_remote_fixture();
        let infra_remote = create_infra_remote_fixture();
        let work_root = create_dual_work_root(&models_remote, &infra_remote);
        create_empty_managed_dirs(work_root.path());

        sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Models,
            Some("1.4.3"),
            &EnvDict::default(),
        )
        .expect("sync models");
        sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Infra,
            Some("1.1.0"),
            &EnvDict::default(),
        )
        .expect("sync infra");

        // Read back state
        let state = load_state(work_root.path())
            .expect("load state")
            .expect("state exists");
        match state {
            ProjectRemoteState::Dual { models, infra } => {
                let models = models.expect("models synced");
                let infra = infra.expect("infra synced");
                assert_eq!(models.current_version, "1.4.3");
                assert_eq!(models.resolved_tag, "v1.4.3");
                assert!(!models.revision.is_empty());
                assert_eq!(infra.current_version, "1.1.0");
                assert_eq!(infra.resolved_tag, "v1.1.0");
                assert!(!infra.revision.is_empty());
            }
            _ => panic!("expected Dual state"),
        }
    }

    #[test]
    fn dual_sync_preserves_runtime_local_dirs() {
        let models_remote = create_models_remote_fixture();
        let infra_remote = create_infra_remote_fixture();
        let work_root = create_dual_work_root(&models_remote, &infra_remote);
        create_empty_managed_dirs(work_root.path());
        write_runtime_local_dirs(work_root.path());

        sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Models,
            Some("1.4.3"),
            &EnvDict::default(),
        )
        .expect("sync models");

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
    fn dual_snapshot_rollback_restores_only_affected_group() {
        let models_remote = create_models_remote_fixture();
        let infra_remote = create_infra_remote_fixture();
        let work_root = create_dual_work_root(&models_remote, &infra_remote);
        create_empty_managed_dirs(work_root.path());
        // Create a local marker file in models/ to verify rollback restores it
        fs::write(work_root.path().join("models/local.txt"), "local-data\n")
            .expect("write local models file");

        // Capture snapshot before any sync
        let snapshot =
            capture_project_remote_snapshot_with_group(work_root.path(), Some(RemoteGroup::Models))
                .expect("capture snapshot");

        // Sync models to latest
        sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Models,
            Some("1.4.3"),
            &EnvDict::default(),
        )
        .expect("sync models v1.4.3");
        assert_eq!(
            fs::read_to_string(work_root.path().join("models/version.txt")).expect("read version"),
            "1.4.3\n"
        );
        // local file should be gone after sync
        assert!(!work_root.path().join("models/local.txt").exists());

        // Rollback models
        restore_project_remote_update(work_root.path(), &snapshot, true).expect("rollback models");

        // models should be rolled back to pre-sync state (local.txt restored, version.txt gone)
        assert!(work_root.path().join("models/local.txt").exists());
        assert_eq!(
            fs::read_to_string(work_root.path().join("models/local.txt")).expect("read local"),
            "local-data\n"
        );
        assert!(!work_root.path().join("models/version.txt").exists());

        // conf/wparse.toml should still exist (not affected by models rollback)
        assert!(work_root.path().join("conf/wparse.toml").exists());
    }

    #[test]
    fn dual_sync_initializes_cache_for_each_group_separately() {
        let models_remote = create_models_remote_fixture();
        let infra_remote = create_infra_remote_fixture();
        let work_root = create_dual_work_root(&models_remote, &infra_remote);
        create_empty_managed_dirs(work_root.path());

        sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Models,
            Some("1.4.2"),
            &EnvDict::default(),
        )
        .expect("sync models");
        sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Infra,
            Some("1.0.0"),
            &EnvDict::default(),
        )
        .expect("sync infra");

        assert!(work_root
            .path()
            .join(REMOTE_CACHE_PATH_MODELS)
            .join(".git")
            .exists());
        assert!(work_root
            .path()
            .join(REMOTE_CACHE_PATH_INFRA)
            .join(".git")
            .exists());
        // old single cache path should not exist
        assert!(!work_root.path().join(REMOTE_CACHE_PATH).exists());
    }

    #[test]
    fn dual_sync_second_group_uses_init_version_when_first_group_already_synced() {
        let models_remote = create_models_remote_fixture();
        let infra_remote = create_infra_remote_fixture();
        let work_root = create_dual_work_root(&models_remote, &infra_remote);
        create_empty_managed_dirs(work_root.path());

        // Sync models first — creates Dual state with models=Some, infra=None
        sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Models,
            None,
            &EnvDict::default(),
        )
        .expect("sync models first");

        // Sync infra without --version. Must use its own init_version (1.0.0),
        // not the latest infra tag (1.1.0), because infra hasn't been synced yet.
        let result = sync_project_remote_group_with_dict(
            work_root.path(),
            RemoteGroup::Infra,
            None,
            &EnvDict::default(),
        )
        .expect("sync infra second");

        assert_eq!(result.current_version, "1.0.0");
        assert_eq!(result.resolved_tag, "v1.0.0");
        assert_eq!(result.group.as_deref(), Some("infra"));
    }

    #[test]
    fn restore_managed_dirs_cleans_up_dirs_created_during_failed_update() {
        let work_root = tempdir().expect("tempdir");
        let dirs: &[&str] = &["models", "conf"];
        let backup_root = work_root.path().join(BACKUP_PATH);
        let manifest_path = work_root.path().join(BACKUP_MANIFEST_PATH);

        // Simulate pre-update state: only models/ exists
        fs::create_dir_all(work_root.path().join("models")).expect("create models");
        fs::write(work_root.path().join("models/version.txt"), "1.4.2\n").expect("write version");

        // Create backup of the pre-update state (only models/)
        fs::create_dir_all(&backup_root).expect("create backup root");
        fs::create_dir_all(backup_root.join("models")).expect("create backup models");
        fs::write(backup_root.join("models/version.txt"), "1.4.2\n").expect("write backup version");
        let manifest = BackupManifest {
            existing_dirs: vec!["models".to_string()],
        };
        let body = serde_json::to_vec_pretty(&manifest).expect("encode manifest");
        fs::write(&manifest_path, body).expect("write manifest");

        // Simulate a failed update that created conf/ (not in backup manifest)
        fs::create_dir_all(work_root.path().join("conf")).expect("create conf during update");
        fs::write(work_root.path().join("conf/new.toml"), "[new]\n").expect("write new conf");

        // Restore: should remove both models/ and conf/, then restore models/ from backup
        restore_managed_dirs(work_root.path(), dirs).expect("restore");

        // conf/ (created during failed update, not in backup) must be cleaned up
        assert!(
            !work_root.path().join("conf").exists(),
            "conf/ should be removed (not in backup manifest)"
        );
        // models/ (in backup) must be restored
        assert_eq!(
            fs::read_to_string(work_root.path().join("models/version.txt")).expect("read version"),
            "1.4.2\n"
        );
    }

    #[test]
    fn restore_managed_dirs_no_manifest_is_noop() {
        let work_root = tempdir().expect("tempdir");
        let dirs: &[&str] = &["models"];

        fs::create_dir_all(work_root.path().join("models")).expect("create models");
        fs::write(work_root.path().join("models/version.txt"), "data\n").expect("write data");

        // No backup manifest at all — should be a no-op
        restore_managed_dirs(work_root.path(), dirs).expect("restore without manifest");

        // models/ should be untouched
        assert_eq!(
            fs::read_to_string(work_root.path().join("models/version.txt")).expect("read version"),
            "data\n"
        );
    }
}
