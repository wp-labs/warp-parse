# Kafka 源配置

本文档详细介绍如何配置和使用 warp-flow 系统的 Kafka 数据源。

## 概述

Kafka 源用于从 Apache Kafka 消息队列消费数据，支持多个主题、消费者组和灵活的配置选项。

## 连接器定义

### 基础 Kafka 连接器

```toml
# connectors/source.d/kafka_main.toml
[[connectors]]
id = "kafka_main"
type = "kafka"
allow_override = ["topic", "group_id", "config"]

[connectors.params]
brokers = "localhost:9092"
topic = ["access_log"]
```

### 高可用 Kafka 连接器

```toml
# connectors/source.d/kafka_ha.toml
[[connectors]]
id = "kafka_ha_cluster"
type = "kafka"
allow_override = ["topic", "group_id", "config"]

[connectors.params]
brokers = ["kafka1:9092", "kafka2:9092", "kafka3:9092"]
security_protocol = "SASL_SSL"
sasl_mechanisms = "PLAIN"
sasl_username = "consumer_user"
sasl_password = "consumer_pass"
topic = ["events", "logs", "metrics"]
```

## 支持的参数

### 基础连接参数

#### brokers (必需)
Kafka 集群地址列表

```toml[sources.params_override]
brokers = "localhost:9092"
# 或者多节点
brokers = ["kafka1:9092", "kafka2:9092", "kafka3:9092"]
```

#### topic (必需)
消费的主题列表

```toml
[sources.params_override]
topic = ["access_log", "error_log", "audit_log"]
```

#### group_id (推荐)
消费者组ID

```toml
[sources.params_override]
group_id = "log_processor_group"
```

### 安全配置

#### SSL/TLS 配置
```toml
[sources.params_override.config]
security_protocol = "SSL"
ssl_ca_location = "/path/to/ca.pem"
ssl_certificate_location = "/path/to/client.pem"
ssl_key_location = "/path/to/client.key"
ssl_key_password = "key_password"
```

#### SASL 认证
```toml
[sources.params_override.config]
security_protocol = "SASL_PLAINTEXT"
sasl_mechanisms = "PLAIN"
sasl_username = "consumer_user"
sasl_password = "consumer_pass"
```

#### SASL/SCRAM 认证
```toml
[sources.params_override.config]
security_protocol = "SASL_SCRAM-SHA-256"
sasl_mechanisms = "SCRAM-SHA-256"
sasl_username = "consumer_user"
sasl_password = "consumer_pass"
```

### 高级配置

#### 消费策略
```toml
[sources.params_override.config]
auto_offset_reset = "earliest"  # 或 "latest"
enable_auto_commit = "false"
auto_commit_interval_ms = "5000"
```

#### 会话和心跳配置
```toml
[sources.params_override.config]
session_timeout_ms = "30000"
heartbeat_interval_ms = "3000"
max_poll_interval_ms = "300000"
```

#### 批量消费配置
```toml
[sources.params_override.config]
max_poll_records = "500"
fetch_min_bytes = "1"
fetch_max_wait_ms = "500"
```

## 配置示例

### 基础配置
```toml
# wpsrc.toml
[[sources]]
enable = true
key = "kafka_access_logs"
connect = "kafka_main"
tags = ["env:production", "type:access_log"]

[sources.params_override]
topic = ["nginx_access_log"]
group_id = "nginx_log_processor"
```

### 多主题配置
```toml
# wpsrc.toml
[[sources]]
enable = true
key = "kafka_multi_logs"
connect = "kafka_main"
tags = ["env:production", "type:multi_log"]

[sources.params_override]
topic = ["access_log", "error_log", "audit_log"]
group_id = "unified_log_processor"

[sources.params_override.config]
auto_offset_reset = "earliest"
enable_auto_commit = "false"
```

### 安全集群配置
```toml
# wpsrc.toml
[[sources]]
enable = true
key = "kafka_secure_logs"
connect = "kafka_ha_cluster"
tags = ["env:production", "security:tls"]

[sources.params_override]
topic = ["secure_events"]
group_id = "secure_event_processor"

[sources.params_override.config]
auto_offset_reset = "latest"
enable_auto_commit = "true"
auto_commit_interval_ms = "1000"
```

