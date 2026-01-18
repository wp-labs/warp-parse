use clap::{Args, Parser, Subcommand};
use wp_proj::consts::{DEFAULT_ANALYSE_LINE_MAX, DEFAULT_ANALYSE_MODE, DEFAULT_WORK_ROOT};

use warp_parse::build::CLAP_LONG_VERSION;
// sinks helpers are available via facade::config
// use wp_engine::sinks::{DebugViewer, ViewOuter}; // no longer used directly
// use wp_conf::conf::sink::{SinkUseConf, SinksEnum}; // no longer used directly

//use crate::model::*;

// defaults moved to crate::consts

#[derive(Subcommand)]
pub enum WProj {
    /// 规则工具：解析规则的管理和调试 | Rule tools: management and debugging of parsing rules
    ///
    /// 提供解析规则（WPL）的验证、分析和调试功能，包括：
    /// • verify：验证规则语法和逻辑
    /// • analyse：分析规则模式和性能
    /// • parse：执行离线解析测试
    #[command(subcommand, name = "rule")]
    Rule(RuleCmd),

    /// 一键初始化完整工程骨架 | Initialize complete project skeleton
    ///
    /// 创建 Warp Flow Engine 项目的完整目录结构和配置文件，包括配置目录、
    /// 连接器配置、模型目录（WPL/OML/知识库）和默认项目配置
    #[command(name = "init", visible_alias = "初始化")]
    Init(ProjectInitArgs),

    /// 批量检查项目配置和文件完整性 | Batch check project configuration and file integrity
    ///
    /// 全面检查项目的各个方面，包括配置文件语法和逻辑验证、连接器配置
    /// 完整性检查、模型文件存在性和格式验证、依赖关系和路径正确性检查
    #[command(name = "check", visible_alias = "检查")]
    Check(ProjectCheckArgs),

    /// 数据管理工具：清理、统计、验证 | Data management tools: cleanup, statistics, validation
    ///
    /// 管理项目生成和处理的数据，包括数据清理、统计、数据源检查和数据验证
    #[command(subcommand, name = "data")]
    Data(DataCmd),

    /// 模型管理工具：规则、源、汇、知识库 | Model management tools: rules, sources, sinks, knowledge base
    ///
    /// 管理和监控项目中的各种模型组件，包括输入源、输出汇、数据流路径和知识库
    #[command(subcommand, name = "model")]
    Model(ModelCmd),
}

#[derive(Parser)]
#[command(
    name = "wproj",
    about = "Warp Flow Engine 项目管理工具\n\nwproj 是 Warp Flow Engine 的官方命令行工具，提供完整的项目生命周期管理功能，包括：
• 项目初始化和配置管理
• 数据源的检查、统计和验证
• 模型（规则/源/汇）的管理和监控
• 知识库（KnowDB）的创建和维护

Warp Flow Engine Project Management Tool

wproj is the official CLI tool for Warp Flow Engine, providing comprehensive project lifecycle management:
• Project initialization and configuration management
• Data source checking, statistics, and validation
• Model (rules/sources/sinks) management and monitoring
• Knowledge base (KnowDB) creation and maintenance",
    version = CLAP_LONG_VERSION,
    author = "Warp Flow Engine Team"
)]
pub struct WProjCli {
    /// 安静模式，减少输出信息 | Quiet mode with reduced output
    #[clap(
        short = 'q',
        long,
        action,
        help = "安静模式，减少输出信息 | Quiet mode with reduced output"
    )]
    // 说明：-q/--quiet 在 apps/wproj/main.rs 中于 clap 解析前被提前消费
    //（通过 wp_cli_core::split_quiet_args 过滤），此处保留仅用于 help 展示与向后兼容。
    pub quiet: bool,
    #[command(subcommand)]
    pub cmd: WProj,
}

#[derive(Subcommand, Debug)]
#[command(
    name = "knowdb",
    about = "知识库管理工具（V2）| Knowledge base management tools (V2)"
)]
pub enum KnowdbCmd {
    /// 生成目录式 KnowDB 骨架 | Generate directory-based KnowDB skeleton
    #[command(
        name = "init",
        visible_alias = "初始化",
        about = "生成目录式 KnowDB 骨架 | Generate directory-based KnowDB skeleton"
    )]
    Init(KnowdbInitArgs),

    /// 校验 KnowDB 目录结构与必要文件 | Validate KnowDB directory structure and required files
    #[command(
        name = "check",
        visible_alias = "检查",
        about = "校验 KnowDB 目录结构与必要文件 | Validate KnowDB directory structure and required files"
    )]
    Check(KnowdbCheckArgs),

    /// 清理 KnowDB 目录与缓存文件 | Clean up KnowDB directories and cache files
    #[command(
        name = "clean",
        visible_alias = "清理",
        about = "清理 KnowDB 目录与缓存文件 | Clean up KnowDB directories and cache files"
    )]
    Clean(KnowdbCleanArgs),
}

