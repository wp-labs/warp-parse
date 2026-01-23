use std::time::Duration;

use orion_error::{ErrorConv, ErrorOwe, UvsReason};
use orion_variate::EnvDict;
use tokio::time::sleep;
use wp_log::info_ctrl;
use wp_proj::wpgen::load_wpgen_resolved;
use wp_proj::wpgen::{log_resolved_out_sink, sample_exec_direct_core};
// no need to pre-build sink here; direct core builds from out_sink spec
use wp_engine::facade::config::WarpConf;
use wp_engine::facade::generator::{GenGRA, SampleGRA};
use wp_engine::runtime::generator::SpeedProfile;
use wp_error::run_error::{RunError, RunReason};
use wp_error::RunResult;
use wp_log::conf::log_init;

#[derive(Debug, Clone, Copy)]
pub struct SampleRunOpts {
    pub print_stat: bool,
    pub line_cnt: Option<usize>,
    pub gen_speed: Option<usize>,
    pub stat_sec: usize,
}

impl SampleRunOpts {
    pub fn new(
        print_stat: bool,
        line_cnt: Option<usize>,
        gen_speed: Option<usize>,
        stat_sec: usize,
    ) -> Self {
        Self {
            print_stat,
            line_cnt,
            gen_speed,
            stat_sec,
        }
    }
}

// Handler for `wpgen sample` subcommand.
pub async fn run(
    work_root: &str,
    wpl_dir: Option<&str>,
    conf_name: &str,
    opts: SampleRunOpts,
    dict: &EnvDict,
) -> RunResult<()> {
    let SampleRunOpts {
        print_stat,
        line_cnt,
        gen_speed,
        stat_sec,
    } = opts;
    // no direct use of SinkBackendType in direct mode

    let god = WarpConf::new(work_root);
    wp_engine::sinks::register_builtin_sinks();
    // 1) 判断配置文件是否存在，提前给出清晰提示
    let conf_path = god.config_path(conf_name);
    if !std::path::Path::new(&conf_path).exists() {
        return Err(RunError::from(RunReason::Uvs(UvsReason::core_conf(
            format!("config file not found: {}", conf_path.display()),
        ))));
    }
    info_ctrl!(
        "wpgen.sample: loading config from '{}'",
        conf_path.display()
    );
    let resolved = load_wpgen_resolved(conf_name, &god, dict).err_conv()?;
    log_init(&resolved.conf.logging.to_log_conf()).owe_res()?;
    log_resolved_out_sink(&resolved);
    let conf = &resolved.conf.generator;
    // 如果命令行指定了 gen_speed，使用恒定速率；否则使用配置中的 speed_profile
    let speed_profile: Option<SpeedProfile> = if gen_speed.is_some() {
        None // 使用 gen_speed 作为恒定速率
    } else {
        conf.speed_profile.clone().map(|p| p.into())
    };
    let gen_rt = GenGRA {
        total_line: line_cnt.or(conf.count),
        gen_speed: gen_speed.unwrap_or(conf.speed),
        speed_profile,
        parallel: resolved.conf.generator.parallel,
        stat_sec,
        stat_print: print_stat,
        rescue: "./data/rescue".to_string(),
    };
    let _prepared = (
        SampleGRA {
            gen_conf: gen_rt.clone(),
        },
        (),
    );
    // 默认从 ./models/wpl/ 搜索样本；用户可通过 --wpl 指定其他根目录
    let default_rule_root = "./models/wpl".to_string();
    let rule_root = wpl_dir.unwrap_or(default_rule_root.as_str());
    // 诊断：打印关键参数与环境覆盖
    let wf_gen_batch = std::env::var("WF_GEN_BATCH").unwrap_or_else(|_| "(unset)".into());
    let wf_gen_unit = std::env::var("WF_GEN_UNIT_SIZE").unwrap_or_else(|_| "(unset)".into());
    info_ctrl!(
        "wpgen.sample: rule_root='{}', find_name='sample.dat', parallel={}, total_line={:?}, gen_speed={:?}, stat_sec={}, env: WF_GEN_BATCH={}, WF_GEN_UNIT_SIZE={}",
        rule_root,
        resolved.conf.generator.parallel,
        gen_rt.total_line,
        gen_rt.gen_speed,
        gen_rt.stat_sec,
        wf_gen_batch,
        wf_gen_unit
    );
    // 使用 out_sink 规格按副本构建 sink，并行直连发送
    sample_exec_direct_core(
        rule_root,
        "sample.dat",
        (SampleGRA { gen_conf: gen_rt }, resolved.out_sink.clone()),
        gen_speed.unwrap_or(0),
    )
    .await?;
    sleep(Duration::from_secs(2)).await;
    // 明确提示任务完成
    info_ctrl!("wpgen.sample: completed");
    Ok(())
}
