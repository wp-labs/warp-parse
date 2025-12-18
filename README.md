# Warp Parse

[![Build & Test](https://github.com/wp-labs/warp-parse/actions/workflows/build-and-test.yml/badge.svg)](https://github.com/wp-labs/warp-parse/actions/workflows/build-and-test.yml)
[![Release](https://github.com/wp-labs/warp-parse/actions/workflows/release.yml/badge.svg)](https://github.com/wp-labs/warp-parse/actions/workflows/release.yml)
[![License: Elastic 2.0](https://img.shields.io/badge/License-Elastic%202.0-green.svg)](https://www.elastic.co/licensing/elastic-license)
[![Rust Version](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)

Warp Parse is a high-performance Rust ETL engine built for log/telemetry ingestion, enrichment, and delivery. It focuses on high-throughput parsing (WPL) and transformation (OML) pipelines, unified connector APIs, and turnkey operability for security and data engineering teams.

> Documentation lives in `docs/` (mdBook). The short landing page is at `docs/README.md`, while detailed performance data is in `docs/performance.md`.

## Highlights

- **Extreme throughput:** Tokio + zero-copy parsing combinators; on a Mac M4 10C/16G bench Warp Parse reached **2,456,100 EPS / 559 MiB/s** for Nginx parsing and sustained **797,700 EPS** in end-to-end TCP->File parse+transform (Vector reference: 470,312 EPS). APT 3K logs hold **1062 MiB/s** throughput (see `docs/performance.md`).
- **Programmable rules:** WPL (parse DSL) + OML (transform DSL) keep complex rules readable, versionable, and lightweight (e.g., Nginx parse rule 174B vs. Vector 416B).
- **Unified connectors:** Sources/Sinks implemented via `wp-connector-api` and registered in `wparse/feats.rs` or `plugins.rs`, ensuring consistent behavior across binaries (`wparse`, `wpgen`, `wprescue`).
- **Ops ready:** Single binary deployment, file-based configuration, built-in `wpkit` health utilities, and feature flags for optional stacks (`community`, `kafka`, `runtime-core`, etc.).

## Repository Layout

| Path | Description |
| ---- | ----------- |
| `Cargo.toml`, `build.rs` | Workspace manifest, build metadata (`shadow-rs`). |
| `wparse/`, `wpgen/`, `wprescue/` | CLI binaries (parse runtime, generator, rescue kit). Feature wiring lives in `feats.rs`/`plugins.rs`. |
| `connectors/source.d/`, `connectors/sink.d/` | Sample connector configs (TOML). |
| `examples/` | Runnable scenarios and configs for quick POC. |
| `docs/` | mdBook documentation (`make -C docs build`). `performance.md` hosts benchmark tables. |
| `../wp-engine`, `../wp-connectors` | Upstream sibling crates referenced as path deps for advanced features (e.g., Kafka). |

## Getting Started

```bash
cargo build                   # default "community" feature
cargo build --release         # optimized binaries
cargo run --bin wparse -- --help
cargo run --bin wpgen  -- --help
cargo run --bin wpkit  -- --help
```

Feature examples:

```bash
cargo build --features kafka
cargo build --no-default-features --features runtime-core
```

## Testing & QA

- Format: `cargo fmt --all`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
- Tests: `cargo test` (append feature gates like `--features sink_test` as needed)
- Benchmarks: `cargo bench` (Criterion)

Before submitting changes, run `fmt`, `clippy`, `test`, and key feature builds (`community`, `kafka`).

## Documentation & Examples

- Docs: `make -C docs build` or `make -C docs serve`
- Config docs: `docs/10-user/04-sources/README.md`, `docs/10-user/03-sinks/README.md`
- DSL guides: `docs/10-user/06-wpl/01-wpl_basics.md`, `docs/10-user/07-oml/01-oml_basics.md`
- Performance deep dive: `docs/performance.md`
- Sample scenarios: `examples/`

## License

Elastic License 2.0 (ELv2) - see [https://www.elastic.co/licensing/elastic-license](https://www.elastic.co/licensing/elastic-license) for details.

---

# Warp Parse（中文版）

[![Build & Test](https://github.com/wp-labs/warp-parse/actions/workflows/build-and-test.yml/badge.svg)](https://github.com/wp-labs/warp-parse/actions/workflows/build-and-test.yml)
[![Release](https://github.com/wp-labs/warp-parse/actions/workflows/release.yml/badge.svg)](https://github.com/wp-labs/warp-parse/actions/workflows/release.yml)
[![License: Elastic 2.0](https://img.shields.io/badge/License-Elastic%202.0-green.svg)](https://www.elastic.co/licensing/elastic-license)
[![Rust Version](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)

Warp Parse 是面向可观测性、安全、实时风控、数据平台团队等对性能和实时性有极致要求场景的高性能 ETL 引擎，主打日志/事件接入、解析与转换。核心包括高吞吐解析（WPL）、转换（OML）、路由、统一连接器 API 以及极简交付运维。

> 完整文档位于 `docs/`（mdBook）。`docs/README.md` 为概览页，性能基准详见 `docs/performance.md`。

## 关键特性

- **极致吞吐：**  在众多场景全面超过Vector性能。
（参见 `docs/performance.md`）。
- **易编写规则：** WPL（解析 DSL）+ OML（转换 DSL）具有远超正式表达式、Lua 可读性。
- **连接器统一：** 基于 `wp-connector-api`方例社区扩展发展。
- **运维友好：** 单二进制部署，配置文件化；提供`wproj` `wpgen` `wprescue` 套件，方便工程管理、数据生成和 急救工具。
- **知识转换：** 通过内存数据库，支持知识富化(支持SQL)
- **数据路由：** 基于规则、转换模型进行数据路由、支持多路复制和过滤器

## 各Git Repo 用途：
* warp-parse : WarpParse社区版
* wp-advanced-api ： 高级控制接口
* wp-connectors ： 连接器
* wp-docs  ： 使用文档
* wp-engine ： 引擎
* wp-example ：使用示例
* wp-infras  ：基础库
* wp-rule ：基础库
* wp-open-api ：开放扩展接口
## 项目结构

| 路径 | 说明 |
| ---- | ---- |
| `Cargo.toml`、`build.rs` | 工作区清单与 `shadow-rs` 构建信息。 |
| `wparse/`、`wpgen/`、`wprescue/` | CLI 主程序，特性注册在 `feats.rs` / `plugins.rs`。 |
| `connectors/source.d/`、`connectors/sink.d/` | 源/汇连接器示例配置（TOML）。 |
| `examples/` | 可直接运行的样例与配置。 |
| `docs/` | mdBook 文档，含 `performance.md` 性能报告。 |
| `../wp-engine`、`../wp-connectors` | 上游 sibling 仓库，提供引擎/连接器能力（如 Kafka）。 |


## 许可协议

Elastic License 2.0 (ELv2) - 详情请参阅 [https://www.elastic.co/licensing/elastic-license](https://www.elastic.co/licensing/elastic-license)
