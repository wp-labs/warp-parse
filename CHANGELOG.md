# Changelog

[English](./CHANGELOG.en.md) | 中文

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.19.4 Unreleased]

### Changed
- **wp-connectors**: 连接器依赖从 `v0.9.1` 升级到 `v0.9.2`，同步上游 Postgres sink 支持及其共享数据库依赖调整。

## [0.19.3] - 2026-03-10

### Changed
- **wp-connectors**: 连接器依赖从 `v0.9.0` 升级到 `v0.9.1`，同步上游 HTTP sink 实现以及 ClickHouse 从 `host` 到 `endpoint` 的配置模型调整。

## [0.19.2] - 2026-03-08

### Added
- **Self Check CLI**: 新增 `wproj self check` 命令，用于按 channel 检查远端更新元数据（仅检查，不安装）。
- **Release Automation**: 在发布流程新增 `update-wp-install-manifest` 任务，发布成功后自动更新 `wp-install` 仓库的 `updates/<channel>/manifest.json` 与 `versions/<tag>.json`。
- **wproj self**: 新增 `--channel`、`--updates-base-url`、`--updates-root`、`--json` 参数，支持远端与本地更新清单源切换。

### Changed
- **wp-motor**: 核心引擎依赖从 `v1.17.8` 升级到 `v1.18.0`。
- **wp-connectors**: 连接器依赖从 `v0.7.10-beta` 升级到 `v0.9.0`。
- **Dependencies**: 核心依赖升级到新主线（`orion-error 0.6`、`wp-connector-api 0.8`、`wp-error 0.8`、`wp-log 0.2` 等）。
- **Runtime Connectors**: 为规避升级期间 API 不兼容，社区外部连接器注册调整为暂时跳过并输出告警日志。

### Fixed
- **Error Handling**: 适配 `orion-error 0.6` 的 `UvsFrom`/`from_*` 新接口，统一错误上下文附加方式。
- **Build**: 修复依赖升级后的编译失败问题，恢复 `cargo check --all-targets` 通过。
- **Self Update Validation**: 对 `sha256` 校验改为严格校验（必须为 64 位十六进制字符），并将支持目标限制为 `aarch64-apple-darwin`、`aarch64-unknown-linux-gnu` 和 `x86_64-unknown-linux-gnu`。
- **wproj self Safety**: 增加 channel/路径一致性、目标资产存在性以及版本与制品文件名一致性检查，减少误判更新的情况。

## [0.18.4] - 2026-03-04

### Changed
- 升级 `wp-motor` 核心引擎从 v1.17.5 到 v1.17.6
- `wp-motor` v1.17.6 主要增强观测与统计链路（背压指标、聚合语义修正、热路径优化），并修复 parser 退出与 recovery failover 稳定性问题

## [0.18.3] - 2026-02-27

### Changed
- 升级 `wp-motor` 核心引擎从 v1.17.4-alpha 到 v1.17.5-alpha
- 升级 `wp-connectors` 从 v0.7.7-beta 到 v0.7.8-beta
- 更新项目依赖到最新版本

## [0.18.2] - 2026-02-20

### Changed
- 升级 `wp-motor` 核心引擎从 v1.17.0-alpha 到 v1.17.4-alpha，主要变化包括：
  - **Sinks/Buffer**：新增 sink 级别批量缓冲区，支持可配置 `batch_size` 参数；小包进入待发缓冲区定期刷新，大包自动旁路直接发送（零拷贝）
  - **Sinks/Config**：新增 `batch_timeout_ms` 配置项（默认 300ms），控制缓冲区定期刷新间隔
  - **Sinks/File**：移除 `BufWriter` 和 `proc_cnt` 定期刷新，改为直接写入 `tokio::fs::File`；上游批量组装使用户空间缓冲冗余
- 升级 `wp-connectors` 从 v0.7.6-beta 到 v0.7.7-beta，主要变化包括：
  - **Doris**：使用新协议
  - 更新 `reqwest` 从 0.12 到 0.13
  - 更新 `env_logger` 从 0.10 到 0.11

## [0.18.1] - 2026-02-13

### Changed
- 升级 `wp-motor` 核心引擎从 v1.17.0-alpha 到 v1.17.2-alpha，主要变化包括：
  - **wp-lang**：`kv`/`kvarr` key 解析支持括号类字符 `()`、`<>`、`[]`、`{}`

## [0.18.0] - 2026-02-12

