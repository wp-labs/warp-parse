//! Rescue 数据管理处理器

use std::path::PathBuf;

use crate::args::{RescueCmd, RescueStatArgs};
use wp_cli_core::rescue::scan_rescue_stat;
use wp_error::run_error::RunResult;

/// 分发 rescue 子命令
pub fn dispatch_rescue_cmd(cmd: RescueCmd) -> RunResult<()> {
    match cmd {
        RescueCmd::Stat(args) => run_rescue_stat(args),
    }
}

/// 执行 rescue 统计
fn run_rescue_stat(args: RescueStatArgs) -> RunResult<()> {
    let RescueStatArgs {
        work_root,
        rescue_path,
        detail,
        json,
        csv,
    } = args;

    // 构建完整的 rescue 路径
    let full_path = if PathBuf::from(&rescue_path).is_absolute() {
        rescue_path
    } else {
        PathBuf::from(&work_root)
            .join(&rescue_path)
            .to_string_lossy()
            .to_string()
    };

    // 扫描并统计
    let summary = scan_rescue_stat(&full_path, detail);

    // 根据输出格式打印
    if json {
        summary.print_json();
    } else if csv {
        summary.print_csv(detail);
    } else {
        summary.print_table(detail);
    }

    Ok(())
}
