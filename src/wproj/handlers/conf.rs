use std::path::PathBuf;

use crate::args::ConfUpdateArgs;
use crate::format::print_json;
use orion_error::{ToStructError, UvsFrom};
use warp_parse::project_remote;
use wp_error::run_error::{RunReason, RunResult};

pub fn run_conf_update(args: ConfUpdateArgs) -> RunResult<()> {
    let work_root = resolve_work_root(&args.work_root)?;
    let result = project_remote::sync_project_remote(&work_root, args.version.as_deref())?;
    if args.json {
        return print_json(&result);
    }

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

fn resolve_work_root(raw: &str) -> RunResult<PathBuf> {
    std::fs::canonicalize(raw).map_err(|e| {
        RunReason::from_conf()
            .to_err()
            .with_detail(format!("resolve work root '{}' failed: {}", raw, e))
    })
}