#[derive(Subcommand, Debug)]
#[command(
    name = "model",
    about = "模型组件管理和监控工具 | Model component management and monitoring tools"
)]
pub enum ModelCmd {
    /// 列出并检查源连接器 | List and check source connectors
    #[command(
        name = "sources",
        about = "列出并检查源连接器 | List and check source connectors"
    )]
    Sources(SourcesCommonArgs),

    /// 列出汇组和路由配置 | List sink groups and route configurations
    #[command(
        name = "sinks",
        about = "列出汇组和路由配置 | List sink groups and route configurations"
    )]
    Sinks(SinksCommonArgs),

    /// 显示数据流路径：规则→OML→汇 | Display data flow paths: rules→OML→sinks
    #[command(
        name = "route",
        about = "显示数据流路径：规则→OML→汇 | Display data flow paths: rules→OML→sinks"
    )]
    Route(SinksRouteArgs),

    /// 知识库管理工具（V2）| Knowledge base management tools (V2)
    #[command(subcommand, name = "knowdb")]
    Knowdb(KnowdbCmd),
}

#[derive(Args, Debug, Clone, Default)]
pub struct KnowdbInitArgs {
    /// 工作目录（包含 conf 与 models）| Work directory (contains conf and models)
    #[clap(
        short,
        long,
        default_value = ".",
        visible_alias = "工作目录",
        help = "工作目录（包含 conf 与 models）| Work directory (contains conf and models)"
    )]
    pub work_root: String,

    /// 生成完整模板（包含示例数据和SQL）| Generate complete templates (with sample data and SQL)
    #[clap(
        long = "full",
        default_value_t = false,
        visible_alias = "完整",
        help = "生成完整模板（包含示例数据和SQL）| Generate complete templates (with sample data and SQL)"
    )]
    pub full: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub struct KnowdbCheckArgs {
    /// 工作目录（包含 conf 与 models）
    #[clap(short, long, default_value = ".", visible_alias = "工作目录")]
    pub work_root: String,
}

#[derive(Args, Debug, Clone, Default)]
pub struct KnowdbCleanArgs {
    /// 工作目录（包含 conf 与 models）
    #[clap(short, long, default_value = ".", visible_alias = "工作目录")]
    pub work_root: String,
}

#[derive(Subcommand, Debug)]
#[command(
    name = "rule",
    about = "解析规则（WPL）管理工具 | Parsing rules (WPL) management tools"
)]
pub enum RuleCmd {
    /// 使用规则执行离线解析测试 | Execute offline parsing tests with rules
    #[command(
        name = "parse",
        visible_alias = "解析",
        about = "使用规则执行离线解析测试 | Execute offline parsing tests with rules"
    )]
    Parse(ParseArgs),
}

#[derive(Subcommand, Debug)]
#[command(name = "project")]
pub enum ProjectCmd {
    /// 一键初始化完整工程骨架 | Init full project skeleton
    #[command(name = "init", visible_alias = "初始化")]
    Init(ProjectInitArgs),
    /// 环境体检 | Environment doctor
    #[command(name = "doctor", visible_alias = "体检")]
    Doctor,
    /// 批量检查项目（conf/sources/sinks/wpl/oml）| Batch check projects
    #[command(name = "check", visible_alias = "检查")]
    Check(ProjectCheckArgs),
}

#[derive(Args, Debug, Clone, Default)]
pub struct ProjectCheckArgs {
    /// 根目录 | Root path (contains multiple projects)
    #[clap(short, long, default_value = DEFAULT_WORK_ROOT, visible_alias = "工作目录")]
    pub work_root: String,
    /// 检查项：conf,connectors,sources,sinks,wpl,oml,all | What to check
    #[clap(long = "what", default_value = "all", visible_alias = "检查项")]
    pub what: String,
    /// 强制日志输出到控制台 | Log to console
    #[clap(long, default_value_t = false, visible_alias = "控制台日志")]
    pub console: bool,
    /// 命中第一处失败立即退出 | Fail fast
    #[clap(long, default_value_t = false, visible_alias = "快速失败")]
    pub fail_fast: bool,
    /// JSON 输出 | JSON output
    #[clap(long = "json", default_value = "false", visible_alias = "输出JSON")]
    pub json: bool,
    /// 仅输出失败项 | Only print failed items
    #[clap(long = "only-fail", default_value_t = false, visible_alias = "仅失败")]
    pub only_fail: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub struct ProjectInitArgs {
    /// 工作目录 | Work directory
    #[clap(
        short,
        long,
        default_value = DEFAULT_WORK_ROOT,
        visible_alias = "工作目录",
        help = "工作目录 | Work directory",
    )]
    pub work_root: String,

    /// 初始化模式：full/model/conf/data | Initialization mode: full/model/conf/data
    #[clap(
        short,
        long = "mode",
        default_value = "normal",
        visible_alias = "模式",
        help = "初始化模式：full/normal/model/conf/data | Initialization mode: full/normal/model/conf/data"
    )]
    pub mode: String,
}