### Changed
- 升级 `wp-motor` 核心引擎从 v1.15.5 到 v1.17.0-alpha，主要变化包括：
  - **OML Match 增强**：新增 OR 条件语法 `cond1 | cond2 | ...`，支持单源和多源匹配，兼容值匹配和函数匹配
  - **OML Match 增强**：多源匹配不再限制源字段数量（之前限制为 2/3/4 个）
  - **OML NLP**：新增 `extract_main_word` 和 `extract_subject_object` 管道函数，用于中文文本分析
  - **OML NLP**：新增可配置 NLP 词典系统，支持通过 `NLP_DICT_CONFIG` 环境变量自定义词典
  - **WPL 新功能**：新增分隔符模式语法 `{…}`，支持通配符（`*`、`?`）、空白匹配器（`\s`、`\h`、`\S`、`\H`）和保留组 `(…)`，用于在单个声明中表达复杂分隔符逻辑
  - **Bug 修复**：修复 kvarr 模式分隔符解析问题

## [0.17.1] - 2026-02-09

### Changed
- 升级 `wp-motor` 核心引擎从 v1.15.1 到 v1.15.5，主要变化包括：
  - **文档**：新增完整的英文 WPL 语法参考文档
  - **性能优化**：OML 批处理性能提升 12-17%
  - **性能优化**：OML 零拷贝优化，多阶段管道性能提升最高 32%
- 更新项目依赖到最新版本

## [0.17.0] - 2026-02-07

### Changed
- 升级 `wp-motor` 核心引擎到 v1.15.1 版本，主要变化包括：
  - **WPL 新增功能**：新增 `not()` 包装函数用于反转管道函数结果
  - **WPL 新增功能**：新增 `not()` 组包装器用于字段解析中的否定断言
  - **OML 新增功能**：引入 `static { ... }` 语法用于模型范围的常量和模板缓存，提升性能
  - **OML 配置**：新增 `enable` 配置选项，支持禁用 OML 模型
  - **Sinks/File**：新增 `sync` 参数控制磁盘刷新策略（高性能模式 vs 数据安全模式）
  - **Sinks/File**：移除 proto binary 格式支持，当前支持格式：json、csv、kv、show、raw、proto-text
  - **Bug 修复**：修复 `sync` 参数未强制数据写入磁盘的问题
  - **Bug 修复**：修复 WPL 管道函数 `f_chars_not_has` 和 `chars_not_has` 的类型检查 bug
- 更新项目依赖到最新版本

## [0.16.1] - 2026-02-05

### Changed
- 升级 `wp-motor` 核心引擎到 v1.14.1-alpha 版本，主要变化包括：
  - **WPL 管道处理器**：新增 `strip/bom` 处理器用于移除 BOM（字节顺序标记）
    - 支持 UTF-8、UTF-16 LE/BE、UTF-32 LE/BE BOM 检测和移除
    - O(1) 快速检测（仅检查前 2-4 字节）
    - 保留输入容器类型（String → String, Bytes → Bytes, ArcBytes → ArcBytes）

## [0.16.0] - 2026-02-04

### Changed
- 升级 `wp-motor` 核心引擎到 v1.14.0 版本，主要变化包括：
  - **WPL 函数增强**：新增 `starts_with` 管道函数，用于高效字符串前缀匹配
  - **OML 管道函数**：新增 `starts_with` 函数用于前缀匹配
  - **OML 管道函数**：新增 `map_to` 函数用于类型感知的条件值分配（支持 string、integer、float、boolean）
  - **OML 匹配表达式**：支持基于函数的模式匹配（`match read(field) { starts_with('prefix') => result }`）
    - 字符串匹配函数：`starts_with`、`ends_with`、`contains`、`regex_match`、`is_empty`、`iequals`
    - 数值比较函数：`gt`、`lt`、`eq`、`in_range`
  - **OML 解析器**：支持 `chars()` 等值构造器中的引号字符串（单引号和双引号）
  - **OML 转换器**：新增临时字段自动过滤功能（以 `__` 开头的字段自动转换为 ignore 类型）
  - **OML 语法简化**：管道表达式中 `pipe` 关键字现在为可选（`take(field) | func` 和 `pipe take(field) | func` 都支持）
  - **修复问题**：修复 OML 匹配表达式中 `in_range` 函数解析失败的问题
  - **修复问题**：修复 `map_to` 解析器中大整数精度丢失的问题
  - **修复问题**：修复 OML 显示输出的往返解析兼容性问题

