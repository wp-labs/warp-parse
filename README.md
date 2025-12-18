<p align="center">
  <img src="docs/WP—LOGO.V2.png" alt="Warp Parse Logo" width="200"/>
</p>

<p align="center">
  <a href="https://github.com/wp-labs/warp-parse/actions/workflows/build-and-test.yml">
    <img src="https://github.com/wp-labs/warp-parse/actions/workflows/build-and-test.yml/badge.svg" alt="Build & Test"/>
  </a>
  <a href="https://github.com/wp-labs/warp-parse/actions/workflows/release.yml">
    <img src="https://github.com/wp-labs/warp-parse/actions/workflows/release.yml/badge.svg" alt="Release"/>
  </a>
  <a href="https://www.elastic.co/licensing/elastic-license">
    <img src="https://img.shields.io/badge/License-Elastic%202.0-green.svg" alt="License: Elastic 2.0"/>
  </a>
  <a href="https://www.rust-lang.org">
    <img src="https://img.shields.io/badge/rust-1.75+-orange.svg" alt="Rust Version"/>
  </a>
</p>

<h1 align="center">Warp Parse</h1>

<p align="center">
  <strong>高性能 Rust ETL 引擎，专为极致日志处理而设计</strong>
</p>

<p align="center">
  <a href="#核心特性">特性</a> •
  <a href="#快速开始">快速开始</a> •
  <a href="#文档">文档</a> •
  <a href="#性能基准">性能</a> •
  <a href="#许可证">许可证</a>
</p>

---

Warp Parse is a high-performance Rust ETL engine built for observability, security, real-time risk control, and data platform teams. It focuses on log/telemetry ingestion, parsing, and transformation, providing extreme throughput parsing (WPL), transformation (OML), routing, unified connector APIs, and streamlined operations.

> 📚 **Documentation:** `docs/` (mdBook) • 📊 **Performance:** `docs/performance.md`

## ✨ Core Features

### 🚀 Extreme Throughput
- **2.4M+ EPS** for Nginx log parsing
- **10x+ faster** than Vector in production scenarios
- Zero-copy parsing combinators with Tokio async runtime
- Sustains **1000+ MiB/s** for large log processing

### 📝 Readable Rules
- **WPL** (Warp Processing Language) - Parse DSL with intuitive syntax
- **OML** (Object Markup Language) - Transform DSL for complex data manipulation
- Rules are **30-50% smaller** than equivalent Vector configurations
- Human-readable and maintainable, unlike complex regex

### 🔌 Unified Connectors
- Built on `wp-connector-api` for consistent behavior
- Extensible plugin architecture
- Community-friendly development framework
- Feature-gated optional components

### 🛠️ Ops Friendly
- **Single binary** deployment - no external dependencies
- Configuration-driven with TOML files
- Complete tool suite:
  - `wproj` - Project management
  - `wpgen` - Data generation for testing
  - `wprescue` - Data recovery tools

### 🧠 Knowledge Transformation
- In-memory database for data enrichment
- SQL query support for complex joins
- Real-time data correlation and lookup

### 🎯 Data Routing
- Rule-based intelligent routing
- Multi-path data replication
- Advanced filtering capabilities
- Dynamic sink configuration

## 🏗️ Git Repository Overview