// 旧 Sink 工具组（Kafka/DB/Syslog）已迁移至 wpsink，这里不再暴露。

// 旧 os 子命令已移除。

#[derive(Subcommand, Debug, Clone)]
#[command(name = "stat")]
pub enum StatCmd {
    /// 同时统计源与文件型 sink | Combined (src-file + sink-file)
    #[command(name = "file", visible_alias = "文件")]
    File(StatSinkArgs),
    /// 统计启用文件源的输入行数 | Source files
    #[command(name = "src-file", visible_alias = "源文件")]
    SrcFile(StatSrcArgs),
    /// 统计文件型 sink 的输出行数 | Sink files
    #[command(name = "sink-file", visible_aliases = ["sink文件", "汇文件"])]
    SinkFile(StatSinkArgs),
}

#[derive(Subcommand, Debug, Clone)]
#[command(name = "validate")]
pub enum ValidateCmd {
    /// 按 expect 对文件型 sink 做比例/区间校验 | Validate sink files by expect
    #[command(name = "sink-file", visible_aliases = ["sink文件", "汇文件"])]
    SinkFile(ValidateSinkArgs),
}

#[derive(Subcommand, Debug)]
#[command(
    name = "data",
    about = "数据管理工具：清理、统计、验证 | Data management tools: cleanup, statistics, validation"
)]
pub enum DataCmd {
    /// 清理本地输出数据文件 | Clean local output data files
    #[command(
        name = "clean",
        visible_alias = "清理",
        about = "清理本地输出数据文件 | Clean local output data files"
    )]
    Clean(DataArgs),

    /// 检查数据源连通性和配置 | Check data source connectivity and configuration
    #[command(
        name = "check",
        visible_alias = "检查",
        about = "检查数据源连通性和配置 | Check data source connectivity and configuration"
    )]
    Check(DataArgs),

    /// 统计数据量和处理性能 | Statistics of data volume and processing performance
    #[command(
        name = "stat",
        about = "统计数据量和处理性能 | Statistics of data volume and processing performance"
    )]
    Stat(DataStatArgs),

    /// 验证数据分布和比例 | Validate data distribution and proportions
    #[command(
        name = "validate",
        about = "验证数据分布和比例 | Validate data distribution and proportions"
    )]
    Validate(DataValidateArgs),
}

#[derive(Subcommand, Debug)]
#[command(name = "sources")]
pub enum SourcesCmd {
    /// List source connectors and references (connectors/source.d)
    #[command(name = "list")]
    List(SourcesCommonArgs),
}

#[derive(Args, Debug, Clone)]
pub struct SourcesCommonArgs {
    /// 工作目录 | Work root
    #[clap(short, long, default_value = DEFAULT_WORK_ROOT, visible_alias = "工作目录")]
    pub work_root: String,
    /// JSON 输出 | JSON output
    #[clap(long = "json", default_value = "false", visible_alias = "输出JSON")]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SourcesRouteArgs {
    #[clap(flatten)]
    pub common: CommonFiltArgs,
}

#[derive(Args, Debug, Clone)]
pub struct SinksCommonArgs {
    /// 工作目录 | Work root
    #[clap(short, long, default_value = DEFAULT_WORK_ROOT, visible_alias = "工作目录")]
    pub work_root: String,
    /// JSON 输出 | JSON output
    #[clap(long = "json", default_value = "false", visible_alias = "输出JSON")]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SinksRouteArgs {
    #[clap(flatten)]
    pub common: CommonFiltArgs,
}

