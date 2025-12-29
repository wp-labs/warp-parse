use orion_error::ErrorConv;
use std::sync::Arc;
use wp_cli_core::connectors::sources as sources_core;
use wp_conf::engine::EngineConfig;
use wp_error::run_error::RunResult;
use wp_proj::sources::Sources;

use crate::args::SourcesCommonArgs;

pub fn list_sources_for_cli(args: &SourcesCommonArgs) -> RunResult<()> {
    let eng_conf = Arc::new(EngineConfig::load_or_init(&args.work_root).err_conv()?);
    let sources = Sources::new(&args.work_root, eng_conf.clone());
    let rows = sources_core::route_table(&args.work_root, eng_conf.as_ref(), None).err_conv()?;
    if args.json {
        sources.display_as_json(&rows);
    } else {
        sources.display_as_table(&rows);
    }
    Ok(())
}
