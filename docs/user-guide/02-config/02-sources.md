# 源配置（Sources）
<!-- 角色：使用配置者 | 最近验证：2025-12-11 -->

## 概览

Source（源）是 warp-flow 系统中负责数据输入的组件，支持多种数据源和协议。采用统一的连接器架构，提供灵活的数据接入能力。

### 定位与目录
- **配置文件**：`$WORK_ROOT/models/sources/wpsrc.toml`（兼容 legacy `models/source/wpsrc.toml` 与 `source/wpsrc.toml`）
- **连接器定义**：从 `$WORK_ROOT/models/sources` 起向上查找最近的 `connectors/source.d/*.toml`（≤32 层）

### 核心概念
- **连接器**：可复用的输入连接定义，包含 `id/type/params/allow_override`
- **参数覆写**：通过白名单机制安全覆写连接器参数
- **标签系统**：支持为数据源添加标签，便于路由和过滤

## 支持的 Source 类型

### 内置 Source
- **file**：文件输入，支持监控和轮询
- **null**：空输入，用于测试

### 扩展 Source
- **syslog**：Syslog 协议输入（UDP/TCP）
- **kafka**：Apache Kafka 消息队列输入

## 配置规则

### 基本规则
- 仅支持 `[[sources]] + connect/params_override` 格式
- `params` 为 `params_override` 的别名，推荐使用 `params_override`
- 覆写键必须 ∈ connector `allow_override` 白名单；超出即报错
- `enable` 字段控制是否启用（默认 true）
- `tags` 字段支持添加数据源标签

### 配置结构
```toml
[[sources]]
key = "source_identifier"           # 源的唯一标识
connect = "connector_id"            # 引用的连接器 ID
enable = true                       # 是否启用（可选，默认 true）
tags = ["source:tag1", "type:log"]  # 标签（可选）
params_override = {                 # 参数覆写（可选）
    # 覆写连接器参数
}
```

## 配置示例

### 最小示例
```toml
[[sources]]
key = "file_1"
connect = "file_src"
params_override = { base = "data/in_dat", file = "gen.dat" }
```

### 文件输入示例
```toml
# connectors/source.d/00-file-default.toml
[[connectors]]
id = "file_src"
type = "file"
allow_override = ["base", "file", "encode"]
[connectors.params]
base = "data/in_dat"
file = "gen.dat"
encode = "text"

# models/sources/wpsrc.toml
[[sources]]
key = "access_log"
connect = "file_src"
params_override = {
    base = "./logs",
    file = "access.log",
    encode = "text"
}
tags = ["type:access", "env:prod"]
```

### Syslog 输入示例
```toml
# connectors/source.d/syslog-udp.toml
[[connectors]]
id = "syslog_udp_src"
type = "syslog"
allow_override = ["addr", "port", "protocol", "header_mode", "prefer_newline"]
[connectors.params]
addr = "0.0.0.0"
port = 1514
protocol = "udp"
header_mode = "parse"
prefer_newline = false

# models/sources/wpsrc.toml
[[sources]]
key = "syslog_udp"
connect = "syslog_udp_src"
params_override = {
    port = 1514,
    header_mode = "parse",
    prefer_newline = true
}
tags = ["protocol:syslog", "transport:udp"]
```

### Kafka 输入示例
```toml
# connectors/source.d/30-kafka.toml
[[connectors]]
id = "kafka_src"
type = "kafka"
allow_override = ["topic", "group_id", "config"]
[connectors.params]
brokers = "localhost:9092"
topic = ["access_log"]
group_id = "wpgen_consumer"

# models/sources/wpsrc.toml
[[sources]]
key = "kafka_access"
connect = "kafka_src"
params_override = {
    topic = ["access_log", "error_log"],
    config = ["auto.offset.reset=earliest", "enable.auto.commit=true"]
}
tags = ["source:kafka", "type:log"]
```

