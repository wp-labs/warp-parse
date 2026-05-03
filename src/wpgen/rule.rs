use std::time::Duration;

use orion_error::{conversion::ToStructError, ErrorConv, ErrorWrapAs, UvsFrom};
use orion_variate::EnvDict;
use tokio::time::sleep;
use wp_engine::facade::config::WarpConf;
use wp_engine::facade::generator::{GenGRA, RuleGRA};
use wp_engine::runtime::generator::SpeedProfile;
use wp_error::{run_error::RunReason, RunResult};
use wp_log::conf::log_init;
use wp_proj::wpgen::load_wpgen_resolved;

#[derive(Debug, Clone, Copy)]
pub struct RuleRunOpts {
    pub stat_print: bool,
    pub line_cnt: Option<usize>,
    pub gen_speed: Option<usize>,
    pub stat_sec: usize,
}

impl RuleRunOpts {
    pub fn new(
        stat_print: bool,
        line_cnt: Option<usize>,
        gen_speed: Option<usize>,
        stat_sec: usize,
    ) -> Self {
        Self {
            stat_print,
            line_cnt,
            gen_speed,
            stat_sec,
        }
    }
}

// Handler for `wpgen rule` subcommand.
pub async fn run(
    work_root: &str,
    wpl_dir: Option<&str>,
    conf_name: &str,
    opts: RuleRunOpts,
    dict: &EnvDict,
) -> RunResult<()> {
    let RuleRunOpts {
        stat_print,
        line_cnt,
        gen_speed,
        stat_sec,
    } = opts;

    let god = WarpConf::new(work_root);
    wp_engine::sinks::register_builtin_sinks();
    let conf_path = god.config_path(conf_name);
    if !std::path::Path::new(&conf_path).exists() {
        return Err(RunReason::from_conf()
            .to_err()
            .with_detail(format!("config file not found: {}", conf_path.display())));
    }
    wp_log::info_ctrl!("wpgen.rule: loading config from '{}'", conf_path.display());
    let rt = load_wpgen_resolved(conf_name, &god, dict).err_conv()?;
    log_init(&rt.conf.logging.to_log_conf()).wrap_as(RunReason::from_conf(), "init log failed")?;
    wp_proj::wpgen::log_resolved_out_sink(&rt);

    let g = &rt.conf.generator;
    let speed_profile: Option<SpeedProfile> = if gen_speed.is_some() {
        None
    } else {
        g.speed_profile.clone().map(|p| p.into())
    };
    let gen_conf = GenGRA {
        total_line: line_cnt.or(g.count),
        gen_speed: gen_speed.unwrap_or(g.speed),
        speed_profile,
        parallel: rt.conf.generator.parallel,
        stat_sec,
        stat_print,
        rescue: "./data/rescue".to_string(),
    };
    let _prepared = (
        RuleGRA {
            gen_conf: gen_conf.clone(),
        },
        (),
    );
    let default_rule_root = god
        .load_engine_config(dict)
        .err_conv()?
        .rule_root()
        .to_string();
    let rule_root = wpl_dir.unwrap_or(default_rule_root.as_str());
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
    wp_proj::wpgen::rule_exec_direct_core(
        stat_print,
        rule_root,
        (RuleGRA { gen_conf }, rt.out_sink.clone()),
        gen_speed.unwrap_or(0),
        dict,
    )
    .await?;
    sleep(Duration::from_secs(2)).await;
    wp_log::info_ctrl!("wpgen.rule: completed");
    Ok(())
}
