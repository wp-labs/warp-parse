# Sinks 并行与分片
<!-- 角色：使用配置者 | 最近验证：2025-12-11 -->

目标
- 通过业务组的组级并行与文件命名模板，实现多文件输出与更高吞吐。

核心概念
- 业务组并行：设置 `[sink_group].parallel = N`，会并行运行该组的 N 个协程（每个副本一个协程）。
- 文件分片（仅业务组）：当使用 `file` 且为 `base+file` 写法时，结合 `replica_shard=true` 可自动产出 `file_0.ext`、`file_1.ext` …；或使用 `params.file_template = "name-{replica1}.ext"`（或 `{replica}`）。
- 基础组（infra.d）不支持 `parallel`，也不支持 `replica_shard/file_template`；如需提升吞吐与分片，请在 business.d 使用并行。

操作步骤
1）连接器（文件）
```
[[connectors]]
id = "file_main"
type = "file"
allow_override = ["base","file","path","fmt","file_template","replica_shard"]
[connectors.params]
fmt  = "json"
base = "./data/out_dat"
file = "out.json"
```
2）业务组（business.d）并行
```
version = "2.0"
[sink_group]
name = "/sink/demo"
parallel = 4

[[sink_group.sinks]]
name = "out"
connect = "file_main"
# 自动分片（base+file）：out_0.json/out_1.json/…
params = { replica_shard = true }
# 或者使用自定义模板
# params = { file_template = "out-{replica1}.json" }
```
3）基础组（infra.d）不支持并行
```
version = "2.0"
[sink_group]
name = "default"

[[sink_group.sinks]]
name = "default_sink"
connect = "file_main"
```

注意
- 仅业务组支持 `parallel` 与分片模板；基础组不支持。
- 仅 `base+file` 写法支持自动分片；纯 `path` 不改名，除非提供 `file_template`（同样仅限业务组）。
- 文件 sink 采用内部缓冲（100 KiB 级别）并在批量写时周期性 flush；如需极端性能，请结合业务组并行与磁盘/CPU 侧优化。

相关
- 设计与配置总览：`../02-config/04-sinks_config.md`
- 架构与模型：`../../30-decision/README.md`
- CLI 校验/展示：`./validate_route.md`
