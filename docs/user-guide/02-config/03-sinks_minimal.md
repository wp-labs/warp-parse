# Sinks 最小可运行骨架

目标：提供一份可复制的最小 现行 Sinks 目录骨架（connectors/sink.d + models/sinks/{business.d,infra.d} + defaults.toml），支持文件类 sink 输出与 CLI 校验/展示。

目录结构（相对工作目录）
```
connectors/
  sink.d/
    02-file-json.toml
models/
  sinks/
    business.d/
      demo.toml
    infra.d/
      default.toml
    defaults.toml
```

1) 连接器（connectors/sink.d/02-file-json.toml）
```toml
[[connectors]]
id = "file_json_sink"
type = "file"
allow_override = ["base","file","path"]
[connectors.params]
fmt  = "json"
base = "./data/out_dat"
file = "default.json"
```

2) 业务路由（models/sinks/business.d/demo.toml）
```toml
version = "2.0"

[sink_group]
name = "demo"
oml  = []
tags = ["biz:demo"]

[[sink_group.sinks]]
name = "json"
connect = "file_json_sink"
params = { file = "demo.json" }
tags = ["sink:json"]
```

3) 基础/兜底（models/sinks/infra.d/default.toml）
```toml
version = "2.0"

[sink_group]
name = "default"

[[sink_group.sinks]]
name = "json"
connect = "file_json_sink"
params = { file = "default.json" }
```
提示：基础组不支持 `parallel` 与文件分片；如需并发/分片，请在业务组配置。

4) 默认项（models/sinks/defaults.toml）
```toml
version = "2.0"

[defaults]
tags = ["env:dev"]

[defaults.expect]
basis = "total_input"
mode  = "warn"
```

校验与展示
```bash
# 校验 connectors/routes
wproj sinks validate --work-root .

# 列出 connectors 与被引用关系
wproj sinks list --work-root .

# 展示路由解析结果（目标路径、fmt 等）
wproj sinks route --work-root .
```

运行提示
- 文件类 sink 输出文件位于 `./data/out_dat/` 下；业务组 demo 的 `json` sink 输出为 `demo.json`。
- 可依据需要在 business.d 中添加更多 sinks 或组；infra.d 中可按固定组名（default/miss/residue/intercept/error/monitor）扩展兜底流向。
- tags 合并顺序：defaults -> group -> sink；仅结构化路径注入标签（Raw 不注入）。
