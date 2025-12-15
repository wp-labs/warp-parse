# 文件 Sink 配置

本文档介绍文件型 Sink 的现行配置与能力，已与代码实现对齐。

## 概述

文件 Sink 将处理后的数据写入本地文件系统，支持多种输出格式和灵活的路径配置。常用于离线验收、归档与调试。

支持的输出格式（`fmt`）：`json`、`csv`、`kv`、`raw`、`proto`、`proto-text`（默认 `json`）。

## 连接器定义

推荐直接使用仓库自带模板（位于 `connectors/sink.d/`）：

```toml
# JSON
[[connectors]]
id = "file_json_sink"
type = "file"
allow_override = ["base","file"]
[connectors.params]
fmt  = "json"
base = "./data/out_dat"
file = "default.json"

# Prototext
[[connectors]]
id = "file_proto_sink"
type = "file"
allow_override = ["base","file"]
[connectors.params]
fmt  = "proto-text"
base = "./data/out_dat"
file = "default.dat"

# Raw
[[connectors]]
id = "file_raw_sink"
type = "file"
allow_override = ["base","file"]
[connectors.params]
fmt  = "raw"
base = "./data/out_dat"
file = "default.raw"
```

如需在路由中覆写完整路径或分片相关键，请在自有连接器中扩展白名单：`"path"`、`"fmt"`、`"file_template"`、`"replica_shard"`。

## 可用参数（路由 `params`）

- `base` + `file`：目标目录与文件名（推荐写法）。
- `path`：完整路径（二选一，提供 `path` 时忽略 `base+file`）。
- `fmt`：输出格式（见上）。
- `replica_shard`：布尔；当组并行 `parallel>1` 时，若为 `true` 且使用 `base+file`，自动产出 `name_0.ext/name_1.ext/...`。
- `file_template`：自定义命名模板，支持 `{replica}`（从 0）、`{replica1}`（从 1）、`{file}`（原文件名）。

说明：文件 Sink 会自动创建父目录；内部使用缓冲写入并按批次刷新，无“手动缓冲大小/同步模式”等参数。

## 配置示例

1) 基础 JSON 输出
```toml
# business.d/json_output.toml
version = "2.0"
[sink_group]
name = "/sink/json_output"
oml  = ["logs"]

[[sink_group.sinks]]
name = "json"
connect = "file_json_sink"
params = { base = "/var/log/warpflow", file = "application.json" }
```

2) 错误日志分离（按过滤器）
```toml
version = "2.0"
[sink_group]
name = "/sink/error_logs"
oml  = ["application_logs"]

[[sink_group.sinks]]
name = "all"
connect = "file_json_sink"
params = { file = "all.json" }

[[sink_group.sinks]]
name = "err"
connect = "file_json_sink"
filter = "./error_filter.wpl"
params = { file = "err.json" }
```

3) 并行 + 自动分片
```toml
version = "2.0"
[sink_group]
name = "/sink/high_volume"
parallel = 4

[[sink_group.sinks]]
name = "out"
connect = "file_json_sink"
params = { base = "/data/out", file = "batch.json", replica_shard = true }
# 结果：/data/out/batch_0.json, batch_1.json, ...
```

4) 并行 + 自定义模板
```toml
version = "2.0"
[sink_group]
name = "/sink/custom_names"
parallel = 3

[[sink_group.sinks]]
name = "out"
connect = "file_json_sink"
params = { base = "/data/out", file = "x.json", file_template = "x-{replica1}.json" }
# 结果：x-1.json/x-2.json/x-3.json
```

过滤器示例（WPL）
```wpl
# error_filter.wpl
level == "ERROR" || level == "FATAL"
```

## 性能与行为
- 内置缓冲写（约 100 KiB）并每 100 条自动 flush；程序结束时会显式 flush。
- 推荐通过组并行与磁盘/CPU 协同优化吞吐；不再提供 fast_file/缓冲环境变量。
- 不内置轮转；如需按大小/时间轮转，请配合系统工具（logrotate 等）。

## 常见问题
- 权限不足：确认输出目录可写，必要时 `sudo chown -R <user> <dir>`。
- 磁盘空间不足：`df -h` 检查并清理历史文件。
- 路径不存在：文件 Sink 会自动创建父目录；若失败，请检查上级目录权限。
- 分片未生效：需同时满足 `parallel>1` 且设置 `replica_shard=true`（或提供 `file_template`）。

## 相关文档
- [Sink 配置基础](./01-sinks_basics.md)
- [Sink 路由](./routing.md)
- [Sink 并行与分片](./parallel_and_sharding.md)
- [连接器管理](./connectors.md)
- [故障排除指南](../09-FQA/troubleshooting.md)
