# wproj check 配置检查项

`wproj check` 共 8 项检查，按依赖顺序执行。前 4 项必须通过，后 4 项允许目标文件缺失（缺失视为通过）。

## 检查项总览

| # | 检查组件 | 配置文件 | 必须存在 | 允许缺失 |
|---|---------|---------|---------|---------|
| 1 | Engine | `conf/wparse.toml` | 是 | 否 |
| 2 | Sources | `conf/wpsrc.toml` | 是 | 否 |
| 3 | Connectors | `connectors/source.d/*.toml`, `connectors/sink.d/*.toml` | 是 | 否 |
| 4 | Sinks | `usecase/*/sink/**/*.toml`, `models/sinks/**/*.toml` | 是 | 否 |
| 5 | WPL | `{rule_root}/*.wpl` | 否 | 是（Miss = 通过） |
| 6 | OML | `{oml_root}/**/model.oml` | 否 | 是（Miss = 通过） |
| 7 | SemanticDict | `{knowledge_root}/semantic_dict.toml` | 否 | 是（使用内置词典） |
| 8 | Wpgen | `conf/wpgen.toml` | 否 | 是（optional） |

---

## 1. Engine — `conf/wparse.toml`

| 检查项 | 说明 | 级别 |
|--------|------|------|
| 文件存在 | 读取 `{work_root}/conf/wparse.toml` | Error |
| TOML 解析 | 反序列化为 `EngineConfig`，`#[serde(deny_unknown_fields)]` 拒绝未知字段 | Error |
| 环境变量展开 | 所有支持 `${VAR}` 的字段通过 EnvDict 展开 | Error |
| 路径绝对化 | models.wpl、models.oml、topology.sources/sinks、rescue.path、admin_api TLS 等相对路径转绝对路径 | Error |
| 工作目录切换 | 进程工作目录切换到 work_root | Error |

## 2. Sources — `conf/wpsrc.toml`

| 检查项 | 说明 | 级别 |
|--------|------|------|
| 文件存在 | 检查 `{work_root}/conf/wpsrc.toml` | Error |
| TOML 解析 + env eval | `WarpSources::env_load_toml()` 加载并展开环境变量 | Error |
| 结构校验 | 序列化回 TOML 后调用 `parse_and_validate_only()` 验证结构正确性 | Error |
| Source 实例构建 | `load_source_instances_from_file()` 验证所有 source 定义可解析为有效实例 | Error |
| **File 路径存在性** | `base` 目录存在 + `file` 路径/glob 有匹配文件 | Warning |
| **Syslog/TCP 端口范围** ★P0 | `port` ∈ [1, 65535] | Error |
| **Syslog/TCP 协议校验** ★P0 | `protocol` ∈ {tcp, udp} | Error |

## 3. Connectors — `connectors/source.d/`, `connectors/sink.d/`

| 检查项 | 说明 | 级别 |
|--------|------|------|
| 目录解析 | 从 Engine config 获取目录，回退到 `{work_root}/connectors/source.d`、`sink.d` | Error |
| TOML 解析 | 逐个加载目录下 `*.toml` 文件为 `ConnectorDef`，含 env eval | Error |
| ID 字符校验 | ID 只能包含 `[a-z0-9_]` | Error |
| Source ID 后缀 | Source connector ID 必须以 `_src` 结尾 | Error |
| Sink ID 后缀 | 非 file 类型的 sink connector ID 必须以 `_sink` 结尾 | Error |
| File 前缀建议 | file source/sink ID 建议以 `file_` 开头 | Warning |
| Kind 一致性 | 文件名 kind 提示与声明 type 匹配 | Warning |
| **Warning 输出** ★P2 | Warning 级别的 lint 问题现在会输出到控制台（之前被静默丢弃） | Warning |
| **死连接器检测** ★P2 | 检测 source connector 的 `refs == 0`（定义但未被引用）和 sink connector 未被任何路由使用 | Warning |

## 4. Sinks — `usecase/*/sink/`, `models/sinks/`