| Repository | Description |
| ---------- | ----------- |
| [`warp-parse`](https://github.com/wp-labs/warp-parse) | ⭐ WarpParse Community Edition |
| `wp-advanced-api` | Advanced Control Interface |
| `wp-connectors` | Connector Library |
| `wp-docs` | Documentation |
| `wp-engine` | Engine Core |
| `wp-example` | Usage Examples |
| `wp-infras` | Infrastructure Library |
| `wp-rule` | Rule Library |
| `wp-open-api` | Open Extension Interface |

## 📁 Repository Layout

| Path | Description |
| ---- | ----------- |
| `Cargo.toml`, `build.rs` | Workspace manifest and build metadata |
| `wparse/`, `wpgen/`, `wprescue/` | CLI main programs |
| `connectors/` | Sample connector configurations |
| `examples/` | Ready-to-run examples |
| `docs/` | Comprehensive documentation |
| `../wp-engine` | Upstream engine crate |

## 🚀 Quick Start

```bash
# Install from source
git clone https://github.com/wp-labs/warp-parse.git
cd warp-parse
cargo build --release

# Or download pre-built binary
wget https://github.com/wp-labs/warp-parse/releases/latest/download/wparse-linux-x64
```

### Basic Usage

```bash
# Check configuration
wparse check

# Run in daemon mode
wparse daemon -c ./config/

# Generate test data
wpgen rule -n 1000 -o sample.log

# Process data
wparse batch -i sample.log -o output.json
```

## 📚 Documentation

- **User Guide**: [docs/user-guide](./docs/user-guide/)
- **Performance Report**: [docs/performance.md](./docs/performance.md)
- **Examples**: [examples/](./examples/)

## 📊 Performance Benchmarks

| Scenario | Warp Parse (EPS) | Vector (EPS) | Performance Gain |
| -------- | --------------- | ------------ | --------------- |
| Nginx (File) | **2,456,100** | 540,540 | **4.5x** |
| ELB (TCP) | **884,700** | 163,600 | **5.4x** |
| Sysmon JSON | **440,000** | 76,717 | **5.7x** |
| APT 3K | **314,200** | 33,614 | **9.3x** |

*Tested on Mac M4 10C/16G. Full report in [docs/performance.md](./docs/performance.md)*

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## 📄 License

Elastic License 2.0 (ELv2) - see [LICENSE](LICENSE) for details.

---

<p align="center">
  <strong>Built with ❤️ by the WP Labs team</strong>
</p>

---

# Warp Parse（中文版）

<p align="center">
  <strong>高性能 Rust ETL 引擎，专为极致日志处理而设计</strong>
</p>

<p align="center">
  <a href="#核心特性">特性</a> •
  <a href="#快速开始">快速开始</a> •
  <a href="#文档">文档</a> •
  <a href="#性能基准">性能</a> •
  <a href="#许可证">许可证</a>
</p>

---

Warp Parse 是面向可观测性、安全、实时风控、数据平台团队的高性能 ETL 引擎，专注于日志/事件接入、解析与转换，提供高吞吐解析（WPL）、转换（OML）、路由、统一连接器 API 及极简运维体验。

> 📚 **文档位置：** `docs/` (mdBook) • 📊 **性能数据：** `docs/performance.md`

## ✨ 核心特性

### 🚀 极致吞吐
- Nginx 日志解析 **240万+ EPS**
- 生产环境中比 Vector **快 10 倍以上**
- 基于 Tokio 异步运行时的零拷贝解析
- 大日志处理持续 **1000+ MiB/s** 吞吐

### 📝 规则易编写
- **WPL** (Warp Processing Language) - 语法直观的解析 DSL
- **OML** (Object Markup Language) - 复杂数据转换 DSL
- 规则比 Vector 配置 **小 30-50%**
- 人类可读且易维护，告别复杂正则

### 🔌 连接器统一
- 基于 `wp-connector-api` 保证行为一致
- 可扩展的插件架构
- 社区友好的开发框架
- 特性门控的可选组件

### 🛠️ 运维友好
- **单二进制**部署 - 无外部依赖
- TOML 配置文件驱动
- 完整工具套件：
  - `wproj` - 项目管理
  - `wpgen` - 测试数据生成
  - `wprescue` - 数据恢复工具

### 🧠 知识转换
- 内存数据库支持数据富化
- SQL 查询支持复杂关联
- 实时数据关联查询

### 🎯 数据路由
- 基于规则的智能路由
- 多路数据复制
- 高级过滤功能
- 动态输出配置

## 🏗️ Git 仓库说明

| 仓库 | 说明 |
| ---- | ---- |
| [`warp-parse`](https://github.com/wp-labs/warp-parse) | ⭐ WarpParse 社区版 |
| `wp-advanced-api` | 高级控制接口 |
| `wp-connectors` | 连接器库 |
| `wp-docs` | 使用文档 |
| `wp-engine` | 引擎核心 |
| `wp-example` | 使用示例 |
| `wp-infras` | 基础库 |
| `wp-rule` | 规则库 |
| `wp-open-api` | 开放扩展接口 |

## 📁 项目结构

| 路径 | 说明 |
| ---- | ---- |
| `Cargo.toml`, `build.rs` | 工作区清单和构建信息 |
| `wparse/`, `wpgen/`, `wprescue/` | CLI 主程序 |
| `connectors/` | 连接器示例配置 |
| `examples/` | 可运行示例 |
| `docs/` | 完整文档 |
| `../wp-engine` | 上游引擎库 |

## 🚀 快速开始

```bash
# 从源码安装
git clone https://github.com/wp-labs/warp-parse.git
cd warp-parse
cargo build --release

# 或下载预编译二进制
wget https://github.com/wp-labs/warp-parse/releases/latest/download/wparse-linux-x64
```

### 基本使用

```bash
# 检查配置
wparse check

# 守护进程模式运行
wparse daemon -c ./config/

# 生成测试数据
wpgen rule -n 1000 -o sample.log

# 处理数据
wparse batch -i sample.log -o output.json
```

## 📚 文档

- **用户指南**: [docs/user-guide](./docs/user-guide/)
- **性能报告**: [docs/performance.md](./docs/performance.md)
- **示例**: [examples/](./examples/)

## 📊 性能基准

| 场景 | Warp Parse (EPS) | Vector (EPS) | 性能提升 |
| ---- | --------------- | ------------ | -------- |
| Nginx (File) | **2,456,100** | 540,540 | **4.5倍** |
| ELB (TCP) | **884,700** | 163,600 | **5.4倍** |
| Sysmon JSON | **440,000** | 76,717 | **5.7倍** |
| APT 3K | **314,200** | 33,614 | **9.3倍** |

*测试环境：Mac M4 10C/16G。完整报告见 [docs/performance.md](./docs/performance.md)*

## 🤝 贡献

欢迎贡献代码！请查看我们的 [贡献指南](CONTRIBUTING.md) 了解详情。

## 📄 许可协议

Elastic License 2.0 (ELv2) - 详情请参阅 [LICENSE](LICENSE)。

---

<p align="center">
  <strong>由 WP Labs 团队用 ❤️ 构建</strong>
</p>