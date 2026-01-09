use clap::Parser;
use orion_error::ErrorConv;
use orion_sec::load_sec_dict_by;
use std::env;
use wpcnt_lib::banner::split_quiet_args;

use orion_conf::ToStructError;
use orion_conf::UvsConfFrom;
use wp_engine::facade::cli::WParseCLI;
use wp_engine::facade::WpRescueApp;
use wp_error::run_error::RunResult;
use wp_error::RunReason;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> RunResult<()> {
    warp_parse::feats::register_for_runtime();
    let argv: Vec<String> = env::args().collect();
    let (_quiet, filtered_args) = split_quiet_args(argv);
    let env_dict = load_sec_dict_by(".warp_parse", "sec_key.toml", orion_sec::SecFileFmt::Toml)
        .map_err(|e| RunReason::from_conf(format!("{}", e)).to_err())?;

    let cmd = WParseCLI::parse_from(&filtered_args);
    match cmd {
        WParseCLI::Daemon(_) => {
            eprintln!("wprescue 仅支持 batch 模式（常驻服务）");
            std::process::exit(2);
        }
        WParseCLI::Batch(args) => {
            let mut app = WpRescueApp::try_from(args, env_dict).err_conv()?;
            if let Err(e) = app.run_batch().await {
                wp_engine::facade::diagnostics::print_run_error("wprescue", &e);
                std::process::exit(wp_engine::facade::diagnostics::exit_code_for(e.reason()));
            }
        }
    }

    Ok(())
}
