use orion_error::ErrorConv;
use wp_cli_core::connectors::sources as sources_core;
use wp_error::run_error::RunResult;
use wp_proj::sources::Sources;

use crate::args::SourcesCommonArgs;

pub fn list_sources_for_cli(sources: &Sources, args: &SourcesCommonArgs) -> RunResult<()> {
    let rows = sources_core::route_table(&args.work_root, None).err_conv()?;
    if args.json {
        sources.display_as_json(&rows);
    } else {
        sources.display_as_table(&rows);
    }
    Ok(())
}
