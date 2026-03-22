use std::fs;
use std::path::Path;

use git2::{build::CheckoutBuilder, Oid, Remote, Repository, StatusOptions};
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

    let repo = Repository::open(work_root).map_err(|e| {
        conf_err(format!(
            "open git repository {} failed: {}",
            work_root.display(),
            e
        ))
    })?;
    ensure_clean_worktree(&repo)?;

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
    let mut remote = ensure_remote(repo, repo_url)?;
    remote
        .fetch(&["refs/tags/*:refs/tags/*"], None, None)
        .map_err(|e| conf_err(format!("fetch remote tags failed: {}", e)))?;
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

fn conf_err(message: impl Into<String>) -> wp_error::RunError {
    RunReason::from_conf().to_err().with_detail(message.into())
}
