use std::env;
use std::sync::Once;
// 全局分配器：在非 Windows 平台启用 jemalloc，提升多线程分配性能

//use tikv_jemallocator::Jemalloc;

//#[global_allocator]
//static GLOBAL: Jemalloc = Jemalloc;

use tikv_jemallocator::Jemalloc;
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
use shadow_rs::shadow;
shadow!(build);
use clap::Parser;

use warp_parse::load_sec_dict;

use wp_cli_core::split_quiet_args;
use wp_engine::facade::diagnostics::{exit_code_for, print_run_error};
use wp_engine::facade::WpApp;
use wp_error::run_error::RunResult;
mod cli;
static BUILD_INFO_ONCE: Once = Once::new();
fn log_build_info_once() {
    BUILD_INFO_ONCE.call_once(|| {
        wp_log::info_ctrl!(
            "wparse version {} (branch {}, commit {}, built {} via {})",
            build::PKG_VERSION,
            build::BRANCH,
            build::SHORT_COMMIT,
            build::BUILD_TIME_3339,
            build::RUST_VERSION,
        );
    });
}
use crate::cli::WParseCLI;
fn register_extension() {
    // Register all built-in sinks, sources, and optional connectors
    // Using the shared feats module for unified registration
    warp_parse::feats::register_for_runtime();
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    if let Err(e) = do_main().await {
        print_run_error("wparse", &e);
        std::process::exit(exit_code_for(e.reason()));
    }
}

async fn do_main() -> RunResult<()> {
    let argv: Vec<String> = env::args().collect();
    let (_quiet, filtered_args) = split_quiet_args(argv);
    register_extension();
    let cmd = WParseCLI::parse_from(&filtered_args);
    let env_dict = load_sec_dict()?;
    match cmd {
        WParseCLI::Daemon(args) => {
            let engine_args: wp_engine::facade::args::ParseArgs = args.into();

            let mut app = WpApp::try_from(engine_args, env_dict)?;
            log_build_info_once();
            app.run_daemon().await?;
        }
        WParseCLI::Batch(args) => {
            let engine_args: wp_engine::facade::args::ParseArgs = args.into();
            let mut app = WpApp::try_from(engine_args, env_dict)?;
            log_build_info_once();
            app.run_batch().await?;
        }
    }
    Ok(())
}
