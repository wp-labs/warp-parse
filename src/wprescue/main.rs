use clap::{Args, Parser};
use orion_error::ErrorConv;
use std::env;
use warp_parse::build::CLAP_LONG_VERSION;
use warp_parse::load_sec_dict;
use wp_cli_core::split_quiet_args;

use wp_engine::facade::WpRescueApp;
use wp_error::error_handling::RobustnessMode;
use wp_error::run_error::RunResult;

#[derive(Parser)]
#[command(
    name = "wprescue",
    version = CLAP_LONG_VERSION,
    about = "Warp Parse rescue CLI"
)]
enum WpRescueCli {
    #[command(name = "daemon", visible_alias = "deamon")]
    Daemon(CliParseArgs),

    #[command(name = "batch")]
    Batch(CliParseArgs),
}

#[derive(Args, Debug, Default, Clone)]
struct CliParseArgs {
    #[clap(long, default_value = None)]
    work_root: Option<String>,
    #[clap(short, long, default_value = "p")]
    mode: String,
    #[clap(short = 'n', long, default_value = None)]
    max_line: Option<usize>,
    #[clap(short = 'w', long = "parse-workers")]
    parse_workers: Option<usize>,
    #[clap(short = 'S', long)]
    check_stop: Option<usize>,
    #[clap(short = 's', long)]
    check_continue: Option<usize>,
    #[clap(long = "stat")]
    stat_sec: Option<usize>,
    #[clap(long = "robust")]
    robust: Option<RobustnessMode>,
    #[clap(short = 'p', long = "print_stat", default_value = "false")]
    stat_print: bool,
    #[clap(long = "log-profile")]
    log_profile: Option<String>,
    #[clap(long = "wpl")]
    wpl_dir: Option<String>,
}

impl From<CliParseArgs> for wp_engine::facade::args::ParseArgs {
    fn from(value: CliParseArgs) -> Self {
        wp_engine::facade::args::ParseArgs {
            work_root: value.work_root,
            mode: value.mode,
            max_line: value.max_line,
            parse_workers: value.parse_workers,
            reload_timeout_ms: None,
            check_stop: value.check_stop,
            check_continue: value.check_continue,
            stat_sec: value.stat_sec,
            robust: value.robust,
            stat_print: value.stat_print,
            log_profile: value.log_profile,
            wpl_dir: value.wpl_dir,
        }
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> RunResult<()> {
    warp_parse::feats::register_for_runtime();
    let argv: Vec<String> = env::args().collect();
    let (_quiet, filtered_args) = split_quiet_args(argv);
    let env_dict = load_sec_dict()?;

    let cmd = WpRescueCli::parse_from(&filtered_args);
    match cmd {
        WpRescueCli::Daemon(_) => {
            eprintln!("wprescue 仅支持 batch 模式（常驻服务）");
            std::process::exit(2);
        }
        WpRescueCli::Batch(args) => {
            let mut app = WpRescueApp::try_from(args.into(), env_dict).err_conv()?;
            if let Err(e) = app.run_batch().await {
                wp_engine::facade::diagnostics::print_run_error("wprescue", &e);
                std::process::exit(wp_engine::facade::diagnostics::exit_code_for(e.reason()));
            }
        }
    }

    Ok(())
}
