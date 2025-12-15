# Sink 配置基础

本文档介绍 warp-flow 系统中数据输出端 (Sink) 的基础概念和配置结构。

## 概述

Sink 是 warp-flow 系统的数据输出端，负责将处理后的数据发送到各种目标系统。系统支持多种输出类型，包括文件、Syslog、Prometheus 等。

## 核心概念

### 1. 配置层次结构

warp-flow 系统采用分层配置架构：

```
全局默认配置 (defaults.toml)
    ↓
路由组配置 (business.d/**/*.toml, infra.d/**/*.toml)
    ↓
连接器定义 (connectors/sink.d/*.toml)
    ↓
解析后的 Sink 实例 (ResolvedSinkSpec)
```

### 2. 核心数据结构

**Sink 组结构** (`RouteGroup`)：
```rust
pub struct RouteGroup {
    pub name: String,              // 路由组名称
    pub parallel: Option<usize>,   // 并行度
    pub oml: Option<StrOrVec>,     // OML 模式匹配
    pub rule: Option<StrOrVec>,    // 规则匹配
    pub tags: Option<Vec<String>>, // 组级标签
    pub expect: Option<GroupExpectSpec>, // 期望值配置
    pub sinks: Vec<RouteSink>,     // Sink 列表
}
```

**Sink 实例结构** (`RouteSink`)：
```rust
pub struct RouteSink {
    pub name: String,              // Sink 名称
    pub connect: String,           // 连接器ID
    pub params: Option<toml::value::Table>, // 参数（扁平覆写）
    pub filter: Option<String>,    // 过滤器文件路径
    pub tags: Vec<String>,         // Sink 级标签
    pub expect: Option<SinkExpectOverride>, // 期望值覆盖
}
```

## 配置文件结构

### 1. 连接器定义 (connectors.toml)

```toml
# connectors/sink.d/file_raw_sink.toml
[[connectors]]
id = "file_raw_sink"
type = "file"
allow_override = ["base", "file", "fmt"]

[connectors.params]
base = "./data/out_dat"
file = "default.dat"
fmt = "json"
```

**关键字段说明**：
- `id`: 连接器唯一标识符
- `type`: 连接器类型 (file, syslog, prometheus 等)
- `allow_override`: 允许源配置覆盖的参数列表
- `params`: 连接器默认参数

### 2. 路由配置 (business.d/**/*.toml, infra.d/**/*.toml)

```toml
# business.d/example.toml
version = "2.0"

[sink_group]
name = "/sink/example"
oml = ["example_pattern"]
parallel = 2
tags = ["env:production"]

[[sink_group.sinks]]
name = "example_sink"
connect = "file_raw_sink"
params = {
    base = "./output",
    file = "example.dat"
}
filter = "./filter.wpl"
tags = ["type:example"]

[sink_group.sinks.expect]
ratio = 1.0
tol = 0.01
```

### 3. 全局默认配置 (defaults.toml)

```toml
# defaults.toml
version = "2.0"

[defaults]
tags = ["env:default"]

[defaults.expect]
basis = "total_input"
min_samples = 100
mode = "error"
```

## 基础配置示例

### 1. 简单文件输出
```toml
# infra.d/simple_file.toml
version = "2.0"

[sink_group]
name = "simple_output"
oml = []
[[sink_group.sinks]]
connect = "file_raw_sink"
params = { file = "simple.log" }
```

### 2. 带过滤器的输出
```toml
# business.d/filtered_output.toml
version = "2.0"

[sink_group]
name = "/sink/filtered"
oml = ["/oml/logs/*"]

[[sink_group.sinks]]
name = "all_logs"
connect = "file_json_sink"
params = { file = "all_logs.json" }

[[sink_group.sinks]]
name = "error_logs"
connect = "file_json_sink"
filter = "./error_filter.wpl"
params = { file = "error_logs.json" }
[sink_group.sinks.expect]
ratio = 0.1
tol = 0.02
```

### 3. 并行输出配置（仅业务组）
```toml
# business.d/parallel_output.toml
version = "2.0"

[sink_group]
name = "/sink/parallel"
oml = ["high_volume"]
parallel = 4
tags = ["type:parallel"]

[[sink_group.sinks]]
name = "output_1"
connect = "file_proto_sink"
params = { file = "output_1.dat" }

[[sink_group.sinks]]
name = "output_2"
connect = "file_proto_sink"
params = { file = "output_2.dat" }
```
注：基础组（infra.d）不支持 `parallel` 与文件分片；如需提升吞吐与分片，请在业务组配置。

## 标签系统

### 1. 标签继承层次

标签系统支持三层继承：
1. **默认标签** (来自 defaults.toml)
2. **组级标签** (来自 sink_group)
3. **Sink 级标签** (来自具体 sink)

### 2. 标签配置示例
```toml
# defaults.toml
[defaults]
tags = ["env:production", "service:warpflow"]

# business.d/example.toml
[sink_group]
tags = ["region:us-west", "tier:processing"]

[[sink_group.sinks]]
tags = ["output:file", "compression:gzip"]
```

**最终标签合并结果**：
```
["env:production", "service:warpflow", "region:us-west", "tier:processing", "output:file", "compression:gzip"]
```

## 连接器类型

### 1. 文件连接器
```toml
[[connectors]]
id = "file_json_sink"
type = "file"
allow_override = ["base", "file", "fmt"]
[connectors.params]
fmt = "json"
```

