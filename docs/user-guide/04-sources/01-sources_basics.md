# 源配置基础

本文档介绍 warp-flow 系统中数据源配置的基础概念和用法。

## 概述

数据源（Source）是 warp-flow 系统的数据输入端，负责从各种数据源接收数据并输入到处理流程中。

## 核心概念

### 1. 连接器（Connectors）
连接器定义了如何与特定类型的数据源建立连接和通信。系统内置了多种连接器类型：

- **File Connector**: 从文件读取数据
- **Kafka Connector**: 从 Kafka 消息队列消费数据
- **Syslog Connector**: 接收 Syslog 协议数据
- **HTTP Connector**: 通过 HTTP 接收数据
- **TCP/UDP Connector**: 通过网络套接字接收数据

### 2. 配置结构

```toml
[source.main]
key = "main_source"
connect = "kafka_main"

[source.main.params]
topic = "events"
group_id = "consumer_group"
bootstrap_servers = ["localhost:9092"]
```

### 3. 连接器定义

连接器定义通常存储在 `connectors/source.d/` 目录下：

```toml
# connectors/source.d/kafka_main.toml
type = "kafka"
name = "kafka_main"

[params]
bootstrap_servers = ["localhost:9092"]
security_protocol = "PLAINTEXT"
```

## 配置步骤

1. **定义连接器**: 在 `connectors/source.d/` 目录下创建连接器配置文件
2. **配置源**: 在主配置文件中引用连接器并指定参数
3. **验证配置**: 使用 CLI 工具验证配置正确性
4. **启动系统**: 启动 warp-flow 系统开始接收数据

## 常用参数

### 通用参数
- `key`: 源的唯一标识符
- `connect`: 引用的连接器名称
- `params`: 连接器特定参数

### Kafka 源参数
- `topic`: 主题名称
- `group_id`: 消费者组ID
- `bootstrap_servers`: Kafka 服务器地址

### 文件源参数
- `path`: 文件路径
- `format`: 文件格式（json, csv, text等）
- `watch`: 是否监控文件变化

### Syslog 源参数
- `protocol`: 协议类型（tcp, udp）
- `port`: 监听端口
- `format`: 消息格式（rfc3164, rfc5424）

## 相关文档

- [文件源配置](./02-file_source.md)
- [Kafka 源配置](./03-kafka_source.md)
- [Syslog 源配置](./04-syslog_source.md)
- [连接器管理](./05-connectors.md)