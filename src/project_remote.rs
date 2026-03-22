use std::fs;
use std::path::Path;

use git2::{
    build::CheckoutBuilder, ErrorCode, FetchOptions, Oid, Remote, Repository, StatusOptions,
};
use orion_conf::{ToStructError, UvsConfFrom};
use orion_variate::EnvDict;
use semver::Version;
use serde::Serialize;
use wp_config::engine::EngineConfig;
use wp_error::run_error::{RunReason, RunResult};

const ENGINE_CONF_PATH: &str = "conf/wparse.toml";
const STATE_PATH: &str = ".run/project_remote_state.json";
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
    revision: Option<String>,
    state_file: Option<Vec<u8>>,
}

#[derive(Debug, Serialize)]
struct ProjectRemoteState {
    current_version: String,
    resolved_tag: String,
    revision: String,
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

    let repo = open_or_init_repo(work_root)?;
    if has_checked_out_head(&repo) {
        ensure_clean_worktree(&repo)?;
    }

    fetch_remote_tags(&repo, &remote_conf.repo)?;

    let target_version = match requested_version {
        Some(version) if !version.trim().is_empty() => version.trim().to_string(),
        _ => resolve_default_version(&repo, remote_conf.init_version.as_str())?,
    };

    let resolved = resolve_tag_for_version(&repo, &target_version)?.ok_or_else(|| {
        conf_err(format!(
            "requested version '{}' was not found",
            target_version
        ))
    })?;

    let from_revision = repo
        .head()
        .ok()
        .and_then(|head| head.target())
        .map(oid_to_string);
    let resolved_revision = oid_to_string(resolved.commit_id);
    let changed = from_revision.as_deref() != Some(resolved_revision.as_str());

    let commit = repo.find_commit(resolved.commit_id).map_err(|e| {
        conf_err(format!(
            "load target commit {} failed: {}",
            resolved.commit_id, e
        ))
    })?;
    repo.checkout_tree(commit.as_object(), Some(CheckoutBuilder::new().force()))
        .map_err(|e| conf_err(format!("checkout tag {} failed: {}", resolved.tag, e)))?;
    repo.set_head_detached(resolved.commit_id)
        .map_err(|e| conf_err(format!("set detached HEAD failed: {}", e)))?;

    let result = ProjectRemoteUpdateResult {
        requested_version: requested_version.map(str::to_string),
        current_version: resolved.version,
        resolved_tag: resolved.tag,
        from_revision,
        to_revision: resolved_revision,
        changed,
    };
    persist_state(work_root, &result)?;
    Ok(result)
}

pub fn capture_project_remote_snapshot<P: AsRef<Path>>(
    work_root: P,
) -> RunResult<ProjectRemoteSnapshot> {
    let work_root = work_root.as_ref();
    let revision = match Repository::open(work_root) {
        Ok(repo) => repo
            .head()
            .ok()
            .and_then(|head| head.target())
            .map(oid_to_string),
        Err(_) => None,
    };
    let state_path = work_root.join(STATE_PATH);
    let state_file = match fs::read(&state_path) {
        Ok(bytes) => Some(bytes),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => {
            return Err(conf_err(format!(
                "read {} failed: {}",
                state_path.display(),
                err
            )))
        }
    };
    Ok(ProjectRemoteSnapshot {
        revision,
        state_file,
    })
}

pub fn restore_project_remote_snapshot<P: AsRef<Path>>(
    work_root: P,
    snapshot: &ProjectRemoteSnapshot,
) -> RunResult<()> {
    let work_root = work_root.as_ref();
    if let Some(revision) = snapshot.revision.as_deref() {
        let repo = Repository::open(work_root).map_err(|e| {
            conf_err(format!(
                "open git repository {} failed: {}",
                work_root.display(),
                e
            ))
        })?;
        checkout_revision(&repo, revision)?;
    }

    let state_path = work_root.join(STATE_PATH);
    match &snapshot.state_file {
        Some(bytes) => {
            if let Some(parent) = state_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| conf_err(format!("create {} failed: {}", parent.display(), e)))?;
            }
            fs::write(&state_path, bytes)
                .map_err(|e| conf_err(format!("write {} failed: {}", state_path.display(), e)))?;
        }
        None => {
            if let Err(err) = fs::remove_file(&state_path) {
                if err.kind() != std::io::ErrorKind::NotFound {
                    return Err(conf_err(format!(
                        "remove {} failed: {}",
                        state_path.display(),
                        err
                    )));
                }
            }
        }
    }
    Ok(())
}

