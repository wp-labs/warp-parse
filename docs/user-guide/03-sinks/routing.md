# Sinks 路由
<!-- 角色：使用配置者 | 最近验证：2025-12-11 -->

目标
- 基于目录式 routes（business.d/infra.d）配置路由分发；理解业务组与基础组的差异及命名规则。

核心概念（单点定义见 `../02-config/04-sinks_config.md`）
- 业务组 business.d：面向场景输出，可设置 `[sink_group].parallel`；与文件类 sink 结合可实现多文件分片（见下）。
- 基础组 infra.d：系统级输出（default/miss/residue/error/monitor）。基础组不支持 `parallel`，也不支持文件分片；如需吞吐/分片，请在业务组配置。
- `full_name`：运行期统一标识 `{group}/{sink}`，CLI 展示与日志使用该格式。

目录与命名
- 业务组：`$SINK_ROOT/business.d/**/*.toml`，支持子目录递归；每个路由文件一个组。
- 基础组：`$SINK_ROOT/infra.d/**/*.toml`，支持子目录递归；固定组名（default/miss/...）。
- 连接器：从 `$WORK_ROOT/models/sinks` 起向上查找最近的 `connectors/sink.d/*.toml`。

最小示例（业务组）
```toml
version = "2.0"
[sink_group]
name = "/sink/demo"
oml  = ["/oml/example/*"]
parallel = 1

[[sink_group.sinks]]
name = "file_out"
connect = "file_json_sink"   # 连接器 id
params = { file = "demo.json" }
```

并行与分片（多文件输出）
- 设置 `parallel = N`，组被复制 N 份；文件类（`file`）在 `replica_shard=true` 或提供 `file_template` 时按副本命名：`name_0.ext`、`name_1.ext`...
- 自定义命名：`params.file_template = "name-{replica1}.ext"` 或 `"name-{replica}.ext"`。

基础组示例（default）
```toml
version = "2.0"
[sink_group]
name = "default"

[[sink_group.sinks]]
name = "default_sink"
connect = "file_json_sink"
```

校验与展示（CLI）
- `wproj sinks validate|list|route -w <WR>`
- `wproj stat sink-file -w <WR>`

相关
- 权威：`../02-config/04-sinks_config.md`
- 分片：`./parallel_and_sharding.md`
- 连接器：`./connectors.md`