| 检查项 | 说明 | 级别 |
|--------|------|------|
| 路由发现 | 扫描 `usecase/*/{case}/sink/{business.d,infra.d}` 和 `models/sinks/{business.d,infra.d}` | Error |
| TOML 解析 + env eval | 每个路由文件 `env_load_toml` + `env_eval` | Error |
| Connector 解析 | 为每个路由目录加载 sink connectors | Error |
| 默认值合并 | 加载并合并 default sink config | Error |
| 路由构建 | `build_route_conf_from()` 解析路由配置 | Error |
| FlexGroup OML/RULE 互斥 | 同一 FlexGroup 中不能同时使用 OML 和 RULE | Error |
| Rule pattern 校验 | 空 pattern 拒绝；以 `/` 开头的 pattern 需额外校验 | Error |
| **File sink 输出路径** ★P1 | file/test_rescue 类型 sink 的 `base` 目录和输出父目录存在性 | Error |

## 5. WPL — `{rule_root}/*.wpl`

| 检查项 | 说明 | 级别 |
|--------|------|------|
| 文件发现 | `find_conf_files()` 找 `rule.wpl`，回退 `glob("*.wpl")` | — |
| 文件缺失 | 无 WPL 文件 → `CheckStatus::Miss`，视为通过 | — |
| **文件读取错误** ★P2 | 文件读取失败（如 UTF-8 编码错误）现在报告具体错误，不再静默降级为空内容 | Error |
| 非空检查 | 每个文件不能为空 | Error |
| 语法解析 | `WplCode::build()` 解析语法 | Error |
| 包结构解析 | `code.parse_pkg()` 验证 package 结构 | Error |
| **重复包名检测** ★P2 | 多个 WPL 文件定义同名 package 时输出 Warning | Warning |
| **重复规则名检测** ★P2 | 同一规则名（`包名::规则名`）在多个文件中重复定义时输出 Warning | Warning |
| **空包检测** ★P2 | package 中没有任何 rule 时输出 Warning | Warning |
| **空规则体检测** ★P3 | rule 中没有任何 field 时输出 Warning | Warning |

## 6. OML — `{oml_root}/**/model.oml`

| 检查项 | 说明 | 级别 |
|--------|------|------|
| 目录存在 | 检查 `{oml_root}/` 目录是否存在 | — |
| 文件发现 | `find_conf_files()` 找 `model.oml` | — |
| 目录/文件缺失 | 无 OML 目录或无 `.oml` 文件 → "OML 文件缺失"，视为通过 | — |
| 非空检查 | `ErrorHandler::check_file_not_empty()` | Error |
| 结构解析 | `fetch_oml_data()` 解析所有 OML 文件 | Error |
| **模型名唯一性** ★P2 | 检测多个 OML 文件中重复的 `name : <model-name>` | Warning |
| **空 rule pattern** ★P3 | 检测空的 `rule :` 声明 | Warning |

## 7. SemanticDict — `{knowledge_root}/semantic_dict.toml`

| 检查项 | 说明 | 级别 |
|--------|------|------|
| Engine config 加载 | 先加载 Engine config 获取 `knowledge_root()` 路径 | Error |
| 主路径探测 | 检查 `{knowledge_root}/semantic_dict.toml` | — |
| 回退路径探测 | 主路径不存在则检查 `{work_root}/knowledge/semantic_dict.toml` | — |
| 文件缺失 | 两个路径都不存在 → "使用内置词典"，视为通过 | — |
| 启用状态 | `conf.enabled == false` → 视为未配置，通过 | — |
| TOML 解析 | `load_semantic_dict()` 反序列化 | Error |
| 模式报告 | 报告 `ADD`（扩展内置词典）或 `REPLACE`（替换内置词典） | Info |
| 词汇统计 | 统计 stop_words、domain_words、status_words、action_verbs、entity_nouns 总数 | Info |
| **空字符串检测** ★P1 | 检测各词汇列表中的空字符串词条，报告数量 | Warning |
| **重复词检测** ★P1 | 检测同类别内的重复词汇，报告重复次数 | Warning |
| **空类别检测** ★P1 | 检测 `chinese = []` / `english = []` 等空列表类别 | Warning |

## 8. Wpgen — `conf/wpgen.toml`

