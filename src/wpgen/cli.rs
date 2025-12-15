use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "wpgen",
    version,
    about = "WarpParse generator (shim)/WarpParse 数据生成器（兼容壳）"
)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Generate data by rule/基于规则生成数据
    Rule {
        /// Work root directory (contains conf/ etc.)/工作根目录（包含 conf/ 等）
        #[arg(short, long, default_value = ".")]
        work_root: String,
        /// Override WPL rule directory (default derives from main.conf.rule_root)/覆盖 WPL 规则目录（默认从 main.conf 的 rule_root 推导）
        #[arg(long = "wpl")]
        wpl_dir: Option<String>,
        /// Config file name (default: wpgen.toml)/配置文件名（默认：wpgen.toml）
        #[arg(short, long, default_value = "wpgen.toml")]
        conf_name: String,
        /// Print stats periodically/周期性打印统计信息
        #[arg(short = 'p', long = "print_stat", default_value_t = false)]
        stat_print: bool,
        /// Total line count override/总行数覆盖
        #[arg(short = 'n')]
        line_cnt: Option<usize>,
        /// Generation speed override/生成速度覆盖
        #[arg(short = 's')]
        gen_speed: Option<usize>,
        /// Stats interval seconds/统计输出间隔（秒）
        #[arg(long = "stat", default_value_t = 1)]
        stat_sec: usize,
    },
    /// Generate data from sample files/基于样本文件生成数据
    Sample {
        /// Work root directory/工作根目录
        #[arg(short, long, default_value = ".")]
        work_root: String,
        /// Override WPL rule directory (default derives from main.conf.rule_root)/覆盖 WPL 规则目录（默认从 main.conf 的 rule_root 推导）
        #[arg(long = "wpl")]
        wpl_dir: Option<String>,
        /// Config file name/配置文件名
        #[arg(short, long, default_value = "wpgen.toml")]
        conf_name: String,
        /// Print stats periodically/周期性打印统计信息
        #[arg(short = 'p', long = "print_stat", default_value_t = false)]
        print_stat: bool,
        /// Total line count override/总行数覆盖
        #[arg(short = 'n')]
        line_cnt: Option<usize>,
        /// Generation speed override/生成速度覆盖
        #[arg(short = 's')]
        gen_speed: Option<usize>,
        /// Stats interval seconds/统计输出间隔（秒）
        #[arg(long = "stat", default_value_t = 1)]
        stat_sec: usize,
    },
    /// Configuration commands/配置相关命令
    Conf {
        #[command(subcommand)]
        sub: ConfCmd,
    },
    /// Data management commands/数据管理相关命令
    Data {
        #[command(subcommand)]
        sub: DataCmd,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfCmd {
    /// Initialize generator config (conf/wpgen.toml)/初始化生成器配置（conf/wpgen.toml）
    Init {
        /// Work root directory/工作根目录
        #[arg(short, long, default_value = ".")]
        work_root: String,
    },
    /// Clean generator config/清理生成器配置
    Clean {
        /// Work root directory/工作根目录
        #[arg(short, long, default_value = ".")]
        work_root: String,
    },
    /// Check generator config/检查生成器配置
    Check {
        /// Work root directory/工作根目录
        #[arg(short, long, default_value = ".")]
        work_root: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum DataCmd {
    /// Clean generated output data according to wpgen config/根据 wpgen 配置清理已生成输出数据
    Clean {
        /// Work root directory/工作根目录
        #[arg(short, long, default_value = ".")]
        work_root: String,
        /// Config file name (default: wpgen.toml). Example: -c wpgen1.toml/配置文件名（默认：wpgen.toml），例如：-c wpgen1.toml
        #[arg(short = 'c', long = "conf", default_value = "wpgen.toml")]
        conf_name: String,
        /// Clean local outputs only (no remote); default true/仅清理本地输出（不触达远端）；默认 true
        #[arg(long, default_value_t = true)]
        local: bool,
    },
    /// Not supported; reserved for future/暂不支持；保留供未来使用
    Check {
        /// Work root directory/工作根目录
        #[arg(short, long, default_value = ".")]
        work_root: String,
    },
}
