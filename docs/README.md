---
page: product-landing
product: Warp Parse
lang: zh-CN
focus: [value, competitiveness, scenario_fit, adoption_cost]
cta: ["开始试用", "文档", "获取支持"]
---

# Warp Parse｜高性能日志/事件解析与接入引擎

一句话价值：以更低成本，把海量异构日志/事件快速、稳定、可观测地接入到你的数据平台与安全体系中。

- 面向对象：安全/平台/数据工程团队；对实时/准实时的日志解析与入库有强诉求的组织
- 典型目标：降低接入成本 ≥50%，稳定跑满带宽，毫秒级延迟可观测，按需横向扩展
- 许可协议：Apache‑2.0（商业可用）

## 价值主张（Value）

- 高性能低延迟：Tokio 异步 + 零拷贝 + 解析组合器；Mac M4 10C16G 基准中 Nginx 解析可达 **2,456,100 EPS / 559 MiB/s**、APT 3K 解析稳定 **1062 MiB/s**，端到端 TCP→File 解析+转换维持 **797,700 EPS**（详见 [performance.md](performance.md)）
- 解析与转换可编程：WPL（解析 DSL）+ OML（转换 DSL），复杂协议/格式也能快速落地
- 连接器统一：基于 `wp-connector-api` 的 Source/Sink 统一注册与扩展，避免接口割裂
- 极简部署与运维：单二进制应用，文件化配置；`wpkit` 自带 ping/校验工具链
- 企业场景友好：私有化交付、本地特性与规范（命名、目录、合规）优先

提示：性能与延迟受规则复杂度、目标端性能、网络/磁盘等因素影响；下文给出示例数据作为参考。

## 竞争力（Competitiveness）

与主流方案（Vector、Fluent Bit/Fluentd、Logstash、Java/Flink ETL）正面对比：

对比表（概览）

| 维度/方案           | Warp Parse                      | Vector                         | Fluent Bit/Fluentd           | Logstash                      | Java/Flink ETL                  |
| ------------------- | -------------------------------- | ------------------------------ | ---------------------------- | ----------------------------- | -------------------------------- |
| 吞吐/延迟           | 高吞吐、低延迟（示例>100MB/s） | 吞吐高、延迟低                 | 吞吐高，延迟低               | 吞吐中，延迟中                | 吞吐可高，延迟取决于算子/GC       |
| 解析/转换           | WPL/OML 可编程、规则易复用      | Remap/VRL 强，复杂协议需自扩展 | 解析基础，复杂度上限偏低     | Grok 等强，但性能与复杂度受限 | Java/DSL 灵活，工程成本高         |
| 连接器生态           | 统一 `wp-connector-api`，易扩展 | 官方生态丰富                   | 官方生态非常丰富             | 官方生态丰富                  | 依赖自研/生态，落地成本高          |
| 运维/交付           | 单体二进制，`wpkit` 提供诊断     | 单体二进制，工具完善            | 体积小，嵌入式友好           | 依赖 JVM，资源较重            | 依赖 JVM/Flink 集群，部署复杂      |
| 学习曲线             | 中（WPL/OML 1–3 天上手）        | 低~中（VRL 简洁）              | 低（配置驱动为主）           | 中（多插件/正则）             | 高（Java/算子/作业调度）          |
| 企业私有化与适配     | 友好，私有化/内网优先            | 友好                            | 友好                         | 一般                           | 一般（需大规模平台配合）          |
| 典型最佳场景         | 高速接入+强解析/转换+低运维      | 高速接入+通用路由              | 轻量边缘/Agent 嵌入          | 生态兼容/历史负载              | 复杂算子/全链路流处理             |

要点解读（逐一对比）

- 对 Vector：
  - 优势：解析/转换 DSL（WPL/OML）在复杂日志场景可读性与复用性更强；统一连接器 API 降低二次开发成本
  - 互补：Vector 生态与运维工具成熟，通用路由/转发场景非常稳；你的场景以通道/转发为主时可优先 Vector
- 对 Fluent Bit/Fluentd：
  - 优势：在复杂解析/转换与高吞吐并存的场景下更稳；DSL 组织大规模规则时更好维护
  - 互补：边缘侧/嵌入式/体积极简场景 FB 更合适
- 对 Logstash：
  - 优势：单体性能、资源占用与延迟；规则工程化与可测试性更强
  - 互补：历史生态/兼容需求明显时 Logstash 迁移成本更低
- 对 Java/Flink ETL：
  - 优势：接入与解析前置更轻、更快；无需集群/算子编排即可覆盖大多数“接入+解析+路由”场景
  - 互补：如需复杂有状态/窗口/联结等算子，建议与 Flink 等联合使用（Warp Parse 前置清洗/归一）

选型建议：
- 你的首要目标是“把多源日志高质量、低延迟地清洗入库”，且需要复杂解析/转换，优先 Warp Parse
- 轻量边缘/Agent 内置优先 Fluent Bit；通用转发/生态复用优先 Vector/Logstash
- 复杂流式计算选 Flink，Warp Parse 负责接入/解析并前置降噪

注意：不同产品定位与生态差异明显，请基于你的真实数据做 POC 评估（性能/稳定性/可维护性）。

## 应用场景匹配度（Scenario Fit）

适用：
- 安全日志实时接入与检测（Syslog、网络设备、主机 Agent、WAF、EDR）
- 业务日志结构化（Nginx/APP/API 网关），清洗后入 Kafka/ES/ClickHouse/湖仓
- 多源归档与在线检索（对象存储/数据湖）
- 异地同步/备份与灾备回放（容灾演练、合规留痕）