| 检查项 | 说明 | 级别 |
|--------|------|------|
| 文件存在 | 检查 `{work_root}/conf/wpgen.toml` | — |
| 文件缺失 | 不存在 → `"wpgen.toml not found (optional)"`，视为通过 | — |
| TOML 解析 + env eval | `WpGenConfig::load_from_path()` 调用 `env_load_toml` | Error |
| 未知字段拒绝 | `WpGenConfig`、`GeneratorConfig`、`OutputConfig` 均标注 `#[serde(deny_unknown_fields)]` | Error |
| **version 非空** ★P0 | `version` 字段不能为空字符串 | Error |
| **generator.count > 0** ★P0 | 若配置 `count`，必须 > 0（=0 报错） | Error |
| **generator.speed = 0** ★P0 | `0` 表示 unlimited，合法值，不报错 | Info |
| **generator.parallel > 0** ★P0 | `parallel` 必须 > 0 | Error |
| **speed_profile 参数** ★P0 | 各变体的 rate/base/amplitude > 0、period_secs/duration_secs > 0、burst_probability/variance ∈ [0,1]、steps 非空 | Error |
| **logging.level** ★P0 | 每一段（逗号分隔）的级别部分必须是 trace/debug/info/warn/error，支持 `module=level` 对格式（如 `info,ctrl=info`） | Error |
| **logging.output** ★P0 | 必须是已知类型：stdout/console/file/both | Error |
| **rule_root 路径** ★P0 | `rule_root` 指定的目录必须存在（相对于 work_root 解析） | Error |
| **sample_pattern glob** ★P0 | `sample_pattern` 必须是合法的 glob pattern | Error |
| **output.connect 引用** ★P0 | `output.connect` 引用的 sink connector 必须存在于 `connectors/sink.d/` | Error |
| **logging.file_path 父目录** | `logging.file_path` 的父目录存在性（运行时自动创建，仅警告） | Warning |

## Fail-Fast 机制

当 `--fail-fast` 开启时，任意组件检查失败立即中止该 target 的后续检查。检查顺序为 1→8，Engine 失败则后续全部跳过。

---

## P4 高风险项记录（规划中，暂不实现）

以下项目因技术风险较高，记录在此供后续参考：

| # | 项目 | 风险说明 |
|---|------|---------|
| 1 | **WPL 字段类型语义校验** | `WplField::validate()` 使用 `panic!` 而非返回错误，在 check 路径调用可能导致进程崩溃 |
| 2 | **WPL Sample 数据校验** | 需要运行时引擎（`WplEngine`）加载管道上下文，涉及 tokio 异步运行时和完整的 pipeline 初始化 |
| 3 | **OML 结构解析（oml_parse_raw）** | 核心解析函数 `oml_parse_raw` 是 `async` 的，check 路径是同步的，需要引入 tokio runtime 或重构解析层 |
| 4 | **WPL-OML 规则交叉引用** | OML 模型的 `rule : /path/*` 与 WPL 规则路径的匹配发生在运行时（`SinkDispatcher::get_match_oml`），check 时需要同时加载两套完整的规则体系并执行匹配逻辑 |
| 5 | **Connectors 反向交叉引用** | 需要修改 `Connectors::check()` 签名以接受 Engine config，并在 check 路径中加载 source 配置和 sink 路由配置来验证 connector 引用有效性。涉及跨组件依赖注入 |
| 6 | **Sink OML pattern 存在性验证** | Sink 路由中的 `oml = ["pattern"]` 需要匹配已加载的 OML 模型名，当前匹配逻辑在运行时 dispatcher 中，迁移到 check 需要加载完整的 OML 模型和路由配置 |

### 改进优先级建议

- **P2（建议下阶段实现）**: #5 Connectors 反向交叉引用 — 风险可控，build 层已有解析逻辑，只需整合到 check 流程
- **P3（需设计评审）**: #3 OML 结构解析 — 可考虑将 `oml_parse_raw` 改为同步或提供同步包装函数
- **P4（需架构变更）**: #1 #2 #4 #6 — 需要在 check 路径中引入运行时组件或重构核心解析 API
