# Warp Parse

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

Apache-2.0 (commercial use permitted).

---

# Warp Parse（中文版）

Warp Parse 是面向安全/数据平台团队的高性能 Rust ETL 引擎，主打日志/事件接入、解析与转换。核心包括高吞吐解析（WPL）、转换（OML）、统一连接器 API 以及极简交付运维。

> 完整文档位于 `docs/`（mdBook）。`docs/README.md` 为概览页，性能基准详见 `docs/performance.md`。

## 关键特性

- **极致吞吐：** 基于 Tokio、零拷贝与解析组合器。在 Mac M4 10C/16G 基准中，Nginx 解析达到 **2,456,100 EPS / 559 MiB/s**，端到端 TCP->File 解析+转换维持 **797,700 EPS**，APT 3K 大包保持 **1062 MiB/s**（参见 `docs/performance.md`）。
- **可编程规则：** WPL（解析 DSL）+ OML（转换 DSL）让复杂协议可读、可复用；规则体积显著小（如 Nginx 解析 174B vs Vector 416B）。
- **连接器统一：** 基于 `wp-connector-api`，Source/Sink 以统一接口注册（如 `wparse/feats.rs`），三大 CLI（`wparse`/`wpgen`/`wprescue`）共享扩展能力。
- **运维友好：** 单二进制部署，配置文件化；`wpkit` 自带诊断，特性开关支持 `community`、`kafka`、`runtime-core` 等不同场景。

## 项目结构

| 路径 | 说明 |
| ---- | ---- |
| `Cargo.toml`、`build.rs` | 工作区清单与 `shadow-rs` 构建信息。 |
| `wparse/`、`wpgen/`、`wprescue/` | CLI 主程序，特性注册在 `feats.rs` / `plugins.rs`。 |
| `connectors/source.d/`、`connectors/sink.d/` | 源/汇连接器示例配置（TOML）。 |
| `examples/` | 可直接运行的样例与配置。 |
| `docs/` | mdBook 文档，含 `performance.md` 性能报告。 |
| `../wp-engine`、`../wp-connectors` | 上游 sibling 仓库，提供引擎/连接器能力（如 Kafka）。 |

## 快速开始

```bash
cargo build                   # 默认启用 community（kafka/mysql）
cargo build --release         # 生产优化版本
cargo run --bin wparse -- --help
cargo run --bin wpgen  -- --help
cargo run --bin wpkit  -- --help
```

常用特性：

```bash
cargo build --features kafka
cargo build --no-default-features --features runtime-core
```

## 测试与质量保障

- 代码格式：`cargo fmt --all`
- 静态检查：`cargo clippy --all-targets --all-features -- -D warnings`
- 单/集成测试：`cargo test`（按需追加 `--features sink_test` 等）
- 基准：`cargo bench`

提交前请执行 `fmt`、`clippy`、`test`，并构建关键特性集（如 `community`、`kafka`）。

## 文档与示例

- 文档构建：`make -C docs build` / `make -C docs serve`
- 源/汇配置：`docs/10-user/04-sources/README.md`、`docs/10-user/03-sinks/README.md`
- WPL/OML 指南：`docs/10-user/06-wpl/01-wpl_basics.md`、`docs/10-user/07-oml/01-oml_basics.md`
- 性能报告：`docs/performance.md`
- 示例：`examples/`

## 许可协议

Apache-2.0，可商用。