## [0.15.8] - 2026-02-03

### Changed
- 升级 `wp-motor` 核心引擎到 v1.13.3 版本，主要变化包括：
  - **WPL 解析器**：支持 `\t`（制表符）和 `\S`（非空白字符）分隔符
  - **WPL 解析器**：支持带引号的特殊字符字段名（如 `"field.name"`、`"field-name"`）
  - **WPL 函数增强**：新增 `regex_match` 正则匹配函数
  - **WPL 函数增强**：新增 `digit_range` 数字范围验证函数
  - **WPL 函数增强**：新增 `chars_replace` 字符级字符串替换函数
  - **日志优化**：高频日志路径使用 `log_enabled!` 守卫，消除日志级别过滤时的循环开销
  - **修复问题**：修复 WPL 模式解析器的编译错误
  - **修复问题**：修复数据救援功能的数据丢失问题
  - **修复问题**：移除 Miss Sink 原始数据显示中的 base64 编码，直接显示实际内容
- 更新所有依赖到最新版本。
- **许可证变更**：项目许可证从 Elastic License 2.0 变更为 Apache 2.0。
- **文档改进**：新增 CONTRIBUTING.md 贡献指南，更新 README.md 说明文档。

## [0.15.7] - 2026-01-30

### Changed
- 升级 `wp-motor` 核心引擎到 v1.13.1 版本，主要变化包括：
  - **WPL 解析器增强**：支持 `\t`（制表符）和 `\S`（非空白字符）分隔符
  - **WPL 解析器增强**：支持带引号的特殊字符字段名（如 `"field.name"`、`"field-name"`）
  - **新增函数**：`chars_replace` 字符级字符串替换函数
  - **日志优化**：高频日志路径使用 `log_enabled!` 守卫，消除日志级别过滤时的循环开销
  - **移除功能**：Syslog UDP Source 移除 `SO_REUSEPORT` 多实例支持（安全风险及跨平台不一致）
- 升级 `wp-connectors` 到 v0.7.5-beta 版本。

## [0.15.5] - 2026-01-28

### Changed
- 升级 `wp-motor` 核心引擎到 v1.11.0-alpha 版本。
- 更新项目依赖到最新版本。

## [0.15.4] - 2026-01-27

### Changed
- 更新所有依赖到最新版本，提升稳定性和性能。

## [0.15.3] - 2026-01-23

### Fixed
- 修复 wp-motor 相关问题，提升运行时稳定性。

## [0.15.2] - 2026-01-22

### Changed
- 从 `wp-engine` 迁移到 `wp-motor` v1.10.2-beta 版本：
  - wp-engine 项目已更名为 wp-motor，所有依赖已更新指向新仓库
  - 升级到 v1.10.2-beta 版本，包含最新的运行时特性与性能优化

## [0.15.1] - 2026-01-18

