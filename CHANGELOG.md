# Changelog

[English](./CHANGELOG.en.md) | 中文

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
