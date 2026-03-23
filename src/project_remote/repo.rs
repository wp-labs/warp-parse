use std::fs;
use std::path::Path;

use git2::{build::CheckoutBuilder, ErrorCode, FetchOptions, Oid, Remote, Repository};
use semver::Version;
use wp_error::run_error::RunResult;

use super::managed::remove_path;
use super::{conf_err, ResolvedTag, STATE_PATH};

pub(super) fn prepare_remote_repo(remote_root: &Path, repo_url: &str) -> RunResult<Repository> {
    if !remote_root.exists() {
        if let Some(parent) = remote_root.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| conf_err(format!("create {} failed: {}", parent.display(), e)))?;
        }
        return Repository::clone(repo_url, remote_root).map_err(|e| {
            conf_err(format!(
                "clone remote repository {} into {} failed: {}",
                repo_url,
                remote_root.display(),
                e
            ))
        });
    }

    match Repository::open(remote_root) {
        Ok(repo) => {
            ensure_remote(&repo, repo_url)?;
            Ok(repo)
        }
        Err(err) if err.code() == ErrorCode::NotFound => {
            remove_path(remote_root)?;
            Repository::clone(repo_url, remote_root).map_err(|e| {
                conf_err(format!(
                    "clone remote repository {} into {} failed: {}",
                    repo_url,
                    remote_root.display(),
                    e
                ))
            })
        }
        Err(err) => Err(conf_err(format!(
            "open remote cache repository {} failed: {}",
            remote_root.display(),
            err
        ))),
    }
}

pub(super) fn fetch_remote_tags(repo: &Repository, repo_url: &str) -> RunResult<()> {
    clear_local_release_tags(repo)?;
    let mut remote = ensure_remote(repo, repo_url)?;
    let mut fetch_options = FetchOptions::new();
    fetch_options.prune(git2::FetchPrune::On);
    remote
        .fetch(
            &[
                "+HEAD:refs/remotes/origin/HEAD",
                "+refs/heads/*:refs/remotes/origin/*",
                "+refs/tags/*:refs/tags/*",
            ],
            Some(&mut fetch_options),
            None,
        )
        .map_err(|e| conf_err(format!("fetch remote tags failed: {}", e)))?;
    Ok(())
}

fn clear_local_release_tags(repo: &Repository) -> RunResult<()> {
    let refs = repo
        .references_glob("refs/tags/*")
        .map_err(|e| conf_err(format!("list local tags failed: {}", e)))?;
    for reference in refs {
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

pub(super) fn resolve_default_target(
    work_root: &Path,
    repo: &Repository,
    init_version: Option<&str>,
) -> RunResult<ResolvedTag> {
    if is_first_initialization(work_root)? {
        if let Some(init_version) = init_version {
            if !init_version.trim().is_empty() {
                return resolve_tag_for_version(repo, init_version.trim())?.ok_or_else(|| {
                    conf_err(format!(
                        "requested version '{}' was not found",
                        init_version.trim()
                    ))
                });
            }
        }
    }
    match resolve_latest_released_target(repo)? {
        Some(resolved) => Ok(resolved),
        None => resolve_remote_head_target(repo),
    }
}

fn is_first_initialization(work_root: &Path) -> RunResult<bool> {
    Ok(!work_root.join(STATE_PATH).exists())
}

fn resolve_latest_released_target(repo: &Repository) -> RunResult<Option<ResolvedTag>> {
    let names = repo
        .tag_names(None)
        .map_err(|e| conf_err(format!("list tags failed: {}", e)))?;
    let latest = names
        .iter()
        .flatten()
        .filter_map(parse_tag_version)
        .max_by(|a, b| a.1.cmp(&b.1))
        .map(|(version, _)| version);
    match latest {
        Some(version) => resolve_tag_for_version(repo, &version),
        None => Ok(None),
    }
}

fn resolve_remote_head_target(repo: &Repository) -> RunResult<ResolvedTag> {
    let head = repo
        .find_reference("refs/remotes/origin/HEAD")
        .map_err(|e| conf_err(format!("resolve origin HEAD failed: {}", e)))?;
    let target_name = head
        .symbolic_target()
        .ok_or_else(|| conf_err("origin HEAD is not a symbolic ref"))?;
    let branch = target_name
        .strip_prefix("refs/remotes/origin/")
        .unwrap_or(target_name)
        .to_string();
    let commit = repo
        .find_reference(target_name)
        .and_then(|reference| reference.peel_to_commit())
        .map_err(|e| {
            conf_err(format!(
                "resolve origin HEAD target {} failed: {}",
                target_name, e
            ))
        })?;
    Ok(ResolvedTag {
        tag: format!("HEAD@{}", branch),
        version: branch,
        commit_id: commit.id(),
    })
}

pub(super) fn resolve_tag_for_version(
    repo: &Repository,
    version: &str,
) -> RunResult<Option<ResolvedTag>> {
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

pub(super) fn checkout_commit(repo: &Repository, commit_id: Oid, tag: &str) -> RunResult<()> {
    let commit = repo
        .find_commit(commit_id)
        .map_err(|e| conf_err(format!("load target commit {} failed: {}", commit_id, e)))?;
    repo.checkout_tree(commit.as_object(), Some(CheckoutBuilder::new().force()))
        .map_err(|e| conf_err(format!("checkout tag {} failed: {}", tag, e)))?;
    repo.set_head_detached(commit_id)
        .map_err(|e| conf_err(format!("set detached HEAD failed: {}", e)))?;
    Ok(())
}