#[derive(Args, Debug, Clone, Default)]
pub struct CommonFiltArgs {
    /// 工作目录 | Work root
    #[clap(short, long, default_value = DEFAULT_WORK_ROOT, visible_alias = "工作目录")]
    pub work_root: String,
    /// 过滤组（可重复）| Filter groups (repeatable)
    #[clap(long = "group", visible_alias = "组")]
    pub group_names: Vec<String>,
    /// 过滤 sink（可重复）| Filter sinks (repeatable)
    #[clap(long = "sink", visible_alias = "汇")]
    pub sink_names: Vec<String>,
    /// 路径包含 | Path contains
    #[clap(long = "path-like", visible_alias = "路径包含")]
    pub path_like: Option<String>,
    /// JSON 输出 | JSON output
    #[clap(long = "json", default_value = "false", visible_alias = "输出JSON")]
    pub json: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub struct DataArgs {
    /// 本地模式（仅清理本地文件）| Local mode
    #[clap(long, default_value = "true", visible_alias = "本地模式")]
    pub local: bool,
    /// 工作目录 | Work root
    #[clap(short, long, default_value = DEFAULT_WORK_ROOT, visible_alias = "工作目录")]
    pub work_root: String,
}

#[derive(Args, Debug, Clone, Default)]
pub struct DataStatArgs {
    #[clap(flatten)]
    pub common: CommonFiltArgs,
    /// 子命令：file/src-file/sink-file；缺省使用 file
    #[command(subcommand)]
    pub command: Option<StatCmd>,
}

#[derive(Args, Debug, Clone, Default)]
pub struct DataValidateArgs {
    /// 工作目录 | Work root
    #[clap(short, long, default_value = DEFAULT_WORK_ROOT, visible_alias = "工作目录")]
    pub work_root: String,
    /// 显式指定总输入条数（缺省依赖源统计）| Override total input count
    #[clap(long = "input-cnt", visible_alias = "输入条数")]
    pub input_cnt: Option<u64>,
}

#[derive(Args, Debug, Clone, Default)]
pub struct StatSrcArgs {
    #[clap(flatten)]
    pub common: CommonFiltArgs,
}

#[derive(Args, Debug, Clone, Default)]
pub struct StatSinkArgs {
    #[clap(flatten)]
    pub common: CommonFiltArgs,
}

#[derive(Args, Debug, Clone, Default)]
pub struct ValidateSinkArgs {
    #[clap(flatten)]
    pub common: CommonFiltArgs,
    /// 显式指定总输入条数 | Specify total input count
    #[clap(long = "input-cnt", visible_alias = "输入条数")]
    pub input_cnt: Option<u64>,
    /// 运行期统计 JSON | Stats JSON file
    #[clap(long = "stats-file", visible_alias = "统计文件")]
    pub stats_file: Option<String>,
    /// 显示详细信息 | Verbose
    #[clap(short = 'v', long = "verbose", visible_alias = "详细", action)]
    pub verbose: bool,
}

#[derive(Args, Debug)]
#[command(name = "check")]
pub struct ChkArgs {
    /// 检查项（示例：shm）| Check item
    #[clap(short, long, default_value = "shm", visible_alias = "项目")]
    pub item: String,
    /// 工作目录 | Work root
    #[clap(short, long, default_value = ".", visible_alias = "工作目录")]
    pub work_root: String,
}

#[derive(Args, Debug, Clone)]
#[command(name = "parse")]
pub struct ParseArgs {
    /// 输入文件路径 | Input file path
    #[clap(short, long, visible_alias = "输入路径")]
    pub in_path: Option<String>,
    #[clap(short = 'R', long, visible_alias = "规则文件")]
    pub rule_file: Option<String>,
    /// JSON 输出 | JSON output
    #[clap(long = "json", default_value = "false", visible_alias = "输出JSON")]
    pub json: bool,
    /// 静默模式 | Quiet output
    #[clap(short = 'q', long = "quiet", action, visible_alias = "静默")]
    pub quiet: bool,
}

#[derive(Args, Debug)]
#[command(name = "analyse")]
pub struct AnalyseArgs {
    /// 工作目录 | Work root
    #[clap(short, long, default_value = DEFAULT_WORK_ROOT, visible_alias = "工作目录")]
    pub work_root: String,
    /// 样本文件路径 | Sample file path
    #[clap(short, long, visible_alias = "输入路径")]
    pub in_path: Option<String>,
    /// 输出文件路径 | Output file path
    #[clap(short, long, visible_alias = "输出路径")]
    pub out_path: Option<String>,
    /// 模式（i 交互）| Mode (i interactive)
    #[clap(short, long, default_value = DEFAULT_ANALYSE_MODE, visible_alias = "模式")]
    pub mode: String,
    /// 规则表达式 | Rule expression
    #[clap(short, long, visible_alias = "规则")]
    pub rule: Option<String>,
    /// 最大行数 | Max lines
    #[clap(short = 'n', long, default_value = DEFAULT_ANALYSE_LINE_MAX, visible_alias = "最大行数")]
    pub line_max: Option<usize>,
    /// 规则文件 | Rule file
    #[clap(short = 'R', long, visible_alias = "规则文件")]
    pub rule_file: Option<String>,
    /// 检查强度 | Check level
    #[clap(short = 's', long, default_value = "2", visible_alias = "检查强度")]
    pub check: usize,
    /// JSON 输出 | JSON output
    #[clap(long = "json", default_value = "false", visible_alias = "输出JSON")]
    pub json: bool,
    /// 知识库路径 | Knowledge path
    #[clap(short = 'k', long, visible_alias = "知识库路径")]
    pub knowledge_path: Option<String>,
}
