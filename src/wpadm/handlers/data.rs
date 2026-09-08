use crate::args::{
    CommonFiltArgs, DataArgs, DataCmd, DataStatArgs, DataValidateArgs, StatCmd, StatSinkArgs,
    ValidateCmd, ValidateSinkArgs,
};
use crate::handlers::cli::{dispatch_stat_cmd, dispatch_validate_cmd};
use orion_conf::{EnvTomlLoad, TomlIO};
use orion_error::conversion::{ErrorWith, SourceErr, ToStructError};
use orion_variate::{EnvDict, EnvEvaluable};
use warp_parse::compat::UvsFrom;
use wp_config::sources::types::{WarpSources, WpSource};
use wp_engine::facade::config as constants;
use wp_engine::facade::config::load_warp_engine_confs;
use wp_error::{run_error::RunResult, RunReason};
use wp_log::conf::log_init;
use wp_proj::project::init::PrjScope;
use wp_proj::project::WarpProject;

pub async fn dispatch_data_cmd(sub: DataCmd, dict: &EnvDict) -> RunResult<()> {
    match sub {
        DataCmd::Clean(args) => do_clean(args, dict).await?,
        DataCmd::Check(args) => do_data_check(args, dict).await?,
        DataCmd::Stat(args) => {
            let DataStatArgs { common, command } = args;
            if let Some(sub) = command {
                dispatch_stat_cmd(sub, dict)?;
            } else {
                dispatch_stat_cmd(StatCmd::File(StatSinkArgs { common }), dict)?;
            }
        }
        DataCmd::Validate(args) => {
            let DataValidateArgs {
                work_root,
                input_cnt,
            } = args;
            let common = CommonFiltArgs {
                work_root,
                ..Default::default()
            };
            dispatch_validate_cmd(
                ValidateCmd::SinkFile(ValidateSinkArgs {
                    common,
                    input_cnt,
                    ..Default::default()
                }),
                dict,
            )?;
        }
    }
    Ok(())
}

async fn do_clean(args: DataArgs, dict: &EnvDict) -> RunResult<()> {
    let project = WarpProject::load(&args.work_root, PrjScope::Normal, dict)?;
    project.data_clean(dict)
}

async fn do_data_check(args: DataArgs, dict: &EnvDict) -> RunResult<()> {
    let (conf_manager, main_conf) = load_warp_engine_confs(args.work_root.as_str(), dict)?;
    log_init(main_conf.log_conf()).source_err(RunReason::from_conf(), "init log failed")?;

    let sources_dir = conf_manager.work_root().join(main_conf.src_root());
    let wpsrc_path = sources_dir.join(constants::WPSRC_TOML);

    // Build WpSourcesConfig from either wpsrc.toml or directory-based format
    let sources_config = if wpsrc_path.exists() {
        WarpSources::load_toml(&wpsrc_path)
            .source_err(RunReason::from_conf(), "load wpsrc.toml failed")
            .with_context(&wpsrc_path)
            .doing("load wpsrc.toml")?
    } else {
        // Scan directory for per-source .toml files (new format)
        let mut sources = Vec::new();
        let pattern = format!("{}/**/*.toml", sources_dir.display());
        for entry in glob::glob(&pattern).map_err(|e| {
            RunReason::from_conf()
                .to_err()
                .with_detail(format!("glob pattern failed: {}", e))
        })? {
            let path = entry.map_err(|e| {
                RunReason::from_conf()
                    .to_err()
                    .with_detail(format!("glob iteration failed: {}", e))
            })?;
            if path.file_name().and_then(|n| n.to_str()) == Some(constants::WPSRC_TOML) {
                continue;
            }
            let content = std::fs::read_to_string(&path)
                .source_err(RunReason::from_conf(), "read source config")
                .with_context(&path)
                .doing("read source config")?;
            let source: WpSource = WpSource::env_parse_toml(&content, dict)
                .source_err(RunReason::from_conf(), "parse source config")
                .with_context(&path)
                .doing("parse source config")?
                .env_eval(dict);
            if source.enable.unwrap_or(true) {
                sources.push(source);
            }
        }
        WarpSources { sources }
    };

    // Use SourceConfigParser to validate and build
    let parser =
        wp_engine::sources::SourceConfigParser::new(conf_manager.work_root().to_path_buf());
    let config_str = toml::to_string_pretty(&sources_config)
        .map_err(|err| {
            RunReason::from_conf()
                .to_err()
                .with_detail("serialize source config failed")
                .with_source(err)
        })
        .doing("serialize source config")?;

    match parser.parse_and_build_from(&config_str, dict).await {
        Ok((inits, _)) => println!("data source check ok! enabled: {}", inits.len()),
        Err(err) => {
            return Err(RunReason::data_error()
                .to_err()
                .with_detail("build source config failed")
                .with_source(err));
        }
    }
    Ok(())
}
