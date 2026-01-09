use std::time::Duration;

use orion_error::{ErrorConv, ErrorOwe, ErrorWith, ToStructError, UvsConfFrom};
use orion_variate::EnvDict;
use tokio::time::sleep;
use wp_engine::facade::config::WarpConf;
use wp_engine::facade::generator::{GenGRA, RuleGRA};
use wp_error::{run_error::RunReason, RunResult};
use wp_log::conf::log_init;
use wp_proj::wpgen::load_wpgen_resolved;

// Handler for `wpgen rule` subcommand.
pub async fn run(
    work_root: &str,
    wpl_dir: Option<&str>,
    conf_name: &str,
    stat_print: bool,
    line_cnt: Option<usize>,
    gen_speed: Option<usize>,
    stat_sec: usize,
    dict: &EnvDict,
) -> RunResult<()> {
    // no direct use of SinkBackendType when using direct runner

    let god = WarpConf::new(work_root);
    // Register built-in sink factories (file/syslog/etc.)
    wp_engine::sinks::register_builtin_sinks();
    // 1) 判断配置文件是否存在
    let conf_path = god.config_path(conf_name);
    if !std::path::Path::new(&conf_path).exists() {
        return RunReason::from_conf("config file not found")
            .err_result()
            .with(&conf_path);
    }
    wp_log::info_ctrl!("wpgen.rule: loading config from '{}'", conf_path.display());
    let rt = load_wpgen_resolved(conf_name, &god, dict).err_conv()?;
    // init logging
    log_init(&rt.conf.logging.to_log_conf()).owe_conf()?;
    wp_proj::wpgen::log_resolved_out_sink(&rt);

    // direct runner builds sink instances from resolved spec; no need to pre-build here
    // build GenGRA from conf (simple mapping)
    let g = &rt.conf.generator;
    let gen_conf = GenGRA {
        total_line: line_cnt.or(g.count),
        gen_speed: gen_speed.unwrap_or(g.speed),
        parallel: rt.conf.generator.parallel,
        stat_sec,
        stat_print,
        rescue: "./data/rescue".to_string(),
    };
    // keep a placeholder tuple to minimize diff; not used in direct flow
    let _prepared = (
        RuleGRA {
            gen_conf: gen_conf.clone(),
        },
        (),
    );
    // run
    let default_rule_root = god
        .load_engine_config(dict)
        .err_conv()?
        .rule_root()
        .to_string();
    let rule_root = wpl_dir.unwrap_or(default_rule_root.as_str());
    // 诊断日志
    let wf_gen_batch = std::env::var("WF_GEN_BATCH").unwrap_or_else(|_| "(unset)".into());
    let wf_gen_unit = std::env::var("WF_GEN_UNIT_SIZE").unwrap_or_else(|_| "(unset)".into());
    wp_log::info_ctrl!(
        "wpgen.rule: rule_root='{}', parallel={}, total_line={:?}, gen_speed={:?}, stat_sec={}, env: WF_GEN_BATCH={}, WF_GEN_UNIT_SIZE={}",
        rule_root,
        rt.conf.generator.parallel,
        gen_conf.total_line,
        gen_conf.gen_speed,
        gen_conf.stat_sec,
        wf_gen_batch,
        wf_gen_unit
    );
    // 使用 out_sink 规格（未构建的 SinkInstanceConf）调用直连 runner
    wp_proj::wpgen::rule_exec_direct_core(
        stat_print,
        rule_root,
        (RuleGRA { gen_conf }, rt.out_sink.clone()),
        gen_speed.unwrap_or(0),
    )
    .await?;
    sleep(Duration::from_secs(2)).await;
    wp_log::info_ctrl!("wpgen.rule: completed");
    Ok(())
}
