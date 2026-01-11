use crate::args::{KnowdbCmd, ModelCmd, StatCmd, ValidateCmd, WProj, WProjCli};
use crate::handlers::rule::dispatch_rule_cmd;
use crate::handlers::sinks::{list_sinks, show_sink_routes};
use crate::handlers::sources::list_sources_for_cli;
use crate::handlers::stat::{run_combined_stat, run_sink_stat, run_src_stat};
use crate::handlers::validate::run_sink_validation;
use crate::handlers::{data, knowdb, project};
use orion_variate::EnvDict;
use warp_parse::load_sec_dict;
use wp_error::run_error::RunResult;

pub async fn dispatch_cli(cli: WProjCli) -> RunResult<()> {
    let dict = load_sec_dict()?;
    match cli.cmd {
        WProj::Rule(sub) => dispatch_rule_cmd(sub, &dict)?,
        WProj::Init(args) => project::init_project(args, &dict)?,
        WProj::Check(args) => project::check_project(args, &dict)?,
        WProj::Data(sub) => data::dispatch_data_cmd(sub, &dict).await?,
        WProj::Model(sub) => dispatch_model_cmd(sub, &dict)?,
    }
    Ok(())
}

fn dispatch_model_cmd(cmd: ModelCmd, dict: &EnvDict) -> RunResult<()> {
    match cmd {
        ModelCmd::Sources(args) => list_sources_for_cli(&args, dict),
        ModelCmd::Sinks(args) => list_sinks(args, dict),
        ModelCmd::Route(args) => show_sink_routes(args, dict),
        ModelCmd::Knowdb(sub) => dispatch_knowdb_cmd(sub, dict),
    }
}

fn dispatch_knowdb_cmd(cmd: KnowdbCmd, dict: &EnvDict) -> RunResult<()> {
    match cmd {
        KnowdbCmd::Init(args) => knowdb::init_knowdb(&args),
        KnowdbCmd::Check(args) => knowdb::check_knowdb(&args, dict),
        KnowdbCmd::Clean(args) => knowdb::clean_knowdb(&args),
    }
}

pub fn dispatch_stat_cmd(sub: StatCmd, dict: &EnvDict) -> RunResult<()> {
    match sub {
        StatCmd::File(a) => run_combined_stat(&a.common, dict),
        StatCmd::SrcFile(a) => run_src_stat(&a.common, dict),
        StatCmd::SinkFile(a) => run_sink_stat(&a.common, dict),
    }
}

pub fn dispatch_validate_cmd(sub: ValidateCmd, dict: &EnvDict) -> RunResult<()> {
    match sub {
        ValidateCmd::SinkFile(args) => run_sink_validation(&args, dict),
    }
}

// No dedicated sink-init command exposed via CLI currently
