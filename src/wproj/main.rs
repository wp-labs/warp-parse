use clap::Parser;
mod args;
mod format;
use libc::exit;
use std::env;
use wp_engine::facade::diagnostics;
use wp_error::run_error::RunResult;
use wpcnt_lib::banner::split_quiet_args;

use crate::args::WProjCli;
mod handlers;
#[tokio::main(flavor = "multi_thread")]
async fn main() {
    if let Err(e) = do_main().await {
        unsafe {
            diagnostics::print_run_error("wproj", &e);
            exit(diagnostics::exit_code_for(e.reason()));
        }
    }
}

async fn do_main() -> RunResult<()> {
    let (_pre_quiet, filtered_args) = split_quiet_args(env::args().collect());
    warp_parse::feats::register_for_runtime();
    let wcl = WProjCli::parse_from(&filtered_args);
    handlers::cli::dispatch_cli(wcl).await
}
// Banner is centralized in wp-cli-utils
