use orion_error::ErrorConv;
use orion_variate::EnvDict;
use std::sync::Arc;
use wp_conf::engine::EngineConfig;
use wp_error::run_error::RunResult;
use wp_proj::sinks::{
    collect_oml_models, expand_route_rows, render_route_rows, render_sink_list, DisplayFormat,
    Sinks,
};

use crate::args::{SinksCommonArgs, SinksRouteArgs};

fn load_sinks(work_root: &str, dict: &EnvDict) -> RunResult<Sinks> {
    let eng_conf = Arc::new(EngineConfig::load_or_init(work_root, dict).err_conv()?);
    Ok(Sinks::new(work_root, eng_conf))
}

pub fn list_sinks(args: SinksCommonArgs, dict: &EnvDict) -> RunResult<()> {
    let sinks = load_sinks(&args.work_root, dict)?;
    let rows = sinks.route_rows(&[], &[])?;
    render_sink_list(&rows, DisplayFormat::from_bool(args.json));
    Ok(())
}

pub fn show_sink_routes(args: SinksRouteArgs, dict: &EnvDict) -> RunResult<()> {
    let sinks = load_sinks(&args.common.work_root, dict)?;
    let rows = sinks.route_rows(&args.common.group_names, &args.common.sink_names)?;
    let oml_map = collect_oml_models(&args.common.work_root)?;
    let expanded = expand_route_rows(&rows, &oml_map);
    render_route_rows(&expanded, DisplayFormat::from_bool(args.common.json));
    Ok(())
}
