use orion_error::conversion::SourceErr;
use orion_variate::EnvDict;
use std::sync::Arc;
use warp_parse::compat::UvsFrom;
use wp_cli_core::business::connectors::sources as sources_core;
use wp_error::run_error::{RunReason, RunResult};
use wp_proj::sources::Sources;

use crate::args::SourcesCommonArgs;
use crate::handlers::engine_config::load_resolved_engine_config;

pub fn list_sources_for_cli(args: &SourcesCommonArgs, dict: &EnvDict) -> RunResult<()> {
    let eng_conf = Arc::new(load_resolved_engine_config(&args.work_root, dict)?);
    let sources = Sources::new(&args.work_root, eng_conf.clone());
    let rows = sources_core::route_table(&args.work_root, eng_conf.as_ref(), None, dict)
        .source_err(RunReason::from_conf(), "load source route table failed")?;
    if args.json {
        sources.display_as_json(&rows);
    } else {
        sources.display_as_table(&rows);
    }
    Ok(())
}
