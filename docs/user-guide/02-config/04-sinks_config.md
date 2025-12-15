# Sink 配置指南

## 概览
- 标识规则
  - 组名：sink_group.name（例如 /sink/example/simple）
  - sink 名：name（组内唯一；未显式提供时按索引回退为 [0]/[1]/…）
  - full_name：运行期与展示统一标识，full_name = sink_group.name + "/" + name（例如 /sink/example/simple/dat）
- 展示输出
  - `wproj stat file` 与 `wproj validate sink-file` 的 Sink/组列均显示 full_name
- 过滤语义（filter）
  - filter 是“拦截条件”：表达式求值为 true 时，该条数据不写入该 sink，而是转发到基础组 intercept（framework/intercept）
  - 每个 sink 可独立设置 filter；与 expect 相互独立

## 目录与文件组织
- sink_root：用例内通常为 <case>/sink
  - business.d/**/*.toml：业务组路由（场景输出，支持子目录）
  - infra.d/**/*.toml：基础组路由（default/miss/residue/intercept/error/monitor，支持子目录）
  - defaults.toml：默认组级期望 [defaults.expect]
  - connectors/sink.d/*.toml：连接器定义（loader 自 sink_root 向上查找最近的该目录）


## 路由文件格式
- 顶层
  - version（可选）
  - sink_group
    - name：组名（字符串）
    - oml / rule：推荐扁平写法；均可为字符串或字符串数组；用于匹配模型或规则。
      - 兼容：保留旧写法 `match = { oml=[…], rule=[…] }`（读取时自动合并，顶层优先）。
    - expect：可选，组级期望（覆盖 defaults）
    - sinks：数组，每项为单个 sink 定义
- 单个 sink 字段
  - name：该 sink 的名称（组内唯一）；未提供则按 [index] 回退
  - connect：引用连接器 id（兼容读取 `use`/`connector`）
  - params：对连接器默认参数的白名单覆盖（keys 必须在连接器 allow_override 列表中）
  - expect：可选，单 sink 期望（仅 ratio/tol/min/max，互斥关系：ratio/tol 与 min/max 不可混用）
  - filter：可选，拦截条件文件路径；命中 true 时丢弃该 sink 并发送至 intercept

## 连接器（connectors/sink.d）与 fmt 归属
- 统一在 connector 决定“写出格式”（fmt）；route 不再建议设置 fmt。
- 文件类（type = "file"）：支持 `raw/json/kv/proto-text` 四种；在 connector.params 指定 `fmt` 与 `base`/`file`；需要多种格式时，定义多个文件连接器（如 file_kv_sink/file_json_sink/...）。
- 非文件类（kafka/mysql/clickhouse/elasticsearch/syslog/prometheus）：格式由后端决定（例如 kafka 通常 JSON）；若在 route 写了 fmt 将被忽略并给出提示。
- 连接器不再需要 `enable` 字段：存在即启用；是否“使用中”以是否被 route 引用为准。

示例（connectors/sink.d）
```toml
[[connectors]]
id = "file_kv_sink"
type = "file"
allow_override = ["base", "file"]
[connectors.params]
fmt  = "kv"
base = "./out"
file = "default.dat"

[[connectors]]
id = "kafka_sink"
type = "kafka"
[connectors.params]
brokers = "kafka:9092"
topic   = "topic_a"
```

配置示例：业务组（filter）
```toml
version = "2.0"

[sink_group]
name = "/sink/filter"
oml  = ["/oml/sh*"]

[[sink_group.sinks]]
name = "all"
connect = "file_kv_sink"
params = { base = "./out/sink", file = "all.dat" }

[[sink_group.sinks]]
name = "safe"
connect = "file_kv_sink"
filter = "./sink/business.d/filter.conf"   # 命中 -> 拦截，不写 safe
params = { base = "./out/sink", file = "safe.dat" }
```

配置示例：基础组（infra）
```toml
version = "2.0"
[sink_group]
name = "intercept"

[[sink_group.sinks]]
name = "intercept"
connect = "file_kv_sink"
params = { base = "./out", file = "intercept.dat" }
```

相关 CLI（用于校验/展示）
- 请参阅：`../03-sinks/validate_route.md`

校验提示
- 分母决定：
  - basis = total_input：总输入
  - basis = group_input：该组各 sink 行数之和（或 stats 中该组输入）
  - basis = model：按模型粒度统计（目前以组内 sinks 行数之和替代）
- min_samples：当分母为 0 或小于该值时，组校验被忽略（打印提示，不 fail）
- 当 route 为非文件类写入 fmt 时，validate 会提示“fmt 由后端决定，已忽略”。

## 常见排错
- 连接器未找到：检查 connectors/sink.d 是否存在对应 id；`wproj sinks list` 可查看引用关系
- 覆盖参数不生效：检查 allow_override 白名单
- filter 未生效：
  - 路径解析相对当前工作目录（建议写相对 sink_root 的相对路径）
  - 日志中会打印“found path/not found filter …”
  - 表达式语法需通过 TCondParser；可先用简单表达式试验
