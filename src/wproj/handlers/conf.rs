use std::path::PathBuf;

use crate::args::ConfUpdateArgs;
use crate::format::print_json;
use orion_error::conversion::ToStructError;
use warp_parse::compat::UvsFrom;
use warp_parse::project_remote;
use warp_parse::project_remote::RemoteGroup;
use wp_engine::facade::args::ParseArgs;
use wp_engine::facade::WpApp;
use wp_error::run_error::{RunReason, RunResult};
use wp_log::{info_ctrl, warn_ctrl};

pub async fn run_conf_update(args: ConfUpdateArgs) -> RunResult<()> {
    let work_root = resolve_work_root(&args.work_root)?;
    let group = parse_group(args.group.as_deref())?;
    run_conf_update_with_sync(
        work_root,
        args.version.as_deref(),
        args.json,
        group,
        |work_root, requested_version, dict, group| match group {
            Some(g) => project_remote::sync_project_remote_group_with_dict(
                work_root,
                g,
                requested_version,
                dict,
            ),
            None => {
                project_remote::sync_project_remote_with_dict(work_root, requested_version, dict)
            }
        },
    )
    .await
}

fn parse_group(raw: Option<&str>) -> RunResult<Option<RemoteGroup>> {
    match raw {
        None | Some("") => Ok(None),
        Some(s) => s
            .parse::<RemoteGroup>()
            .map(Some)
            .map_err(|e| RunReason::from_conf().to_err().with_detail(e)),
    }
}

pub async fn run_conf_update_from_repo(
    work_root: &str,
    repo_url: &str,
    requested_version: Option<&str>,
) -> RunResult<()> {
    let work_root = resolve_work_root(work_root)?;
    info_ctrl!(
        "wproj conf update bootstrap source work_root={} requested_version={} repo={}",
        work_root.display(),
        requested_version.unwrap_or("(auto)"),
        repo_url
    );
    run_conf_update_with_sync(
        work_root,
        requested_version,
        false,
        None,
        |work_root, requested_version, _dict, _group| {
            project_remote::sync_project_remote_from_repo(work_root, repo_url, requested_version)
        },
    )
    .await
}

