# Sinks 配置指南

本文档提供 warp-flow 系统数据输出端（Sinks）的完整使用指南。

## 内容概览

- [Sink 配置基础](./01-sinks_basics.md)
- [文件 Sink 配置](./02-file_sink.md)
- [Syslog Sink 配置](./03-syslog_sink.md)
- [Prometheus Sink 配置](./04-prometheus_sink.md)
- [Sinks 路由](./routing.md)
- [Sinks 并行与分片](./parallel_and_sharding.md)
- [Sinks 连接器（Connector）](./connectors.md)
- [defaults 与 expect](./defaults_expect.md)
- [Sinks CLI：Validate](./validate_route.md)

## 快速开始

1. 了解 [Sink 配置基础概念](./01-sinks_basics.md)
2. 根据输出需求选择合适的 Sink 类型：
   - [文件 Sink](./02-file_sink.md) - 输出到本地文件
   - [Syslog Sink](./03-syslog_sink.md) - 输出到 Syslog 服务器
   - [Prometheus Sink](./04-prometheus_sink.md) - 输出监控指标
3. 配置连接器和路由规则
4. 使用 [Sinks CLI](./validate_route.md) 验证配置

## 相关文档

- [配置指南概述](../02-config/README.md)
- [Sources 配置指南](../04-sources/README.md)
- [性能优化指南](../05-performance/README.md)