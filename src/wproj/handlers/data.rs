use crate::args::{
    CommonFiltArgs, DataArgs, DataCmd, DataStatArgs, DataValidateArgs, StatCmd, StatSinkArgs,
    ValidateCmd, ValidateSinkArgs,
};
use crate::handlers::cli::{dispatch_stat_cmd, dispatch_validate_cmd};
use orion_conf::TomlIO;
use orion_error::{ErrorOwe, ToStructError, UvsDataFrom};
use orion_error::{UvsConfFrom, UvsReason}; // moved from function scope
use wp_conf::sources::types::WarpSources;
use wp_engine::facade::config as constants;
use wp_engine::facade::config::load_warp_engine_confs;
use wp_error::run_error::RunResult;
use wp_log::conf::log_init;
use wp_proj::project::init::PrjScope;
use wp_proj::project::WarpProject;

pub async fn dispatch_data_cmd(sub: DataCmd) -> RunResult<()> {
    match sub {
        DataCmd::Clean(args) => do_clean(args).await?,
        DataCmd::Check(args) => do_data_check(args).await?,
        DataCmd::Stat(args) => {
            let DataStatArgs { common, command } = args;
            if let Some(sub) = command {
                dispatch_stat_cmd(sub)?;
            } else {
                dispatch_stat_cmd(StatCmd::File(StatSinkArgs { common }))?;
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
            dispatch_validate_cmd(ValidateCmd::SinkFile(ValidateSinkArgs {
                common,
                input_cnt,
                ..Default::default()
            }))?;
        }
    }
    Ok(())
}

async fn do_clean(args: DataArgs) -> RunResult<()> {
    let project = WarpProject::load(&args.work_root, PrjScope::Normal)?;
    project.data_clean()
}

async fn do_data_check(args: DataArgs) -> RunResult<()> {
    let (conf_manager, main_conf) = load_warp_engine_confs(args.work_root.as_str())?;
    log_init(main_conf.log_conf()).owe_conf()?;

    // 使用 WarpSources::load_toml 读取 wpsrc.toml 配置
    let wpsrc_path = std::path::PathBuf::from(main_conf.src_conf_of(constants::WPSRC_TOML));
    let sources_config = WarpSources::load_toml(&wpsrc_path).map_err(|e| {
        wp_error::run_error::RunReason::from_conf(format!("Failed to load wpsrc.toml: {}", e))
            .to_err()
    })?;

    // 使用 SourceConfigParser 验证配置并尝试构建（验证配置与依赖）
    let parser = wp_engine::sources::SourceConfigParser::new(conf_manager.work_root().clone());
    let config_str = toml::to_string_pretty(&sources_config).map_err(|e| {
        wp_error::run_error::RunReason::from_conf(format!("Failed to serialize config: {}", e))
            .to_err()
    })?;

    match parser.parse_and_build_from(&config_str).await {
        Ok((inits, _)) => println!("data source check ok! enabled: {}", inits.len()),
        Err(err) => {
            return Err(wp_error::run_error::RunReason::Uvs(UvsReason::from_data(
                err.to_string(),
                None,
            ))
            .to_err())
        }
    }
    Ok(())
}
