# Wparse配置规范

本页定义 wparse 引擎的主配置文件结构、默认值、加载与优先级，以及与源/汇/连接器的关系。示例目录可通过 `wproj conf init` 生成并在本地验证（不再自动生成 `models/knowledge`；如需 KnowDB 模板请使用 `wproj knowdb init`）。

位置与文件名
- 主配置：`$WORK_ROOT/conf/wparse.toml`（若不存在，wparse 会落回内置默认；兼容历史名 `dvron.toml`）
- 源配置：`$WORK_ROOT/models/sources/wpsrc.toml`
- 汇配置：`$WORK_ROOT/models/sinks/{business.d,infra.d,defaults.toml}`
- 连接器：`$WORK_ROOT/connectors/{source.d,sink.d}/*.toml`（从工作根向上查找最近的 `connectors/*`，最多 32 层）

目录布局（推荐）
```
conf/
  wparse.toml
models/
  wpl/           # 解析规则（WPL）
  oml/           # 对象模型（OML）
  sources/       # 源（统一 wpsrc.toml）
  sinks/         # 汇（business.d/infra.d/defaults.toml）
connectors/
  source.d/      # 源连接器定义
  sink.d/        # 汇连接器定义
data/
  logs/          # 日志输出目录（由 [log_conf.file].path 指定）
  rescue/        # 急救数据目录（由 [rescue].path 指定）
.run/            # 运行时产物（pid、rule_mapping.dat 等）
```

配置优先级（概览）
- CLI → 覆盖主配置 → 默认值；未在 CLI 指定时，以配置文件为准
- 生效映射：
  - `--parse-workers` 覆盖 `[performance].parse_workers`
  - `--stat <sec>` 覆盖 `[stat].window_sec`（默认 60）
  - `--robust <debug|normal|strict>` 覆盖 `robust`
  - `--print_stat` 仅影响是否在控制台输出统计日志
  - `--max-line/-n`、`--mode` 影响运行参数，不写入配置
  - 说明：`--log-profile` 预留为日志预设，但当前实现不覆盖配置文件中的 `[log_conf]`，请以配置文件为准


完整示例（推荐默认）
```toml
version = "1.0"
robust  = "normal"           # debug|normal|strict

[models]
wpl     = "./models/wpl"
oml     = "./models/oml"
sources = "./models/sources"
sinks   = "./models/sinks"

[performance]
rate_limit_rps = 10000        # 限速（records/second）
parse_workers  = 2            # 解析并发 worker 数

[rescue]
path = "./data/rescue"        # 兜底/残留/错误数据目录

[log_conf]
output = "File"               # Console|File|Both
level  = "warn,ctrl=info,launch=info,source=info,sink=info,stat=info,runtime=warn,oml=warn,wpl=warn,klib=warn,orion_error=error,orion_sens=warn"

[log_conf.file]
path = "./data/logs"          # 文件输出目录；文件名自动取可执行名（wparse.log）

[stat]
window_sec = 60

[[stat.pick]]                 # 采集阶段统计
key    = "pick_stat"
target = "*"
fields = []
top_n  = 20

[[stat.parse]]                # 解析阶段统计
key    = "parse_stat"
target = "*"
fields = []
top_n  = 20

[[stat.sink]]                 # 下游阶段统计
key    = "sink_stat"
target = "*"
fields = []
top_n  = 20
```

字段说明与默认值
- `version`：固定为 `1.0`
- `robust`：错误处理风格，默认 `normal`；`debug/normal/strict` 三档
- `[models]`：各目录的相对路径（相对工作根）；默认如示例
- `[performance].rate_limit_rps`：整型，默认 `10000`
- `[performance].parse_workers`：整型，默认 `2`；可被 `--parse-workers` 覆盖
- `[rescue].path`：兜底/残留/错误数据目录，默认 `./data/rescue`
- `[log_conf]`：日志配置
  - `output`：`Console|File|Both`
  - `level`：字符串预设（逗号分隔的 root 与定向级别）；或使用结构化写法：
    ```toml
    [log_conf]
    output = "File"
    [log_conf.levels]
    global = "warn"
    ctrl   = "info"
    launch = "info"
    ```
  - `[log_conf.file].path`：文件输出目录（不存在会自动创建）
- `[stat]`：统计窗口秒数（默认 60）；分阶段条目置于 `[[stat.pick]]/[[stat.parse]]/[[stat.sink]]`
  - `key`：统计项标识；`target`：`"*"|"ignore"|自定义（如 "sink:demo"）`
  - `fields`：聚合维度数组；`top_n`：输出条目上限（默认 20）

源/汇与连接器（与主配置的关系）
- 源（Sources）：读取路径由 `[models].sources` 决定；统一使用 `models/sources/wpsrc.toml`（兼容 `models/source/` 与 `source/`）。
  - 写法与校验见：[源配置总览](02-sources.md)
  - 连接器查找：从工作根向上查找最近的 `connectors/source.d`（最多 32 层）
- 汇（Sinks）：读取路径由 `[models].sinks` 决定；采用目录式 V2：`business.d/infra.d/defaults.toml`。
  - 写法与路由见：[Sinks 设计与配置](04-sinks_config.md) 与 [最小骨架](03-sinks_minimal.md)
  - 连接器位置：`connectors/sink.d`（与 wpgen 共享）

可选配置
- `models/knowledge/knowdb.toml`：知识库初始化脚本（如建表/导入 CSV），存在则在启动时载入并在 `.run/authority.sqlite` 构建缓存
- 隐私配置：`conf/privacy.toml` 已废弃（引擎不再加载隐私处理器）；如需脱敏，请在上游或自定义插件中处理

校验与排障
- 初始化：`wproj conf init --work-root <WR>`（生成 `conf/`、`connectors/` 以及部分 `models/` 模板，不包含 `models/knowledge`；如需生成 KnowDB 模板请执行 `wproj knowdb init`）
- 校验主/源/汇配置：`wproj conf check -w <WR>`、`wproj sinks validate|list|route -w <WR>`
- 常见错误：
  - 源连接器不存在：检查 `connectors/source.d/*.toml` 与 `[[sources]].connect` 是否一致
  - 汇路由未找到：确保 `models/sinks/{business.d,infra.d}` 至少存在其中之一

实用建议
- 日志预设：如需快速切换日志详细度，优先修改 `[log_conf]`；`--log-profile` 当前为预留选项
- 性能：批量压测先调大 `parse_workers` 与源/汇并行参数，再观察 `stat` 输出
- 产出与排障：异常/残留数据会落在 `[rescue].path`；日志文件名按可执行名自动命名（例如 `wparse.log`）
