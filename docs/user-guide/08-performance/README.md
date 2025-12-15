# 性能优化指南

本指南介绍如何优化 warp-flow 系统的性能，包括数据源、数据处理和输出端的优化策略。

## 内容概览

- [性能概览](./01-performance_overview.md)
- [数据源性能优化](./02-source_performance.md)
- [数据处理性能](./03_processing_performance.md)
- [Sink 性能优化](./04-sink_performance.md)
- [内存管理](./05_memory_management.md)
- [并发和并行](./06_concurrency.md)
- [监控和指标](./07_monitoring.md)
- [性能测试](./08_benchmarking.md)

## 性能优化要点

### 1. 数据源优化
- 选择合适的连接器类型
- 配置适当的缓冲区大小
- 使用批量处理模式

### 2. 处理优化
- 合理设置并行度
- 优化过滤和转换逻辑
- 使用内存池减少 GC 压力

### 3. 输出优化
- 配置合适的批量大小
- 使用异步输出模式
- 优化网络传输

## 相关文档

- [Sinks 并行与分片](../03-sinks/parallel_and_sharding.md)
- [源配置指南](../04-sources/README.md)
- [监控和指标参考](../../80-reference/README.md)