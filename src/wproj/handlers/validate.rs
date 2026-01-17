use orion_conf::ToStructError;
use orion_variate::EnvDict;
use serde_json::json;
use wp_cli_core::{
    self as wlib,
    utils::validate::{validate_groups, validate_with_stats},
};
use wp_error::run_error::RunResult;
use wp_proj::sinks::{stat::SinkStatFilters, validate::prepare_validate_context};

use crate::{args::ValidateSinkArgs, format::print_json};

pub fn run_sink_validation(args: &ValidateSinkArgs, dict: &EnvDict) -> RunResult<()> {
    let filters = SinkStatFilters::new(
        args.common.work_root.as_str(),
        &args.common.group_names,
        &args.common.sink_names,
        &args.common.path_like,
    );
    let ctx = prepare_validate_context(&filters, args.stats_file.as_deref(), dict)?;
    let input_override = args.input_cnt.or(ctx.input_from_sources);

    let report = match ctx.stats.as_ref() {
        Some(stats) => validate_with_stats(&ctx.groups, Some(stats), input_override),
        None => validate_groups(&ctx.groups, input_override),
    };

    if args.common.json {
        let obj = json!({
            "pass": !report.has_error_fail(),
            "issues": report.items.iter().map(|it| json!({
                "severity": match it.severity {
                    wlib::Severity::Warn => "WARN",
                    wlib::Severity::Error => "ERROR",
                    wlib::Severity::Panic => "PANIC",
                },
                "group": it.group,
                "sink": it.sink,
                "msg": it.msg
            })).collect::<Vec<_>>()
        });
        print_json(&obj)?;
    } else {
        wlib::print_validate_headline(&report);
        if args.verbose {
            wlib::print_validate_tables_verbose(&ctx.groups, ctx.stats.as_ref(), input_override);
        } else {
            wlib::print_validate_tables(&ctx.groups, ctx.stats.as_ref(), input_override);
        }
    }

    if report.has_error_fail() {
        return Err(
            wp_error::run_error::RunReason::Uvs(orion_error::UvsReason::validation_error(
                "validate failed".to_string(),
            ))
            .to_err(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        args::{CommonFiltArgs, ValidateCmd, ValidateSinkArgs},
        handlers::cli::dispatch_validate_cmd,
    };
    use orion_variate::EnvDict;

    #[test]
    fn wproj_validate_sink_file_runs() {
        let work_root = std::path::Path::new("usecase/core/getting_started");
        if !work_root.exists() {
            eprintln!(
                "skip wproj_validate_sink_file_runs: sample work_root {:?} 不存在",
                work_root
            );
            return;
        }
        let args = ValidateSinkArgs {
            common: CommonFiltArgs {
                work_root: work_root.to_string_lossy().into_owned(),
                group_names: vec![],
                sink_names: vec![],
                path_like: None,
                json: true,
            },
            input_cnt: None,
            stats_file: None,
            verbose: false,
        };
        let cmd = ValidateCmd::SinkFile(args);
        let _ = dispatch_validate_cmd(cmd, &EnvDict::default());
    }
}