不适用或需结合其他方案：
- 复杂有状态计算/窗口化/时序联结 → 建议结合 Flink 等流处理引擎
- 批离线大作业/调度编排 → 建议结合 Airflow、Spark 等

## 采用成本（Adoption Cost）

- POC 周期：
  - 30 分钟：单机文件→Kafka/ES 跑通 + 指标可观测
  - 1 天：引入自有样本与规则，完成规范化输出与回放
  - 1 周：接入 2–3 个数据源，SLA/监控/告警跑通，出生产验收报告
- 学习成本：
  - 配置/连接器：0.5–1 天；WPL/OML：1–3 天（取决于规则复杂度）
- 资源成本（参考）：
  - 50MB/s 文本解析→Kafka：约 2C4G 即可稳定运行（示例值）
- 集成成本：
  - 与 Kafka/ES/ClickHouse 等目标端对接；CI/CD 镜像或二进制投产；日志与告警纳管
- 风险与边界：
  - 自定义连接器开发需要 Rust 能力；团队缺口可外包或使用现成连接器

## 性能基准（WarpParse vs Vector）

完整版请查看 [docs/performance.md](performance.md)；以下节选自 2024 年 Mac M4 Mini（10C/16G、macOS）的基准测试。

- 覆盖日志类型：Nginx Access (239B)、AWS ELB (411B)、Sysmon JSON (1KB)、APT Threat (3KB)。
- 测试拓扑：File→BlackHole、TCP→BlackHole、TCP→File；能力：解析与解析+转换共 24 组对比。
- WarpParse 在全部对比中领先：解析场景提升 **3.7x–11.9x**，解析+转换场景提升 **1.4x–11.3x**。

### 解析（Parse Only）吞吐
| 日志 (样本大小) | WarpParse File→BlackHole | Vector File→BlackHole | 吞吐提升 |
| --- | --- | --- | --- |
| Nginx 239B | 2,456,100 EPS / 559.81 MiB/s | 540,540 EPS / 123.20 MiB/s | **4.5x** |
| AWS ELB 411B | 1,012,400 EPS / 396.82 MiB/s | 158,730 EPS / 62.22 MiB/s | **6.4x** |
| Sysmon 1KB | 440,000 EPS / 413.74 MiB/s | 76,717 EPS / 72.14 MiB/s | **5.7x** |
| APT 3KB | 314,200 EPS / 1062.84 MiB/s | 33,614 EPS / 113.71 MiB/s | **9.3x** |

### 解析+转换（Parse + Transform）吞吐
| 日志 (样本大小) | WarpParse File→BlackHole | Vector File→BlackHole | 吞吐提升 |
| --- | --- | --- | --- |
| Nginx 239B | 1,749,200 EPS / 398.69 MiB/s | 470,312 EPS / 107.20 MiB/s | **3.7x** |
| AWS ELB 411B | 710,400 EPS / 278.45 MiB/s | 129,743 EPS / 50.85 MiB/s | **5.5x** |
| Sysmon 1KB | 354,800 EPS / 333.63 MiB/s | 58,200 EPS / 54.73 MiB/s | **6.1x** |
| APT 3KB | 280,000 EPS / 947.15 MiB/s | 30,612 EPS / 103.55 MiB/s | **9.1x** |

- 端到端 TCP→File 场景下 WarpParse 解析吞吐 **1,084,600 EPS / 247 MiB/s**（Vector 91,200 EPS），解析+转换也能保持 **797,700 EPS / 181 MiB/s**（Vector 70,800 EPS）。
- 规则体积显著更小：例如 Nginx 解析规则 174B vs 416B，APT 解析规则 985B vs 1759B，利于快速下发与热更新。
- 资源特性：WarpParse 以更高 CPU 换取吞吐，但完成同等数据的总 CPU 时间更短；APT 场景可稳定在 **1062 MiB/s**，TCP→File 大包场景峰值内存约 1–2GB。
- 结果依赖规则复杂度、目标端性能与网络/磁盘条件，请结合自有数据做 POC 验证。

## 快速集成（Checklist）

- 连接器需求：file/syslog/kafka/tcp（源），file/syslog/kafka/mysql/ES/ClickHouse/Prometheus（汇）
- 负载目标：例如 50–200MB/s 文本解析；是否需要低延迟（毫秒级）
- 规则复杂度：WPL/OML 的覆盖面与可维护性
- 环境与合规：私有化部署/内网访问/日志与数据主权

## 立即开始（Try Now）

- 构建与运行：
  ```bash
  cargo build                   # 默认启用 community（kafka/mysql）
  cargo run --bin wparse -- --help
  cargo run --bin wpgen  -- --help
  cargo run --bin wpkit  -- --help
  ```
- 文档：`docs/`（mdBook），常用入口：
  - 源配置：docs/10-user/04-sources/README.md
  - 汇配置：docs/10-user/03-sinks/README.md
  - WPL：docs/10-user/06-wpl/01-wpl_basics.md
  - OML：docs/10-user/07-oml/01-oml_basics.md

## 常见问答（FAQ）

- 与 Vector/Fluent Bit 有何差异？
  - 更强调“解析/转换 DSL + 连接器统一 API + 极简交付”；功能上互补，聚焦接入前置环节
- 是否支持集群？
  - 通过多进程横向扩展；结合上游（Kafka）做分区级扩展；内部具备背压与断点续传能力
- 是否需要写代码？
  - 常见场景仅需写配置与 WPL/OML 规则；自定义连接器/高级功能可用 Rust 开发

---

版权与许可：Apache‑2.0
