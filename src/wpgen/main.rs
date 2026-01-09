// Split main into small modules to improve readability and navigation.
// - cli.rs: clap 定义（Cli/Cmd/ConfCmd/DataCmd）
// - rule.rs / sample.rs: 生成逻辑
// - conf.rs / data.rs: 配置与数据管理逻辑

use anyhow::Result;
use clap::Parser;
use warp_parse::load_sec_dict; // bring Parser trait for Cli::parse()

mod cli;
mod conf;
mod data;
mod rule;
mod sample;
//mod wpcli;

use crate::cli::{Cli, Cmd, ConfCmd, DataCmd};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    // 注册可用的 sink 工厂（内置）
    warp_parse::feats::register_for_runtime();
    let cli = Cli::parse();
    let env_dict = load_sec_dict()?;
    match cli.cmd {
        Cmd::Rule {
            work_root,
            wpl_dir,
            conf_name,
            stat_print,
            line_cnt,
            speed,
            stat_sec,
        } => {
            rule::run(
                &work_root,
                wpl_dir.as_deref(),
                &conf_name,
                stat_print,
                line_cnt,
                speed,
                stat_sec,
                &env_dict,
            )
            .await?
        }
        Cmd::Sample {
            work_root,
            wpl_dir,
            conf_name,
            print_stat,
            line_cnt,
            speed,
            stat_sec,
        } => {
            sample::run(
                &work_root,
                wpl_dir.as_deref(),
                &conf_name,
                print_stat,
                line_cnt,
                speed,
                stat_sec,
                &env_dict,
            )
            .await?
        }
        Cmd::Conf { sub } => match sub {
            ConfCmd::Init { work_root } => conf::init(&work_root).await?,
            ConfCmd::Clean { work_root } => conf::clean(&work_root).await?,
            ConfCmd::Check { work_root } => conf::check(&work_root, &env_dict).await?,
        },
        Cmd::Data { sub } => match sub {
            DataCmd::Clean {
                work_root,
                conf_name,
                local,
            } => data::clean(&work_root, &conf_name, local, &env_dict).await?,
            DataCmd::Check { work_root: _ } => {
                println!("wpgen: 'data check' is not supported in this CLI");
            }
        },
    }
    Ok(())
}