### TCP 输入示例（通用 TCP 行/长度分帧）
```toml
# connectors/source.d/12-tcp.toml（已内置）
[[connectors]]
id = "tcp_src"
type = "tcp"
allow_override = ["addr", "port", "framing", "tcp_recv_bytes", "prefer_newline", "instances"]
[connectors.params]
addr = "0.0.0.0"
port = 19000
framing = "auto"          # auto|line|len
prefer_newline = true      # auto 时优先按行
tcp_recv_bytes = 10485760  # 10MiB
# instances = 1             # 可选：单 connectors 多实例（默认 1，最大 16）

# models/sources/wpsrc.toml
[[sources]]
key = "tcp_in"
connect = "tcp_src"
enable = true
tags = ["source:tcp", "type:raw"]
params_override = {
  port = 19000,
  framing = "auto",
  prefer_newline = true
}
```
提示：
- `framing = "line"`：按换行切分；`"len"`：按 `<len><space><payload>`（RFC6587 风格）
- `framing = "auto"`：默认优先长度前缀；当 `prefer_newline=true` 时优先按行；遇到“长度前缀进行中”不会回退按行
- 单帧最大 10MB（防御上限），长度前缀最多 10 位十进制
- `instances`：同一监听端口按 round-robin 产出多份 Source 实例，可提升 Parser 并发；默认 1，最多 16

### 复杂配置示例
```toml
# models/sources/wpsrc.toml

# 文件数据源
[[sources]]
key = "web_access"
connect = "file_src"
enable = true
tags = ["type:web", "format:access"]
params_override = {
    base = "./logs/web",
    file = "access.log",
    encode = "text"
}

# Syslog 数据源
[[sources]]
key = "syslog_security"
connect = "syslog_tcp_src"
enable = true
tags = ["type:security", "protocol:syslog"]
params_override = {
    addr = "0.0.0.0",
    port = 1515,
    protocol = "tcp",
    header_mode = "parse",
    prefer_newline = true
}

# Kafka 数据源
[[sources]]
key = "kafka_events"
connect = "kafka_src"
enable = false  # 暂时禁用
tags = ["source:kafka", "type:event"]
params_override = {
    topic = ["user_events", "system_events"],
    group_id = "event_processor",
    config = [
        "auto.offset.reset=earliest",
        "session.timeout.ms=30000"
    ]
}
```

## 连接器配置详解

### 文件连接器
```toml
[[connectors]]
id = "file_src"
type = "file"
allow_override = ["base", "file", "encode"]
[connectors.params]
base = "data/in_dat"            # 基础目录
file = "input.log"              # 文件名
encode = "text"                 # 编码格式：text|base64|hex
```

### Syslog 连接器
```toml
[[connectors]]
id = "syslog_udp_src"
type = "syslog"
allow_override = ["addr", "port", "protocol", "tcp_recv_bytes", "header_mode", "prefer_newline"]
[connectors.params]
addr = "0.0.0.0"                # 监听地址
port = 1514                     # 监听端口
protocol = "udp"                # 协议：udp|tcp
tcp_recv_bytes = 10485760       # TCP 接收缓冲区大小（可选）
header_mode = "parse"          # 头部处理：keep|strip|parse
prefer_newline = false          # 是否优先换行分帧
```

### Kafka 连接器
```toml
[[connectors]]
id = "kafka_src"
type = "kafka"
allow_override = ["topic", "group_id", "config"]
[connectors.params]
brokers = "localhost:9092"      # Kafka broker 地址
topic = ["access_log"]          # 主题列表
group_id = "consumer_group"     # 消费者组 ID
config = []                     # 额外配置（可选）
```

### TCP 连接器（源）
```toml
[[connectors]]
id = "tcp_src"
type = "tcp"
allow_override = ["addr", "port", "framing", "tcp_recv_bytes", "prefer_newline", "instances"]
[connectors.params]
addr = "0.0.0.0"                # 监听地址
port = 19000                    # 监听端口
framing = "auto"                # 分帧：auto|line|len
prefer_newline = false          # auto 模式是否优先按行
tcp_recv_bytes = 10485760       # TCP 接收缓冲
# instances = 1                 # 可选：同端口多实例（默认 1，最大 16）
```

## 标签系统

### 标签用途
- **路由选择**：基于标签进行数据路由
- **过滤条件**：在处理流程中基于标签过滤
- **监控统计**：按标签维度进行数据统计

### 常用标签约定
- `source:type`：数据源类型（file, syslog, kafka）
- `data:format`：数据格式（json, text, binary）
- `env:environment`：环境标识（dev, test, prod）
- `app:service`：关联的服务名称

### 标签示例
```toml
[[sources]]
key = "production_syslog"
connect = "syslog_main"
tags = [
    "source:syslog",
    "protocol:tcp",
    "env:production",
    "datacenter:us-west",
    "severity:all"
]
params_override = {
    port = 514
}
```