fn load_engine_config(work_root: &Path) -> RunResult<EngineConfig> {
    let dict = crate::load_sec_dict().unwrap_or_else(|_| EnvDict::new());
    EngineConfig::load(work_root, &dict).map_err(|e| {
        conf_err(format!(
            "load {} failed: {}",
            work_root.join(ENGINE_CONF_PATH).display(),
            e
        ))
    })
}

fn open_or_init_repo(work_root: &Path) -> RunResult<Repository> {
    match Repository::open(work_root) {
        Ok(repo) => Ok(repo),
        Err(err) if err.code() == ErrorCode::NotFound => Repository::init(work_root).map_err(|e| {
            conf_err(format!(
                "init git repository {} failed: {}",
                work_root.display(),
                e
            ))
        }),
        Err(err) => Err(conf_err(format!(
            "open git repository {} failed: {}",
            work_root.display(),
            err
        ))),
    }
}

fn has_checked_out_head(repo: &Repository) -> bool {
    repo.head().ok().and_then(|head| head.target()).is_some()
}

fn ensure_clean_worktree(repo: &Repository) -> RunResult<()> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);
    let statuses = repo
        .statuses(Some(&mut opts))
        .map_err(|e| conf_err(format!("read worktree status failed: {}", e)))?;
    if statuses.iter().any(|entry| {
        let Some(path) = entry.path() else {
            return entry.status() != git2::Status::CURRENT;
        };
        if path.starts_with(".run/")
            || path.starts_with("runtime/")
            || path.starts_with("data/")
            || path.starts_with("logs/")
        {
            return false;
        }
        entry.status() != git2::Status::CURRENT
    }) {
        return Err(conf_err(
            "project_remote update requires a clean worktree; commit or discard local changes first",
        ));
    }
    Ok(())
}

fn fetch_remote_tags(repo: &Repository, repo_url: &str) -> RunResult<()> {
    clear_local_release_tags(repo)?;
    let mut remote = ensure_remote(repo, repo_url)?;
    let mut fetch_options = FetchOptions::new();
    fetch_options.prune(git2::FetchPrune::On);
    remote
        .fetch(
            &["+refs/tags/*:refs/tags/*"],
            Some(&mut fetch_options),
            None,
        )
        .map_err(|e| conf_err(format!("fetch remote tags failed: {}", e)))?;
    Ok(())
}

fn clear_local_release_tags(repo: &Repository) -> RunResult<()> {
    let mut refs = repo
        .references_glob("refs/tags/*")
        .map_err(|e| conf_err(format!("list local tags failed: {}", e)))?;
    while let Some(reference) = refs.next() {
        let mut reference =
            reference.map_err(|e| conf_err(format!("read local tag failed: {}", e)))?;
        let Some(name) = reference.name() else {
            continue;
        };
        let Some(tag) = name.strip_prefix("refs/tags/") else {
            continue;
        };
        if parse_tag_version(tag).is_none() {
            continue;
        }
        reference
            .delete()
            .map_err(|e| conf_err(format!("delete local tag failed: {}", e)))?;
    }
    Ok(())
}

fn ensure_remote<'a>(repo: &'a Repository, repo_url: &str) -> RunResult<Remote<'a>> {
    match repo.find_remote("origin") {
        Ok(remote) => {
            if remote.url() != Some(repo_url) {
                repo.remote_set_url("origin", repo_url)
                    .map_err(|e| conf_err(format!("set origin URL failed: {}", e)))?;
            }
            repo.find_remote("origin")
                .map_err(|e| conf_err(format!("find origin remote failed: {}", e)))
        }
        Err(_) => repo
            .remote("origin", repo_url)
            .map_err(|e| conf_err(format!("create origin remote failed: {}", e))),
    }
}

fn resolve_default_version(repo: &Repository, init_version: &str) -> RunResult<String> {
    if repo.head().is_err() {
        if init_version.trim().is_empty() {
            return Err(conf_err(
                "project_remote.init_version must be set for first-time initialization",
            ));
        }
        return Ok(init_version.trim().to_string());
    }
    latest_released_version(repo)
}

fn latest_released_version(repo: &Repository) -> RunResult<String> {
    let names = repo
        .tag_names(None)
        .map_err(|e| conf_err(format!("list tags failed: {}", e)))?;
    let latest = names
        .iter()
        .flatten()
        .filter_map(parse_tag_version)
        .max_by(|a, b| a.1.cmp(&b.1))
        .map(|(version, _)| version)
        .ok_or_else(|| conf_err("no released version tag was found"))?;
    Ok(latest)
}

struct ResolvedTag {
    tag: String,
    version: String,
    commit_id: Oid,
}

