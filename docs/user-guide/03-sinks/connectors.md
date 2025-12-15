# Sinks 连接器
<!-- 角色：使用配置者 | 最近验证：2025-12-11 -->

目标
- 用连接器集中声明输出目标与默认参数；在路由中按 `id` 引用，通过白名单安全覆写。

核心概念（权威见 `../02-config/04-sinks_config.md` 与 `../02-config/06-wpgen.md`）
- 连接器文件：`connectors/sink.d/*.toml`，可在一个文件中定义多个 `[[connectors]]` 记录。
- 字段：`id`、`type`（file/kafka/syslog/prometheus/...）、`allow_override`、`params`（默认参数）。
- 覆写：在路由里用 `params` 扁平覆写，键必须属于 `allow_override`。

示例（文件类）
```toml
[[connectors]]
id = "file_main"
type = "file"
allow_override = ["base","file","path","fmt","file_template","replica_shard"]
[connectors.params]
fmt  = "json"
base = "./data/out_dat"
file = "out.json"
```

在路由中引用
```toml
[[sink_group.sinks]]
name = "file_out"
connect = "file_main"
params = { file = "demo.json" }
```

命名模板与分片（结合组并行）
- `file_template = "name-{replica}.json"`（0 基）或 `"name-{replica1}.json"`（1 基）。
- 当组并行 `parallel > 1` 时：
  - 若设置 `replica_shard = true`，且使用 `base+file` 写法，则自动生成 `name_0.ext/name_1.ext...`；
  - 若提供 `file_template`，按模板生成文件名；
  - 仅使用 `path`（完整路径）默认不改名，除非提供 `file_template`。

校验与排错
- `wproj sinks list -w <WR>` 查看连接器与被引用关系。
- `override 'xxx' not allowed`：键不在 `allow_override` 白名单；改用白名单内键或在连接器端增加白名单。
- 嵌套表报错：请扁平书写（`params = { base="...", file="..." }`）。

相关
- 查找规则与合并逻辑：`../02-config/04-sinks_config.md`
- 生成器输出对齐：`../02-config/06-wpgen.md`
