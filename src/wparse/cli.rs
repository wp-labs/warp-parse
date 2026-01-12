use clap::{Args, Parser};
use wp_error::error_handling::RobustnessMode;

/// Local CLI definition so we can control metadata/version independently.
#[derive(Parser)]
#[command(
    name = "wparse",
    version = env!("CARGO_PKG_VERSION"),
    about = "Warp Parse CLI"
)]
pub enum WParseCLI {
    /// Run engine in daemon mode (alias of `work --run-mode=daemon`)
    #[command(name = "daemon", visible_alias = "deamon")]
    Daemon(CliParseArgs),

    /// Run engine in batch mode (alias of `work --batch`)
    #[command(name = "batch")]
    Batch(CliParseArgs),
}

#[derive(Args, Debug, Default, Clone)]
pub struct CliParseArgs {
    #[clap(long, default_value = None)]
    pub work_root: Option<String>,
    #[clap(short, long, default_value = "p")]
    pub mode: String,
    #[clap(short = 'n', long, default_value = None)]
    pub max_line: Option<usize>,
    #[clap(short = 'w', long = "parse-workers")]
    pub parse_workers: Option<usize>,
    #[clap(short = 'S', long)]
    pub check_stop: Option<usize>,
    #[clap(short = 's', long)]
    pub check_continue: Option<usize>,
    #[clap(long = "stat")]
    pub stat_sec: Option<usize>,
    #[clap(long = "robust")]
    pub robust: Option<RobustnessMode>,
    #[clap(short = 'p', long = "print_stat", default_value = "false")]
    pub stat_print: bool,
    #[clap(long = "log-profile")]
    pub log_profile: Option<String>,
    #[clap(long = "wpl")]
    pub wpl_dir: Option<String>,
}

impl From<CliParseArgs> for wp_engine::facade::args::ParseArgs {
    fn from(value: CliParseArgs) -> Self {
        wp_engine::facade::args::ParseArgs {
            work_root: value.work_root,
            mode: value.mode,
            max_line: value.max_line,
            parse_workers: value.parse_workers,
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