fn resolve_tag_for_version(repo: &Repository, version: &str) -> RunResult<Option<ResolvedTag>> {
    let names = repo
        .tag_names(None)
        .map_err(|e| conf_err(format!("list tags failed: {}", e)))?;
    for name in names.iter().flatten() {
        let Some((normalized, _)) = parse_tag_version(name) else {
            continue;
        };
        if normalized != version {
            continue;
        }
        let obj = repo
            .revparse_single(&format!("refs/tags/{}", name))
            .map_err(|e| conf_err(format!("resolve tag {} failed: {}", name, e)))?;
        let commit = obj
            .peel_to_commit()
            .map_err(|e| conf_err(format!("peel tag {} to commit failed: {}", name, e)))?;
        return Ok(Some(ResolvedTag {
            tag: name.to_string(),
            version: normalized,
            commit_id: commit.id(),
        }));
    }
    Ok(None)
}

fn parse_tag_version(tag: &str) -> Option<(String, Version)> {
    let trimmed = tag.strip_prefix('v').unwrap_or(tag);
    Version::parse(trimmed)
        .ok()
        .map(|version| (trimmed.to_string(), version))
}

fn persist_state(work_root: &Path, result: &ProjectRemoteUpdateResult) -> RunResult<()> {
    let state = ProjectRemoteState {
        current_version: result.current_version.clone(),
        resolved_tag: result.resolved_tag.clone(),
        revision: result.to_revision.clone(),
    };
    let path = work_root.join(STATE_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| conf_err(format!("create {} failed: {}", parent.display(), e)))?;
    }
    let body = serde_json::to_vec_pretty(&state)
        .map_err(|e| conf_err(format!("encode project remote state failed: {}", e)))?;
    fs::write(&path, body)
        .map_err(|e| conf_err(format!("write {} failed: {}", path.display(), e)))?;
    Ok(())
}

fn oid_to_string(oid: Oid) -> String {
    oid.to_string()
}

fn checkout_revision(repo: &Repository, revision: &str) -> RunResult<()> {
    let oid = Oid::from_str(revision)
        .map_err(|e| conf_err(format!("parse revision {} failed: {}", revision, e)))?;
    let commit = repo
        .find_commit(oid)
        .map_err(|e| conf_err(format!("load target commit {} failed: {}", revision, e)))?;
    repo.checkout_tree(commit.as_object(), Some(CheckoutBuilder::new().force()))
        .map_err(|e| conf_err(format!("checkout revision {} failed: {}", revision, e)))?;
    repo.set_head_detached(commit.id())
        .map_err(|e| conf_err(format!("set detached HEAD failed: {}", e)))?;
    Ok(())
}