### 2. Syslog 连接器
```toml
[[connectors]]
id = "syslog_tcp"
type = "syslog"
allow_override = ["addr", "port", "protocol"]
[connectors.params]
addr = "127.0.0.1"
port = 1514
protocol = "tcp"
```

### 3. Prometheus 连接器（内置 Exporter）
```toml
[[connectors]]
id = "prometheus_local"
type = "prometheus"
allow_override = ["endpoint", "source_key_format", "sink_key_format"]
[connectors.params]
# 监听地址（对外暴露 /metrics），示例："127.0.0.1:35666"
endpoint = "127.0.0.1:35666"
# 可选：用于从 key 中提取标签的正则（具名分组）
source_key_format = "(?P<source_type>.)_(?P<access_source>.)"
sink_key_format   = "(?P<rule>.)_(?P<sink_type>.)_sink"
```

## 适配器与 conn_url（进阶）

- 若应用注册了连接器适配器（wp-connector-api::config::adapter），可以通过 `conn_url` 简写来生成连接参数，避免手写繁琐的 `params`：
  - MySQL（sink）：`mysql://user:pass@host:3306/db` → 解析为 `{ endpoint, username, password, database, ... }`
  - Kafka（sink）：`kafka://broker1,broker2?topic=xxx&num_partitions=3` → 解析为 `{ brokers, topic, num_partitions, ... }`
- 本仓库的 wparse/wproj/wprescue 已默认注册开发期适配器（dev adapters），便于本地验证与示例；生产环境建议改为接入正式的连接器扩展并在应用启动时注册相应适配器。
- 说明：当前 route 文件不直接解析 `params.conn_url`；建议在生成/CLI 侧使用 conn_url（例如 wproj 命令或生成脚手架）。

说明：当前实现为“自暴露”型 Exporter，不支持自定义 `metric_name/metric_type` 与 Pushgateway 推送；指标名固定为内置计数器（如 `wparse_receive_data`、`wparse_send_to_sink` 等）。

## 期望值配置 (Expect)

### 1. 比例模式
```toml
[sink_group.sinks.expect]
ratio = 1.0    # 期望占比 100%
tol = 0.01     # 允许偏差 ±1%
```

### 2. 范围模式
```toml
[sink_group.sinks.expect]
min = 0.001    # 最小占比 0.1%
max = 2.0      # 最大占比 200%
```

### 3. 全局默认期望值
```toml
[defaults.expect]
basis = "total_input"  # 计算基准
min_samples = 100      # 最小样本数
mode = "error"         # 违规时处理模式
```

## 过滤器配置

### 1. 过滤器文件
过滤器文件使用 WPL (Warp Processing Language) 语法：

```wpl
# filter.wpl
# 只处理错误级别的日志
level == "ERROR" || level == "FATAL"

# 或者复杂条件
(level == "ERROR" && source == "auth") ||
(level == "WARN" && message ~= "timeout")
```

### 2. 过滤器应用
```toml
[[sink_group.sinks]]
name = "filtered_output"
connect = "file_json_sink"
filter = "./error_filter.wpl"    # 应用过滤器
params = { file = "errors.json" }
```

## 配置验证

### 1. 参数覆盖验证
系统严格验证参数覆盖：
- 只能覆盖 `allow_override` 中指定的参数
- 不支持嵌套表结构覆盖

### 2. 唯一性验证
- 同一 sink_group 内 sink 名称必须唯一
- 连接器 ID 必须全局唯一

### 3. 文件存在性验证
- 过滤器文件必须存在且语法正确
- 文件路径参数必须有效

## 配置工具

### 1. 验证配置
```bash
# 验证 sink 配置
wpgen sink validate

# 验证路由配置
wpgen sink route validate

# 验证连接器配置
wpgen connectors validate
```

### 2. 测试连接
```bash
# 测试特定 sink
wpgen sink test --name file_output

# 测试连接器
wpgen connectors test --id file_raw_sink
```

## 最佳实践

### 1. 命名规范
- 连接器文件与 id：遵循《[连接器命名规范](../../10-user/02-config/connectors_naming.md)`
  - 文件名：`NN-<kind>-<variant>.toml`（如 `02-file-json.toml`, `10-syslog-udp.toml`）
  - id：`<kind>_<variant>[_sink]`（如 `file_json_sink`、`syslog_udp_sink`）
- Sink 组名称：简明、可分组（如 `demo`, `metrics`）
- Sink 名称：组内唯一（如 `json`, `error_logs`）

### 2. 目录组织
```
sink/
├── business.d/          # 业务路由配置
├── infra.d/            # 基础设施路由配置
├── defaults.toml       # 全局默认配置
├── privacy.toml        # 已废弃（v2 起默认不启用隐私/脱敏；如需，请在业务侧或自定义管道实现）
└── connectors/         # 连接器定义
    └── sink.d/
        ├── 00-file-default.toml
        ├── 10-syslog-tcp.toml
        └── 20-prometheus.toml
```

### 3. 标签规范
```toml
tags = [
    "env:production",           # 环境
    "service:authentication",   # 服务名称
    "type:access_log",         # 日志类型
    "region:us-west",          # 地理区域
    "tier:edge"                # 系统层级
]
```

## 相关文档

- [Sink 路由](./routing.md)
- [Sink 并行与分片](./parallel_and_sharding.md)
- [连接器管理](./connectors.md)
- [defaults 与 expect](./defaults_expect.md)
- [Sink CLI：Validate](./validate_route.md)