async fn run_conf_update_with_sync<F>(
    work_root: PathBuf,
    requested_version: Option<&str>,
    json: bool,
    group: Option<RemoteGroup>,
    sync_fn: F,
) -> RunResult<()>
where
    F: Fn(
        &std::path::Path,
        Option<&str>,
        &orion_variate::EnvDict,
        Option<RemoteGroup>,
    ) -> RunResult<project_remote::ProjectRemoteUpdateResult>,
{
    info_ctrl!(
        "wproj conf update start work_root={} requested_version={} json={} group={}",
        work_root.display(),
        requested_version.unwrap_or("(auto)"),
        json,
        group
            .map(|g| match g {
                RemoteGroup::Models => "models",
                RemoteGroup::Infra => "infra",
            })
            .unwrap_or("-")
    );
    let _lock_guard = project_remote::acquire_project_remote_lock(&work_root)?;
    let rollback_snapshot =
        project_remote::capture_project_remote_snapshot_with_group(&work_root, group)?;
    let dict = warp_parse::load_sec_dict()?;
    let result = sync_fn(&work_root, requested_version, &dict, group)?;
    info_ctrl!(
        "wproj conf update synced work_root={} requested_version={} current_version={} resolved_tag={} from_revision={} to_revision={} changed={}",
        work_root.display(),
        requested_version.unwrap_or("(auto)"),
        result.current_version,
        result.resolved_tag,
        result.from_revision.as_deref().unwrap_or("-"),
        result.to_revision,
        result.changed
    );

    let check_result = async {
        info_ctrl!(
            "wproj conf update validate start work_root={} version={}",
            work_root.display(),
            result.current_version
        );
        let _cwd_guard = WorkRootGuard::enter(&work_root)?;
        validate_load_model(&work_root, &dict).await
    }
    .await;

    if let Err(check_err) = check_result {
        warn_ctrl!(
            "wproj conf update validate failed work_root={} requested_version={} current_version={} resolved_tag={} error={}",
            work_root.display(),
            requested_version.unwrap_or("(auto)"),
            result.current_version,
            result.resolved_tag,
            check_err
        );
        if let Err(rollback_err) = project_remote::restore_project_remote_update(
            &work_root,
            &rollback_snapshot,
            result.changed,
        ) {
            warn_ctrl!(
                "wproj conf update rollback failed work_root={} requested_version={} current_version={} resolved_tag={} error={}",
                work_root.display(),
                requested_version.unwrap_or("(auto)"),
                result.current_version,
                result.resolved_tag,
                rollback_err
            );
            return Err(RunReason::from_conf().to_err().with_detail(format!(
                "project check failed after update: {}; rollback failed: {}",
                check_err, rollback_err
            )));
        }
        info_ctrl!(
            "wproj conf update rollback done work_root={} requested_version={} reverted_from_version={} resolved_tag={} changed={}",
            work_root.display(),
            requested_version.unwrap_or("(auto)"),
            result.current_version,
            result.resolved_tag,
            result.changed
        );
        return Err(RunReason::from_conf().to_err().with_detail(format!(
            "project check failed after update: {}",
            check_err.display_chain()
        )));
    }
    info_ctrl!(
        "wproj conf update validate passed work_root={} requested_version={} current_version={} resolved_tag={}",
        work_root.display(),
        requested_version.unwrap_or("(auto)"),
        result.current_version,
        result.resolved_tag
    );

    if json {
        info_ctrl!(
            "wproj conf update done work_root={} requested_version={} current_version={} resolved_tag={} json=true",
            work_root.display(),
            requested_version.unwrap_or("(auto)"),
            result.current_version,
            result.resolved_tag
        );
        return print_json(&result);
    }

    info_ctrl!(
        "wproj conf update done work_root={} requested_version={} current_version={} resolved_tag={} json=false",
        work_root.display(),
        requested_version.unwrap_or("(auto)"),
        result.current_version,
        result.resolved_tag
    );
    println!("Project remote update");
    println!("  Work Root : {}", work_root.display());
    println!(
        "  Request   : {}",
        result.requested_version.as_deref().unwrap_or("(auto)")
    );
    println!("  Version   : {}", result.current_version);
    println!("  Tag       : {}", result.resolved_tag);
    println!(
        "  From      : {}",
        result.from_revision.as_deref().unwrap_or("-")
    );
    println!("  To        : {}", result.to_revision);
    println!("  Changed   : {}", result.changed);
    Ok(())
}

async fn validate_load_model(
    work_root: &std::path::Path,
    dict: &orion_variate::EnvDict,
) -> RunResult<()> {
    let parse_args = ParseArgs {
        work_root: Some(work_root.display().to_string()),
        ..Default::default()
    };
    let mut app = WpApp::try_from(parse_args, dict.clone())?;
    app.validate_load_model().await
}

fn resolve_work_root(raw: &str) -> RunResult<PathBuf> {
    std::fs::canonicalize(raw).map_err(|e| {
        RunReason::from_conf()
            .to_err()
            .with_detail(format!("resolve work root '{}' failed", raw))
            .with_source(e)
    })
}

struct WorkRootGuard {
    original: PathBuf,
}

impl WorkRootGuard {
    fn enter(path: &std::path::Path) -> RunResult<Self> {
        let original = std::env::current_dir().map_err(|e| {
            RunReason::from_conf()
                .to_err()
                .with_detail("read current dir failed")
                .with_source(e)
        })?;
        std::env::set_current_dir(path).map_err(|e| {
            RunReason::from_conf()
                .to_err()
                .with_detail(format!("set current dir to '{}' failed", path.display()))
                .with_source(e)
        })?;
        Ok(Self { original })
    }
}

impl Drop for WorkRootGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}
