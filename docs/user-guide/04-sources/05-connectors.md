# 连接器管理

本文档详细介绍 warp-flow 系统中源连接器的管理、创建和维护。

## 连接器概念

### 什么是连接器
连接器是数据源的配置模板，定义了特定类型数据源的默认参数和行为。通过将连接器定义与源实例配置分离，实现了配置的复用和统一管理。

### 连接器的作用
1. **配置复用**: 多个源可以引用同一个连接器
2. **参数标准化**: 统一同类数据源的配置规范
3. **权限控制**: 通过 `allow_override` 控制可覆盖的参数
4. **版本管理**: 便于连接器配置的版本控制

## 连接器定义结构

### 基础结构
```toml
# connectors/source.d/{connector_name}.toml
[[connectors]]
id = "unique_connector_id"
type = "connector_type"
allow_override = ["param1", "param2", "param3"]

[connectors.params]
param1 = "default_value1"
param2 = "default_value2"
param3 = "default_value3"
```

### 字段说明

#### id (必需)
- 连接器的唯一标识符
- 在源配置中通过 `connect` 字段引用
- 命名规范：使用描述性名称，如 `file_src`, `kafka_production`

#### type (必需)
- 连接器类型，决定使用哪种数据源实现
- 支持的类型：`file`, `kafka`, `syslog`, `http`, `tcp`, `udp`

#### allow_override (可选)
- 允许源配置覆盖的参数列表
- 为空时表示不允许覆盖任何参数
- 提供配置灵活性，同时保证安全性

#### params (必需)
- 连接器的默认参数配置
- 被 `allow_override` 包含的参数可以在源配置中覆盖

## 连接器类型详解

### 1. File 连接器
```toml
# connectors/source.d/00-file-default.toml
[[connectors]]
id = "file_src"
type = "file"
allow_override = ["base", "file", "encode"]

[connectors.params]
base = "./data/in_dat"
file = "gen.dat"
encode = "text"
```

**参数说明**:
- `base`: 基础目录路径
- `file`: 文件名
- `encode`: 编码格式 (`text`|`base64`|`hex`)
- `path`: 直接文件路径 (与 base+file 二选一)

### 2. Kafka 连接器
```toml
# connectors/source.d/kafka_production.toml
[[connectors]]
id = "kafka_production"
type = "kafka"
allow_override = ["topic", "group_id", "config"]

[connectors.params]
brokers = ["kafka1:9092", "kafka2:9092", "kafka3:9092"]
security_protocol = "SASL_SSL"
sasl_mechanisms = "PLAIN"
topic = ["default_topic"]
```

**参数说明**:
- `brokers`: Kafka 集群地址列表
- `topic`: 默认主题列表
- `security_protocol`: 安全协议
- `sasl_mechanisms`: SASL 认证机制
- `config`: Kafka 客户端配置

### 3. Syslog 连接器
```toml
# connectors/source.d/syslog_tcp.toml
[[connectors]]
id = "syslog_tcp_src"
type = "syslog"
allow_override = ["addr", "port", "protocol", "header_mode", "prefer_newline"]

[connectors.params]
addr = "127.0.0.1"
port = 1514
protocol = "tcp"
header_mode = "parse"
prefer_newline = false
tcp_recv_bytes = 10485760
```

**参数说明**:
- `addr`: 监听地址
- `port`: 监听端口
- `protocol`: 传输协议 (`udp`|`tcp`)
- `header_mode`: 头部处理模式（`keep|strip|parse`）
- `prefer_newline`: 优先按换行进行分帧
- `tcp_recv_bytes`: TCP 接收缓冲区大小

## 连接器组织结构

### 目录结构
```
connectors/source.d/
├── 00-file-default.toml      # 默认文件连接器
├── 10-syslog-udp.toml        # UDP Syslog 连接器
├── 11-syslog-tcp.toml        # TCP Syslog 连接器
├── 30-kafka.toml             # Kafka 连接器
├── 40-http.toml              # HTTP 连接器
└── custom/                   # 自定义连接器目录
    ├── special-file.toml
    └── legacy-system.toml
```

### 命名规范
- 完整规则见《[连接器命名规范](../../10-user/02-config/connectors_naming.md)》。
- 快速摘录：
  - 文件名：`NN-<kind>-<variant>.toml`（示例：`00-file-default.toml`、`10-syslog-udp.toml`）
  - id：`<kind>_<variant>_src`（示例：`file_src`、`syslog_udp_src`）
  - 字段：统一使用 `type` 指明种类，`allow_override` 控制白名单

## 连接器创建指南

### 1. 环境分离策略
```toml
# connectors/source.d/file_dev.toml
[[connectors]]
id = "file_read_dev"
type = "file"
allow_override = ["base", "file", "encode"]
[connectors.params]
base = "./dev_data"
file = "sample.log"
encode = "text"

# connectors/source.d/file_prod.toml
[[connectors]]
id = "file_read_prod"
type = "file"
allow_override = ["base", "file", "encode"]
[connectors.params]
base = "/var/log/production"
file = "access.log"
encode = "text"
```

### 2. 服务分离策略
```toml
# connectors/source.d/kafka_logs.toml
[[connectors]]
id = "kafka_logs_cluster"
type = "kafka"
allow_override = ["topic", "group_id"]
[connectors.params]
brokers = ["kafka-log1:9092", "kafka-log2:9092"]
topic = ["logs"]
security_protocol = "SASL_SSL"

# connectors/source.d/kafka_events.toml
[[connectors]]
id = "kafka_events_cluster"
type = "kafka"
allow_override = ["topic", "group_id"]
[connectors.params]
brokers = ["kafka-event1:9092", "kafka-event2:9092"]
topic = ["events"]
security_protocol = "SASL_SSL"
```