### 开发环境配置
```toml
# wpsrc.toml
[[sources]]
enable = true
key = "kafka_dev_logs"
connect = "kafka_main"
tags = ["env:development", "team:backend"]

[sources.params_override]
brokers = "dev-kafka:9092"
topic = ["dev_events"]
group_id = "dev_processor"
```

## 数据处理特性

### 1. 消息结构
每个 Kafka 消息被转换为数据包，包含：
- **消息体**: 消息的实际内容
- **元数据**:
  - `topic`: 消息来源主题
  - `partition`: 分区号
  - `offset`: 消息偏移量
  - `timestamp`: 消息时间戳
  - `key`: 消息键 (如果存在)

### 2. 自动标签添加
```json
{
  "data": "原始消息内容",
  "tags": {
    "source_type": "kafka",
    "kafka_topic": "access_log",
    "kafka_partition": 0,
    "kafka_offset": 1234,
    "kafka_timestamp": 1640995200000
  }
}
```

### 3. 消费语义
- **至少一次**: 默认保证，可能重复
- **精确一次**: 需要外部系统支持事务
- **自动提交**: 可配置自动或手动提交偏移量

## 性能优化

### 1. 批量消费
```toml
[sources.params_override.config]
max_poll_records = "1000"        # 增加批处理大小
fetch_min_bytes = "1024"         # 最小抓取字节数
fetch_max_wait_ms = "100"        # 最大等待时间
```

### 2. 连接优化
```toml
[sources.params_override.config]
session_timeout_ms = "60000"     # 增加会话超时
heartbeat_interval_ms = "5000"   # 调整心跳间隔
max_poll_interval_ms = "600000"  # 增加最大轮询间隔
```

### 3. 内存管理
```toml
[sources.params_override.config]
queued_min_messages = "100000"   # 队列最小消息数
queued_max_messages_kbytes = "1048576"  # 队列最大大小(1GB)
```

## 监控和指标

### 1. 内置指标
Kafka 源自动收集以下指标：
- 消费速率 (messages/second)
- 消费延迟 (lag)
- 连接状态
- 错误率

### 2. 标签监控
```toml
tags = [
    "monitor:kafka_source",
    "alert_on_lag:true",
    "alert_on_error:true"
]
```

## 故障排除

### 常见问题

#### 1. 连接失败
```
Error: Failed to connect to Kafka cluster
```
**解决方案**:
- 检查 broker 地址是否正确
- 验证网络连通性
- 确认 Kafka 集群状态

#### 2. 认证失败
```
Error: SASL authentication failed
```
**解决方案**:
- 检查用户名密码
- 验证 SASL 机制配置
- 确认用户权限

#### 3. 主题不存在
```
Error: Topic not found
```
**解决方案**:
- 确认主题名称拼写
- 检查主题是否存在
- 验证主题访问权限

#### 4. 消费者组冲突
```
Error: Consumer group rebalance failed
```
**解决方案**:
- 检查消费者组配置
- 确认组ID唯一性
- 调整会话超时时间

### 调试技巧

#### 1. 验证连接
```bash
# 使用 kafka 命令行工具测试
kafka-console-consumer.sh --bootstrap-server localhost:9092 --topic test_topic --from-beginning
```

#### 2. 检查消费者组状态
```bash
kafka-consumer-groups.sh --bootstrap-server localhost:9092 --describe --group your_group_id
```

#### 3. 启用详细日志
```bash
RUST_LOG=debug wpgen source start wpsrc.toml
```

## 最佳实践

### 1. 消费者组规划
- 为不同类型的数据使用不同的消费者组
- 避免多个系统共享同一个消费者组
- 使用有意义的消费者组名称

### 2. 主题管理
```toml
# 生产环境
topic = ["prod_access_log_v1", "prod_error_log_v1"]

# 开发环境
topic = ["dev_access_log", "dev_debug_log"]
```

### 3. 错误处理
```toml
tags = [
    "retry_on_error:true",
    "max_retry:3",
    "dead_letter_topic:error_events"
]
```

### 4. 监控集成
```toml
tags = [
    "metrics:prometheus",
    "alert_lag_threshold:1000",
    "alert_error_rate:0.01"
]
```

## 相关文档

- [源配置基础](./01-sources_basics.md)
- [文件源配置](./02-file_source.md)
- [Syslog 源配置](./04-syslog_source.md)
- [性能优化指南](../05-performance/README.md)