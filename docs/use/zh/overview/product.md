# Warp Parse 产品概览

## 产品定位

Warp Parse 是面向日志、事件和安全数据接入场景的高性能 Rust 引擎，重点覆盖：

- 多源数据接入
- 高吞吐解析与转换
- 基于配置和规则的路由
- 单二进制部署和运维

典型使用方包括安全平台、可观测性平台、数据平台和实时风控团队。

## 核心价值

- 高吞吐低延迟：适合实时或准实时日志解析与接入
- 规则可编程：通过 WPL 和 OML 组织解析与转换逻辑
- 连接器统一：Source 和 Sink 采用统一扩展模型
- 运维简单：以文件配置和命令行工具为核心
- 私有化友好：适合内网和合规要求较强的环境

## 适用场景

- 安全日志接入与标准化
- Nginx、应用日志、API 网关日志结构化
- Kafka、ES、ClickHouse 等目标前置清洗
- 灾备、归档、离线回放前的数据规整

## 不适合单独承担的场景

- 复杂有状态流式计算
- 大规模窗口聚合和联结
- 批处理编排型离线作业

这些场景通常需要与 Flink、Spark、Airflow 等系统组合使用。

## 主要组件

- `wparse`: 主运行时，负责 batch 或 daemon 模式执行
- `wpgen`: 规则和配置辅助生成工具
- `wproj`: 项目管理、校验、运行时管理入口
- `wprescue`: rescue 数据处理工具

## 快速开始

查看命令帮助：

```bash
wparse --help
wpgen --help
wproj --help
wprescue --help
```

## 相关文档

- 运行时管理面使用说明: [../operations/admin.md](../operations/admin.md)
- 远程工程拉取与规则热更新 SOP: [../operations/project-sync.md](../operations/project-sync.md)
- 使用文档索引: [../README.md](../README.md)
- 对应英文版: [../../en/overview/product.md](../../en/overview/product.md)
