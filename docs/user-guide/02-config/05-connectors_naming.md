# 连接器命名规范

目标：统一 `connectors/{source.d,sink.d}` 下文件与 `id` 的命名，便于团队协作、脚本约定与可读性。

## 总则
- 字符集：小写英文字母、数字、下划线（`[a-z0-9_]`），不使用空格/大写/汉字。
- 语义顺序：`<kind>_<variant>[_src|_sink]`（示例：`syslog_udp_src`、`kafka_main_sink`）。
- Suffix：
  - Sources 强制使用 `_src` 后缀（示例：`syslog_udp_src`）。
  - Sinks 推荐使用 `_sink` 后缀；历史文件类保留简写（例如 `file_json_sink`），新建非文件类统一加 `_sink`。
- Kind 取值（常用）：`file`、`syslog`、`kafka`、`mysql`、`clickhouse`、`elasticsearch`、`prometheus`。
- Variant 取值：体现细化维度，如 `udp|tcp`、`json|kv|raw|proto_text`、`dev|prod|main`、`clusterA` 等。

## 文件命名
- 模式：`NN-<kind>-<variant>.toml`
  - `NN` 两位序号用于分组排序与阅读；建议：
    - 00–09：文件类（file）
    - 10–19：网络类（syslog）
    - 30–49：流式/服务类（kafka/prometheus/...）
    - 90–99：示例/测试（example/test/local）
- 示例：
  - `source.d/00-file-default.toml`（id=`file_src`）
  - `source.d/10-syslog-udp.toml`（id=`syslog_udp_src`）
  - `sink.d/01-file-prototext.toml`（id=`file_proto_sink`）
  - `sink.d/02-file-json.toml`（id=`file_json_sink`）
  - `sink.d/10-syslog-udp.toml`（id=`syslog_udp_sink`）
  - `sink.d/30-kafka.toml`（id=`kafka_main_sink`）

## ID 命名（connectors[].id）
- Sources（严格）：`<kind>_<variant>_src`（必须以 `_src` 结尾）
  - `file_src`（默认文件读取）
  - `file_gen`（本地生成器读回）
  - `syslog_udp_src`、`syslog_tcp_src`
- Sinks（严格）：
  - 非 file 类：`<kind>_<variant>_sink`（必须以 `_sink` 结尾）
  - file 类：建议 `<kind>_<variant>`（以 `file_` 开头；历史命名 `file_json_sink/file_kv_sink/...` 保留，不强制 `_sink`）

## 字段命名对齐
- 统一使用 `type` 指定连接器种类（内部与 `kind` 对应）。
- `allow_override` 仅列出允许在路由/实例层覆盖的字段，避免过度开放。
- 常见字段建议：
  - file：优先使用 `base` + `file`（或单一 `path`），`fmt`（sinks）/`encode`（sources）
  - syslog：`protocol`、`addr`、`port`、`header_mode`、`prefer_newline`、`tcp_recv_bytes`（sources）
  - kafka：`brokers`、`topic`（string/array 皆可）

### 可选：conn_url 辅助写法（需启用适配器）
- 当应用注册了连接器适配器（wp_connector_api::config::adapter），可在生成流程或工具侧通过 `conn_url` 提供简写，由适配器解析为 `params`：
  - MySQL（sink）：`mysql://user:pass@host:port/dbname` → `{ endpoint, username, password, database, table?, batch? }`
  - Kafka（sink）：`kafka://broker1,broker2?topic=xxx&num_partitions=3&replication=1` → `{ brokers, topic, ... }`
- 注意：当前 route 层不直接解析 `params.conn_url`；建议在生成/CLI 侧使用（本仓库的 `wparse/wproj/wprescue` 已默认注册开发期适配器）。

## 模板与工具
- 初始化标准目录与模板：`wproj conf init`
  - sources：会创建 `connectors/source.d/00-file-default.toml` 等默认模板（文件源）
  - sinks：可用 `wproj sinks init` 生成 `models/sinks/{business.d,infra.d,defaults.toml}` 与默认示例
  - 机制：模板由内置资源生成（include_str!），已存在文件不覆盖
- 查看引用：
  - `wproj sources list -w <WR>` / `wproj sources route -w <WR>`
  - `wproj sinks list -w <WR>` / `wproj sinks route -w <WR>`


## 附：示例片段
```toml
# connectors/source.d/00-file-default.toml（file_src, base+file+encode）
[[connectors]]
id = "file_src"
type = "file"
allow_override = ["base", "file", "encode"]
[connectors.params]
base = "data/in_dat"
file = "gen.dat"
encode = "text"

# connectors/sink.d/02-file-json.toml
[[connectors]]
id = "file_json_sink"
type = "file"
allow_override = ["base", "file"]
[connectors.params]
fmt  = "json"
base = "./data/out_dat"
file = "default.json"
```

### 兼容：文件名提示 vs 类型
- 对于 `test_rescue`（文件族）类型，允许文件名提示为 `file`，不会报不一致的 WARN。