### Added
- 集成 shadow-rs 构建时信息支持 (#100)：
  - 添加 shadow-rs 作为构建依赖，在编译时生成元数据
  - 版本命令现在显示 Git commit、构建时间和 Rust 编译器版本
  - 提升部署二进制文件的可追溯性，便于问题排查

### Changed
- 更新项目依赖到最新版本。

## [0.15.0] - 2025-01-17

### Changed
- 升级 `wp-engine` 核心引擎到 v1.10.0-alpha 版本，主要变化包括：
  - **新增 KvArr 解析器**：支持键值对数组格式解析（`key=value` 或 `key:value`），支持灵活的分隔符（逗号、空格或混合），自动类型推断，重复键自动数组索引
  - **修复 meta 字段问题**：修复了 meta fields 在 sub-parser 上下文中被忽略的问题
  - **API 改进**：修复了 wp-cli-core 中 `validate_groups` 函数导出问题，现在从 `wp_cli_core::utils::validate` 模块导出
- 升级 `wp-model-core` 到 0.7.1 版本。

## [0.14.0] - 2025-01-16

### Added
- 新增 `wproj rescue stat` 命令，用于统计 rescue 目录中的数据：
  - 支持按 sink 分组统计文件数量、记录条数和文件大小
  - 支持 `--detail` 显示文件详情
  - 支持 `--json` 和 `--csv` 多种输出格式
- 新增 Doris 连接器支持，现在可以直接将数据写入 Apache Doris 数据库。
- GitHub Release 发布流程新增自动提取 CHANGELOG 功能：
  - 自动从 CHANGELOG.md 和 CHANGELOG.en.md 提取对应版本的更新内容
  - 默认展示英文 changelog，中文内容以折叠区域形式显示
  - 通过 scripts/extract-changelog.sh 脚本实现

### Changed
- 升级 `wp-engine` 核心引擎到 v1.9.0-alpha.2 版本，主要变化包括：
  - **动态速率控制模块**：新增 `SpeedProfile` 支持多种速率模式（恒定、正弦波、阶梯、突发、斜坡、随机游走、复合模式），用于模拟真实流量场景
  - **Rescue 统计模块**：新增 rescue 数据统计功能，支持按 sink 分组统计、多种输出格式（表格、JSON、CSV）
  - **wpgen.toml 配置增强**：支持在配置文件中定义 `speed_profile` 动态速率配置
  - **BlackHoleSink 增强**：新增 `sink_sleep_ms` 参数，支持控制每次 sink 操作的延迟

### Fixed
- 修复 wpgen 配置中 `speed_profile` 动态生成率未生效的问题，现在可以正确从配置文件读取并应用 sinusoidal、stepped、burst 等动态速率模式。
- 修复升级 wp-engine 后 `GenGRA` 缺少 `speed_profile` 字段导致的编译错误。
- 修复 dependabot-branch-filter 工作流中的 YAML 语法错误。
- 修复 adm.gxl 配置文件相关问题。

### Documentation
- 移除过时的技术设计和用户指南文档，清理文档结构。

[0.14.0]: https://github.com/wp-labs/warp-parse/releases/tag/v0.14.0

## [0.13.1] - 2026-01-14

### Changed
- 升级 `wp-engine` 核心引擎到 v1.8.2-beta 版本，获取最新的运行时特性与性能优化。
- 升级 `wp-connectors` 连接器到 v0.7.5-alpha 版本，提升数据源适配稳定性。
- 更新 CI 工作流，新增基于 wp-examples 仓库的集成测试步骤，确保发布质量。
- 清理未使用的模板文件 `_gal/tpl/Cargo.toml` 和工作流配置，简化项目结构。
- 更新 README 中的性能测试相关说明与示例。

[0.13.1]: https://github.com/wp-labs/warp-parse/releases/tag/v0.13.1

## [0.13.0] - 2024-05-09

> :information_source: 本次版本紧随 [wp-engine v1.8.0 changelog](https://github.com/wp-labs/wp-engine/releases/tag/v1.8.0) 调整，CLI 侧变更以适配核心引擎 API 为主，建议同时阅读引擎发布说明以了解 runtime 行为差异。

### Added
- 全新 **Field Pipe** 方案文档《docs/field-pipe-design.md》，阐述字段集合 pipe 与单字段 pipe 拆分后的执行模型，帮助使用者理解 `take/last/@key` 等 selector 与 `base64_decode` 等函数的协作方式。
- `wproj` 数据、统计、验证子命令现在会自动加载安全字典 (`EnvDict`)，无需手动设置即可获取密钥、变量等运行态配置。

### Changed
- `wproj`、`wparse`、`wprescue` 三个 CLI 统一改用 `wp_cli_core::split_quiet_args` 处理 `-q/--quiet`，并在入口注册运行时特性，保证安静模式与插件加载行为一致。
- 全量迁移到 `wp_cli_core` 的 sink/source 统计与校验实现：`stat`/`validate` 输出直接使用核心库排版，路由/OML 展示与引擎保持一致；`wpgen rule` 的直连执行也会把运行时变量下发给引擎层。
- 模板 `_gal/tpl/Cargo.toml` 与主工程 `Cargo.toml` 更新依赖，去除废弃的 `wp-cli-utils`，直接引用 `wp-cli-core` 以获得最新 CLI 能力集合。

### Fixed
- 适配 `wp-engine` v1.8.0 升级后的 API（例如 `WarpProject::init/load`、`load_warp_engine_confs`、`collect_oml_models` 等）需要显式 `EnvDict` 参数的问题，解决多处编译错误并提升运行时的配置一致性。
- 统计/验证命令在非 JSON 模式下与 `wp-cli-core` 类型不匹配导致的显示/解析崩溃，当前统一转换为核心库格式后即可正常输出。

[0.13.0]: https://github.com/wp-labs/warp-parse/releases/tag/v0.13.0