fn conf_err(message: impl Into<String>) -> wp_error::RunError {
    RunReason::from_conf().to_err().with_detail(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    struct RemoteFixture {
        _temp: TempDir,
        remote_path: PathBuf,
    }

    fn write_engine_conf(work_root: &Path, repo_url: &str) {
        let conf_dir = work_root.join("conf");
        fs::create_dir_all(&conf_dir).expect("create conf dir");
        fs::write(
            conf_dir.join("wparse.toml"),
            format!(
                r#"version = "1.0"

[project_remote]
enabled = true
repo = "{repo_url}"
init_version = "1.4.2"
"#
            ),
        )
        .expect("write wparse.toml");
    }

    fn commit_all(repo: &Repository, message: &str) -> Oid {
        let mut index = repo.index().expect("open index");
        index
            .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
            .expect("add all");
        index.write().expect("write index");
        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        let sig = Signature::now("warp-parse-test", "warp-parse@test.local").expect("signature");
        let parent = repo
            .head()
            .ok()
            .and_then(|head| head.target())
            .and_then(|oid| repo.find_commit(oid).ok());
        match parent.as_ref() {
            Some(parent) => repo
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &[parent])
                .expect("commit with parent"),
            None => repo
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                .expect("initial commit"),
        }
    }

    fn tag_head(repo: &Repository, tag: &str) {
        let obj = repo
            .head()
            .expect("head")
            .peel(git2::ObjectType::Commit)
            .expect("peel head");
        repo.tag_lightweight(tag, &obj, false)
            .expect("create lightweight tag");
    }

    fn create_remote_fixture() -> RemoteFixture {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = Repository::init(temp.path()).expect("init remote repo");
        write_engine_conf(temp.path(), temp.path().to_str().expect("repo path utf8"));
        fs::create_dir_all(temp.path().join("rules")).expect("create rules dir");
        fs::write(temp.path().join("rules/version.txt"), "1.4.2\n").expect("write v1.4.2");
        commit_all(&repo, "release 1.4.2");
        tag_head(&repo, "v1.4.2");

        fs::write(temp.path().join("rules/version.txt"), "1.4.3\n").expect("write v1.4.3");
        commit_all(&repo, "release 1.4.3");
        tag_head(&repo, "v1.4.3");

        RemoteFixture {
            remote_path: temp.path().to_path_buf(),
            _temp: temp,
        }
    }

    fn clone_remote(remote_path: &Path) -> TempDir {
        let temp = tempfile::tempdir().expect("tempdir");
        Repository::clone(remote_path.to_str().expect("remote path utf8"), temp.path())
            .expect("clone remote");
        temp
    }

    fn checkout_tag(work_root: &Path, tag: &str) {
        let repo = Repository::open(work_root).expect("open clone repo");
        let obj = repo
            .revparse_single(&format!("refs/tags/{tag}"))
            .expect("find tag");
        let commit = obj.peel_to_commit().expect("tag commit");
        repo.checkout_tree(commit.as_object(), Some(CheckoutBuilder::new().force()))
            .expect("checkout tag");
        repo.set_head_detached(commit.id()).expect("detach head");
    }

    #[test]
    fn sync_project_remote_updates_to_requested_version_and_persists_state() {
        let fixture = create_remote_fixture();
        let clone = clone_remote(&fixture.remote_path);
        checkout_tag(clone.path(), "v1.4.2");

        let result = sync_project_remote(clone.path(), Some("1.4.3")).expect("sync remote");

        assert_eq!(result.requested_version.as_deref(), Some("1.4.3"));
        assert_eq!(result.current_version, "1.4.3");
        assert_eq!(result.resolved_tag, "v1.4.3");
        assert!(result.changed);
        assert_eq!(
            fs::read_to_string(clone.path().join("rules/version.txt")).expect("read version file"),
            "1.4.3\n"
        );

        let state: serde_json::Value = serde_json::from_slice(
            &fs::read(clone.path().join(STATE_PATH)).expect("read state file"),
        )
        .expect("parse state json");
        assert_eq!(state["current_version"], "1.4.3");
        assert_eq!(state["resolved_tag"], "v1.4.3");
        assert_eq!(state["revision"], result.to_revision);
    }

    #[test]
    fn sync_project_remote_uses_latest_release_when_version_is_not_provided() {
        let fixture = create_remote_fixture();
        let clone = clone_remote(&fixture.remote_path);
        checkout_tag(clone.path(), "v1.4.2");

        let result = sync_project_remote(clone.path(), None).expect("sync remote");

        assert_eq!(result.requested_version, None);
        assert_eq!(result.current_version, "1.4.3");
        assert_eq!(result.resolved_tag, "v1.4.3");
        assert_eq!(
            fs::read_to_string(clone.path().join("rules/version.txt")).expect("read version file"),
            "1.4.3\n"
        );
    }

    #[test]
    fn sync_project_remote_allows_runtime_generated_paths_but_rejects_other_dirty_changes() {
        let fixture = create_remote_fixture();

        let allowed_clone = clone_remote(&fixture.remote_path);
        checkout_tag(allowed_clone.path(), "v1.4.2");
        fs::create_dir_all(allowed_clone.path().join("runtime")).expect("create runtime dir");
        fs::write(
            allowed_clone.path().join("runtime/admin_api.token"),
            "test-token\n",
        )
        .expect("write runtime token");
        let allowed = sync_project_remote(allowed_clone.path(), Some("1.4.3"));
        assert!(allowed.is_ok(), "runtime files should not block update");

        let dirty_clone = clone_remote(&fixture.remote_path);
        checkout_tag(dirty_clone.path(), "v1.4.2");
        fs::write(dirty_clone.path().join("notes.txt"), "dirty\n").expect("write dirty file");
        let err = sync_project_remote(dirty_clone.path(), Some("1.4.3"))
            .expect_err("non-runtime dirty worktree should fail");
        assert!(
            err.to_string().contains("clean worktree"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn sync_project_remote_initializes_non_git_work_root() {
        let fixture = create_remote_fixture();
        let work_root = tempfile::tempdir().expect("tempdir");
        write_engine_conf(
            work_root.path(),
            fixture.remote_path.to_str().expect("repo path utf8"),
        );

        let result = sync_project_remote(work_root.path(), Some("1.4.2"))
            .expect("sync should initialize git repo");

        assert_eq!(result.current_version, "1.4.2");
        assert_eq!(result.resolved_tag, "v1.4.2");
        assert!(
            work_root.path().join(".git").exists(),
            "git repo should be initialized"
        );
        assert_eq!(
            fs::read_to_string(work_root.path().join("rules/version.txt")).expect("read version"),
            "1.4.2\n"
        );
    }
}
