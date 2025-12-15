use wp_error::run_error::RunResult;
use wp_proj::sinks::{
    collect_oml_models, expand_route_rows, render_route_rows, render_sink_list, DisplayFormat,
    Sinks,
};

use crate::args::{SinksCommonArgs, SinksRouteArgs};

pub fn list_sinks(args: SinksCommonArgs) -> RunResult<()> {
    let sinks = Sinks::new();
    let rows = sinks.route_rows(&args.work_root, &[], &[])?;
    render_sink_list(&rows, DisplayFormat::from_bool(args.json));
    Ok(())
}

pub fn show_sink_routes(args: SinksRouteArgs) -> RunResult<()> {
    let sinks = Sinks::new();
    let rows = sinks.route_rows(
        &args.common.work_root,
        &args.common.group_names,
        &args.common.sink_names,
    )?;
    let oml_map = collect_oml_models(&args.common.work_root)?;
    let expanded = expand_route_rows(&rows, &oml_map);
    render_route_rows(&expanded, DisplayFormat::from_bool(args.common.json));
    Ok(())
}