## CLI 工具

### 检查与验证
```bash
# 在工作根目录下执行

# 列出所有 source connectors 和引用关系
wproj sources list --work-root .

# 展示解析后的详细配置
wproj sources route --work-root .

# 验证配置文件语法
wproj sources validate --work-root .
```

### 输出信息
- **list**：显示连接器 ID、类型、被引用次数
- **route**：显示解析后的完整参数、标签、元数据
- **validate**：检查语法错误、连接器存在性、参数合法性

## 常见错误与排查

### 配置错误
- **覆写键超出白名单**：检查 `connector.allow_override` 配置
- **连接器未找到**：检查 connectors 目录和文件查找路径
- **参数类型错误**：确保覆写参数类型与连接器定义一致

### 路径问题
- **connectors 目录未找到**：遵守"从 models/sources 向上查找最近 connectors/source.d（≤32 层）"规则
- **相对路径解析**：确保所有路径相对于正确的工作目录

### 运行时问题
- **端口占用**：检查配置的端口是否被其他进程占用
- **权限问题**：确保有读取文件和绑定端口的权限
- **网络连接**：检查网络连通性和防火墙设置

## 性能优化

### 文件输入优化
- 使用 `inotify` 模式进行文件监控（Linux 系统）
- 合理设置缓冲区大小
- 考虑使用多个文件输入进行负载均衡

### Syslog 优化
- UDP 协议性能更好但可能丢包
- TCP 协议可靠但性能略低
- 调整 `tcp_recv_bytes` 缓冲区大小

### Kafka 优化
- 合理设置消费者组数量
- 调整 `session.timeout.ms` 和 `heartbeat.interval.ms`
- 使用适当的批处理大小


# wpsrc.toml（现代布局）

本文描述源配置：在 wpsrc.toml 内以 `[[sources]]` 声明条目，并通过 `connect` 引用 `connectors/source.d/*.toml` 中定义的连接器，再用 `params_override` 做白名单覆写。
注：为便于块表写法，支持使用 `params` 作为 `params_override` 的等价别名。推荐仍使用 `params_override` 以强调“覆写”语义。

- 默认位置：`$WORK_ROOT/models/sources/wpsrc.toml`（兼容 legacy `models/source/wpsrc.toml` 与 `source/wpsrc.toml`）
- 连接器目录：`$WORK_ROOT/connectors/source.d/*.toml`（从 `models/sources` 起向上查找最近的该目录，≤32 层）
- 兼容性：不再接受旧字段（如 `type/path/...`）；必须使用 `connect/params_override`（或其别名 `params`）
- 启停：每个源通过 `enable = true|false` 控制是否启用（缺省 true）
- 标签：`tags` 列表支持 `"k:v"` 或 `"k=v"` 语法；这些标签会注入到记录中

## 顶层结构

- [[sources]]：源条目列表；每个条目至少包含以下字段
  - key：string；在本文件中唯一
  - connect：string；连接器 id（定义在 connectors/source.d 中）
  - enable：bool；可选，默认 true
  - tags：array<string>；可选；形如 ["env:prod", "region=cn"]
  - params_override：table；可选；仅允许覆写连接器 `allow_override` 列表中的参数；支持别名 `params`

示例（最小，行内表写法）：

```toml
[[sources]]
key = "file_1"
enable = true
connect = "file_src"
tags = ["env:test"]
params_override = { base = "./src_dat", file = "gen.dat" }

[[sources]]
key  = "syslog_udp"
enable = true
connect = "syslog_udp_src"
tags = ["env:test", "service:syslog"]

[[sources]]
key  = "syslog_tcp"
enable = true
connect = "syslog_tcp_src"
params_override = { port = 1514, tcp_recv_bytes = 10485760 }
```

示例（块表写法，等价；便于多行与注释）：
```toml
[[sources]]
key = "file_1"
enable = true
connect = "file_src"

# 推荐：使用 params_override 块表
[sources.params_override]
base = "./src_dat"
file = "gen.dat"

[[sources]]
key = "file_2"
enable = true
connect = "file_src"

# 亦可使用别名 params（与 params_override 等价）
[sources.params]
base = "./src_dat"
file = "gen2.dat"
```

