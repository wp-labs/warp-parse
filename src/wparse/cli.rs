use clap::{Args, Parser};
use warp_parse::build::CLAP_LONG_VERSION;
use wp_error::error_handling::RobustnessMode;

/// Local CLI definition so we can control metadata/version independently.
#[derive(Parser)]
#[command(
    name = "wparse",
    version = CLAP_LONG_VERSION,
    about = "Warp Parse CLI"
)]
pub enum WParseCLI {
    /// Run engine in daemon mode (alias of `work --run-mode=daemon`)
    #[command(name = "daemon", visible_alias = "deamon")]
    Daemon(CliParseArgs),

    /// Run engine in batch mode (alias of `work --batch`)
    #[command(name = "batch")]
    Batch(CliParseArgs),

    /// Offline config check: no sockets, locks or writes; exit non-zero on config errors
    #[command(name = "check")]
    Check(CheckArgs),

    /// Print version or check version constraint
    #[command(name = "version")]
    Version(VersionArgs),
}

#[derive(Args, Debug, Default, Clone)]
pub struct VersionArgs {
    /// Exit 0 if current version >= VERSION, exit 1 otherwise
    #[clap(long = "ge", value_name = "VERSION")]
    pub ge: Option<String>,
}

#[derive(Args, Debug, Default, Clone)]
pub struct CheckArgs {
    /// Work root directory (absolute path); omit to use the current dir.
    /// Loads `<dir>/conf/wparse.toml` and `<dir>/topology/sinks/**` like the daemon.
    #[clap(long = "work-root", default_value = None)]
    pub work_root: Option<String>,
    /// Emit a machine-readable JSON report on stdout
    #[clap(long = "json", default_value_t = false)]
    pub json: bool,
    /// Extra env file loaded before the secret dict (existing env wins)
    #[clap(long = "env-file", value_name = "FILE")]
    pub env_file: Option<String>,
    /// Treat warnings as failures as well
    #[clap(long = "strict", default_value_t = false)]
    pub strict: bool,
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
    /// Override the engine state directory (pid/lock/rule mapping/authority DB);
    /// takes precedence over `[runtime].state_dir`
    #[clap(long = "state-dir")]
    pub state_dir: Option<String>,
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
            state_dir: value.state_dir,
            deny_duplicate_oml: None,
        }
    }
}
