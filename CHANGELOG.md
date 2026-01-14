# Changelog

[English](./CHANGELOG.en.md) | 中文

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