## 连接器定义（connectors/source.d/*.toml）

```toml
[[connectors]]
id = "file_src"
type = "file"
allow_override = ["base", "file", "encode"]
[connectors.params]
base = "./src_dat"
file = "gen.dat"
encode = "text"

[[connectors]]
id = "syslog_udp_src"
type = "syslog"
allow_override = ["addr", "port", "protocol", "tcp_recv_bytes", "header_mode", "prefer_newline"]
[connectors.params]
addr = "0.0.0.0"
port = 1514
protocol = "udp"
header_mode = "parse"
prefer_newline = false

[[connectors]]
id = "kafka_src"
type = "kafka"
allow_override = ["topic", "group_id", "config"]
[connectors.params]
brokers = "localhost:9092"
topic   = ["access_log"]
```

## 源类型与可覆写字段（示例）

### 1) 文件源（type = "file"）
- 连接器 params：`base`, `file`, `encode`
- 典型 allow_override：`["base", "file", "encode"]`

示例：
```toml
[[sources]]
key  = "file_access"
connect = "file_src"
enable = true
params_override = { base = "./src_dat", file = "access.log" }
```

块表写法（等价）：
```toml
[[sources]]
key  = "file_access"
connect = "file_src"
enable = true
[sources.params]
base = "./src_dat"
file = "access.log"
```

### 2) Syslog 源（type = "syslog"）
- 连接器 params：`addr`, `port`, `protocol`, `tcp_recv_bytes`, `header_mode`, `prefer_newline`
- 典型 allow_override：与 params 一致

说明：
- 自动识别并解析 RFC3164 和 RFC5424 格式；`header_mode` 控制是否进行剥离/解析/标签注入

示例（UDP）：
```toml
[[sources]]
key  = "syslog_udp"
connect = "syslog_udp_src"
enable = true
params_override = { header_mode = "parse", prefer_newline = true }
```

示例（TCP）：
```toml
[[sources]]
key  = "syslog_tcp"
connect = "syslog_tcp_src"
params_override = { port = 1514, tcp_recv_bytes = 10485760 }
```

### 3) Kafka 源（type = "kafka"，需启用 kafka 特性）
- 连接器 params：`brokers`, `topic`, `config`
- 典型 allow_override：`["topic", "group_id", "config"]`

示例：
```toml
[[sources]]
key  = "kafka_access"
connect = "kafka_src"
params_override = { topic = ["access_log"], config = ["auto.offset.reset=earliest"] }
```

## 标签与记录字段
- 数据进入解析管线后，源上的 tags 会被追加为记录字段（key/value），便于统计/路由/下游使用
- syslog 源在 header_mode="parse" 时，会自动注入下列字段：
  - syslog.pri、syslog.facility、syslog.severity

## 建议与常见问题
- 端口：非 root 环境建议使用 1514 等非特权端口；UDP 环境可能受沙箱限制（CI/macOS 等），无法绑定时请使用 TCP 做可重复测试
- 吞吐与稳定性：
  - UDP：建议控制发送速率（GEN_SPEED）与放大接收缓冲（仓库已设置 SO_RCVBUF），仍建议根据环境限速
  - TCP：支持 RFC6587 octet-counting 与换行分帧，稳定性好；tcp_recv_bytes 可按需调整
- 启停：临时关闭某个源：`enable = false`
 - 白名单：仅允许覆写连接器 `allow_override` 中列出的键，超出白名单将报错。
 - 写法选择：`params_override` 与 `params` 完全等价，配置中请选择其一；同一条目内不要混用行内表与块表来定义同一个表。

## 完整示例（混合多源）
```toml
[[sources]]
key  = "file_access"
connect = "file_src"
params_override = { base = "./src_dat", file = "access.log" }

[[sources]]
key  = "syslog_udp"
connect = "syslog_udp_main"
params_override = { header_mode = "parse", prefer_newline = true }

[[sources]]
key  = "syslog_tcp"
connect = "syslog_tcp_main"
params_override = { port = 1514, tcp_recv_bytes = 10485760 }

[[sources]]
key  = "kafka_access"
connect = "kafka_main"
params_override = { topic = ["access_log"] }
```

---

如需更多源类型/高级用法（例如 MQTT、HTTP、自定义扩展），可在 extensions/sources 下查看对应 crate 的 README 或在 issues 中提出需求。
