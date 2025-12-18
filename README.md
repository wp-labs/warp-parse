<p align="center">
  <img src="docs/WP—LOGO.V2.png" alt="Warp Parse Logo" width="200"/>
</p>

# Warp Parse

[![Build & Test](https://github.com/wp-labs/warp-parse/actions/workflows/build-and-test.yml/badge.svg)](https://github.com/wp-labs/warp-parse/actions/workflows/build-and-test.yml)
[![Release](https://github.com/wp-labs/warp-parse/actions/workflows/release.yml/badge.svg)](https://github.com/wp-labs/warp-parse/actions/workflows/release.yml)
[![License: Elastic 2.0](https://img.shields.io/badge/License-Elastic%202.0-green.svg)](https://www.elastic.co/licensing/elastic-license)
[![Rust Version](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)

Warp Parse is a high-performance Rust ETL engine built for observability, security, real-time risk control, and data platform teams. It focuses on log/telemetry ingestion, parsing, and transformation, providing high-throughput parsing (WPL), transformation (OML), routing, unified connector APIs, and streamlined operations.

> Documentation lives in `docs/` (mdBook). `docs/README.md` is the overview, and detailed performance data is in `docs/performance.md`.

## Core Features

- **Extreme throughput:** Outperforms Vector across numerous scenarios (see `docs/performance.md`).
- **Readable rules:** WPL (parse DSL) + OML (transform DSL) offer far better readability than regular expressions and Lua.
- **Unified connectors:** Based on `wp-connector-api` for easy community ecosystem extension.
- **Ops friendly:** Single binary deployment, configuration-based; provides `wproj`, `wpgen`, `wprescue` tool suite.
- **Knowledge transformation:** Enables data enrichment through SQL queries with in-memory database.
- **Data routing:** Routes data based on rules and transformation models, supports multi-path replication and filters.

## Git Repository Overview

| Repository | Description |
| ---------- | ----------- |
| `warp-parse` | WarpParse Community Edition |
| `wp-advanced-api` | Advanced Control Interface |
| `wp-connectors` | Connector Library |
| `wp-docs` | Documentation |
| `wp-engine` | Engine Core |
| `wp-example` | Usage Examples |
| `wp-infras` | Infrastructure Library |
| `wp-rule` | Rule Library |
| `wp-open-api` | Open Extension Interface |

## Repository Layout

| Path | Description |
| ---- | ----------- |
| `Cargo.toml`, `build.rs` | Workspace manifest and build metadata. |
| `wparse/`, `wpgen/`, `wprescue/` | CLI main programs, features registered in `feats.rs`/`plugins.rs`. |
| `connectors/source.d/`, `connectors/sink.d/` | Sample source/sink connector configs. |
| `examples/` | Runnable examples and configurations. |
| `docs/` | mdBook documentation, includes performance report. |
| `../wp-engine`, `../wp-connectors` | Upstream repositories providing engine and connector capabilities. |

## License

Elastic License 2.0 (ELv2) - see [https://www.elastic.co/licensing/elastic-license](https://www.elastic.co/licensing/elastic-license) for details.

---

<p align="center">
  <img src="docs/WP—LOGO.V2.png" alt="Warp Parse Logo" width="200"/>
</p>

# Warp Parse（中文版）

[![Build & Test](https://github.com/wp-labs/warp-parse/actions/workflows/build-and-test.yml/badge.svg)](https://github.com/wp-labs/warp-parse/actions/workflows/build-and-test.yml)
[![Release](https://github.com/wp-labs/warp-parse/actions/workflows/release.yml/badge.svg)](https://github.com/wp-labs/warp-parse/actions/workflows/release.yml)
[![License: Elastic 2.0](https://img.shields.io/badge/License-Elastic%202.0-green.svg)](https://www.elastic.co/licensing/elastic-license)
[![Rust Version](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)

Warp Parse 是面向可观测性、安全、实时风控、数据平台团队的高性能 ETL 引擎，专注于日志/事件接入、解析与转换，提供高吞吐解析（WPL）、转换（OML）、路由、统一连接器 API 及极简运维体验。

> 完整文档位于 `docs/`（mdBook）。`docs/README.md` 为概览，性能数据详见 `docs/performance.md`。

## 核心特性

- **极致吞吐：** 众多场景下性能全面超越 Vector（详见 `docs/performance.md`）。
- **规则易编写：** WPL（解析 DSL）+ OML（转换 DSL），可读性远超正则表达式和 Lua。
- **连接器统一：** 基于 `wp-connector-api`，便于社区生态扩展。
- **运维友好：** 单二进制部署，配置化；提供 `wproj`、`wpgen`、`wprescue` 工具套件。
- **知识转换：** 通过内存数据库支持 SQL 查询，实现数据富化。
- **数据路由：** 基于规则和转换模型进行路由，支持多路复制与过滤器。

## Git 仓库说明

| 仓库 | 说明 |
| ---- | ---- |
| `warp-parse` | WarpParse 社区版 |
| `wp-advanced-api` | 高级控制接口 |
| `wp-connectors` | 连接器库 |
| `wp-docs` | 使用文档 |
| `wp-engine` | 引擎核心 |
| `wp-example` | 使用示例 |
| `wp-infras` | 基础库 |
| `wp-rule` | 规则库 |
| `wp-open-api` | 开放扩展接口 |

## 项目结构

| 路径 | 说明 |
| ---- | ---- |
| `Cargo.toml`、`build.rs` | 工作区清单与构建信息。 |
| `wparse/`、`wpgen/`、`wprescue/` | CLI 主程序，特性注册在 `feats.rs`/`plugins.rs`。 |
| `connectors/source.d/`、`connectors/sink.d/` | 源/汇连接器示例配置。 |
| `examples/` | 可运行的示例与配置。 |
| `docs/` | mdBook 文档，包含性能报告。 |
| `../wp-engine`、`../wp-connectors` | 上游仓库，提供引擎和连接器能力。 |

## 许可协议

Elastic License 2.0 (ELv2) - 详情请参阅 [https://www.elastic.co/licensing/elastic-license](https://www.elastic.co/licensing/elastic-license)
