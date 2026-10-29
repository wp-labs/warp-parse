use std::env;
use std::path::Path;
// 全局分配器：在非 Windows 平台启用 jemalloc，提升多线程分配性能

//use tikv_jemallocator::Jemalloc;

//#[global_allocator]
//static GLOBAL: Jemalloc = Jemalloc;

use tikv_jemallocator::Jemalloc;
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
use clap::Parser;

use warp_parse::compat::UvsFrom;
use warp_parse::{init_rustls_crypto_provider, load_sec_dict, log_build_info_once};

use orion_error::conversion::ToStructError;
use wp_cli_core::split_quiet_args;
use wp_engine::facade::diagnostics::{exit_code_for, print_run_error};
use wp_engine::facade::WpApp;
use wp_error::run_error::{RunReason, RunResult};
mod cli;

use crate::cli::WParseCLI;
fn register_extension() {
    // Register all built-in sinks, sources, and optional connectors
    // Using the shared feats module for unified registration
    warp_parse::feats::register_for_runtime();
}

/// Apply the effective state directory (CLI `--state-dir` wins over the site
/// overlay and the engine config) and prime the process cache used by
/// maintenance paths.
fn inject_state_dir(
    work_root: &Path,
    dict: &orion_variate::EnvDict,
    args: &mut wp_engine::facade::args::ParseArgs,
) -> RunResult<()> {
    let state_dir =
        warp_parse::state::resolve_effective_state_dir(work_root, dict, args.state_dir.as_deref())?;
    args.state_dir = Some(state_dir.to_path_buf().to_string_lossy().to_string());
    Ok(())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    if let Err(e) = do_main().await {
        print_run_error("wparse", &e);
        std::process::exit(exit_code_for(e.reason()));
    }
}

async fn do_main() -> RunResult<()> {
    init_rustls_crypto_provider();
    let argv: Vec<String> = env::args().collect();
    let (_quiet, filtered_args) = split_quiet_args(argv);
    register_extension();
    let cmd = WParseCLI::parse_from(&filtered_args);
    // 实例级环境文件：多个 wparse 共享 wparse.toml，但各自 work_root 下的 .env 独立。
    let instance_root = match &cmd {
        WParseCLI::Daemon(args) | WParseCLI::Batch(args) => Some(
            wp_engine::facade::args::resolve_run_work_root(&args.work_root)?,
        ),
        _ => None,
    };
    if let Some(root) = instance_root.as_deref() {
        warp_parse::load_instance_env(Path::new(root))?;
    }
    let env_dict = load_sec_dict()?;
    match cmd {
        WParseCLI::Daemon(args) => {
            let work_root = instance_root.expect("daemon work root resolved");
            let mut engine_args: wp_engine::facade::args::ParseArgs = args.into();
            inject_state_dir(Path::new(&work_root), &env_dict, &mut engine_args)?;

            let mut app = WpApp::try_from(engine_args, env_dict.clone())?;
            let admin_api = warp_parse::admin_api::start_if_enabled(
                Path::new(&work_root),
                &env_dict,
                app.control_handle(),
            )
            .await?;
            log_build_info_once();
            let run_result = app.run_daemon().await;
            if let Some(admin_api) = admin_api {
                admin_api.shutdown().await;
            }
            run_result?;
        }
        WParseCLI::Batch(args) => {
            let work_root = instance_root.expect("batch work root resolved");
            let mut engine_args: wp_engine::facade::args::ParseArgs = args.into();
            inject_state_dir(Path::new(&work_root), &env_dict, &mut engine_args)?;
            let mut app = WpApp::try_from(engine_args, env_dict)?;
            log_build_info_once();
            app.run_batch().await?;
        }
        WParseCLI::Version(args) => {
            let ver = warp_parse::build::PKG_VERSION;
            if let Some(target) = args.ge {
                let current = semver::Version::parse(ver).map_err(|e| {
                    RunReason::from_conf()
                        .to_err()
                        .with_detail(format!("invalid current version '{}': {}", ver, e))
                })?;
                let target = semver::Version::parse(&target).map_err(|e| {
                    RunReason::from_conf()
                        .to_err()
                        .with_detail(format!("invalid target version '{}': {}", target, e))
                })?;
                if current >= target {
                    println!("{} (>= {})", ver, target);
                } else {
                    eprintln!("{} (< {})", ver, target);
                    std::process::exit(1);
                }
            } else {
                println!("{}", warp_parse::build::CLAP_LONG_VERSION);
                println!("features: {}", warp_parse::feats::features_list());
            }
        }
    }
    Ok(())
}