### 3. 性能分级策略
```toml
# connectors/source.d/syslog_standard.toml
[[connectors]]
id = "syslog_standard"
type = "syslog"
allow_override = ["addr", "port", "protocol"]
[connectors.params]
tcp_recv_bytes = 10485760  # 10MB

# connectors/source.d/syslog_high_perf.toml
[[connectors]]
id = "syslog_high_perf"
type = "syslog"
allow_override = ["addr", "port", "protocol"]
[connectors.params]
tcp_recv_bytes = 104857600  # 100MB
```

## 连接器最佳实践

### 1. 参数覆盖设计
```toml
# ✅ 好的设计：明确的覆盖权限
[[connectors]]
id = "file_main"
type = "file"
allow_override = ["base", "file", "encode"]

# ❌ 避免：过度开放覆盖权限
[[connectors]]
id = "file_too_open"
type = "file"
allow_override = ["*"]  # 不安全

# ❌ 避免：完全不开放覆盖权限
[[connectors]]
id = "file_too_strict"
type = "file"
allow_override = []  # 不灵活
```

### 2. 默认值设置
```toml
# ✅ 好的设计：合理的默认值
[[connectors]]
id = "syslog_secure"
type = "syslog"
allow_override = ["addr", "port"]
[connectors.params]
addr = "127.0.0.1"        # 安全的默认地址
port = 1514               # 非特权端口
protocol = "tcp"          # 可靠的协议
header_mode = "parse"     # 处理协议头：解析 + 注入元标签 + 剥离
prefer_newline = false    # 默认按长度前缀优先；换行优先可在纯换行流量中更高效
```

### 3. 文档和注释
```toml
# connectors/source.d/kafka_main.toml
# 生产环境 Kafka 集群连接器
# 支持多主题消费和 SASL 认证
[[connectors]]
id = "kafka_main"
type = "kafka"
allow_override = [
    "topic",          # 允许覆盖主题列表
    "group_id",       # 允许覆盖消费者组
    "config"          # 允许覆盖客户端配置
]

[connectors.params]
# Kafka 集群地址
brokers = ["kafka1:9092", "kafka2:9092", "kafka3:9092"]

# 安全配置
security_protocol = "SASL_SSL"
sasl_mechanisms = "PLAIN"

# 默认主题
topic = ["default_events"]
```

## 连接器管理操作

### 1. 查看连接器
```bash
# 列出所有连接器
wpgen connectors list

# 查看特定连接器详情
wpgen connectors show kafka_main

# 验证连接器配置
wpgen connectors validate
```

### 2. 连接器测试
```bash
# 测试连接器连接性
wpgen connectors test kafka_main

# 测试连接器参数
wpgen connectors test --params topic=test_events file_src
```

### 3. 连接器更新
```bash
# 更新连接器配置
wpgen connectors update file_main --param base=/new/path

# 重新加载连接器
wpgen connectors reload
```

## 故障排除

### 常见问题

#### 1. 连接器 ID 重复
```
Error: Duplicate connector ID 'file_main'
```
**解决方案**:
- 检查所有连接器文件中的 ID 唯一性
- 使用描述性且唯一的 ID

#### 2. 参数覆盖错误
```
Error: Parameter 'file' not in allow_override list
```
**解决方案**:
- 将需要覆盖的参数添加到 `allow_override` 列表
- 或者移除源配置中的参数覆盖

#### 3. 连接器类型不存在
```
Error: Unknown connector type 'redis'
```
**解决方案**:
- 检查连接器类型拼写
- 确认系统支持该类型
- 查看支持的连接器类型列表

### 调试技巧

#### 1. 配置验证
```bash
# 完整的配置验证
wpgen source validate wpsrc.toml --verbose
```

#### 2. 连接器调试
```bash
# 启用调试日志
RUST_LOG=debug wpgen connectors list
```

#### 3. 参数解析测试
```bash
# 测试参数解析
wpgen connectors parse --id kafka_main --param topic=test
```

## 高级特性

### 1. 连接器继承
虽然系统不支持直接的连接器继承，但可以通过配置复用实现类似效果：

```toml
# base 连接器
[[connectors]]
id = "kafka_base"
type = "kafka"
allow_override = ["topic", "group_id"]
[connectors.params]
brokers = ["kafka1:9092", "kafka2:9092"]
security_protocol = "SASL_SSL"

# 专用连接器
[[connectors]]
id = "kafka_logs"
type = "kafka"
allow_override = ["topic", "group_id"]
[connectors.params]
brokers = ["kafka1:9092", "kafka2:9092"]
security_protocol = "SASL_SSL"
topic = ["logs"]
```

### 2. 环境变量支持
在连接器配置中使用环境变量：

```toml
# connectors/source.d/kafka_env.toml
[[connectors]]
id = "kafka_env"
type = "kafka"
allow_override = ["topic", "group_id"]
[connectors.params]
brokers = "${KAFKA_BROKERS}"
security_protocol = "${KAFKA_SECURITY_PROTOCOL}"
```

### 3. 连接器模板
创建连接器模板，便于快速创建新的连接器：

```bash
# 创建基于模板的连接器
wpgen connectors create --template kafka --name kafka_custom --params brokers=custom:9092
```

## 相关文档

- [源配置基础](./01-sources_basics.md)
- [文件源配置](./02-file_source.md)
- [Kafka 源配置](./03-kafka_source.md)
- [Syslog 源配置](./04-syslog_source.md)
