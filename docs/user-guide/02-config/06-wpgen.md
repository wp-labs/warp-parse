# wpgen（生成器配置）

概述
- 目标：让数据生成器 wpgen 的输出与 当前 Sink 路由保持一致，复用 `connectors/sink.d` 的连接器定义，避免在 `wpgen.toml` 重复维护连接参数。
- 兼容性：保留已发布的 `wpgen.toml`（`[output].type/file/kafka/syslog`）；当存在 `output.connect` 时优先走 connectors 写法，其它 output 字段将被忽略。

配置写法（新）
- conf/wpgen.toml（关键字段）
```toml
version = "1.0"

[generator]
mode = "sample"
count = 1000
speed = 1000
parallel = 1

[output]
# 引用 connectors/sink.d 中的连接器 id；当该字段存在时，type/format 将被忽略
connect = "file_kv_sink"
# 可选：为运行期展示命名
name = "gen_out"
# 仅能覆写 allow_override 白名单内的键（新版使用 params；误写 params_override 将报错）
params = { base = "./src_dat", file = "gen.dat" }

[logging]
level = "warn"
output = "file"
file_path = "./data/logs/"

```

- connectors/sink.d/00-wpgen.toml（示例连接器）
```toml
[[connectors]]
id = "file_kv_sink"
type = "file"
allow_override = ["base", "file"]
[connectors.params]
fmt  = "kv"
base = "./data/out_dat"
file = "out.dat"

[[connectors]]
id = "kafka_main"
type = "kafka"
allow_override = ["brokers", "topic", "num_partitions", "replication"]
[connectors.params]
brokers = "localhost:9092"
topic = "wparse-test"

[[connectors]]
id = "syslog_udp"
type = "syslog"
allow_override = ["addr", "port", "protocol"]
[connectors.params]
addr = "127.0.0.1"
port = 514
protocol = "udp"
```

运行规则
- `wpgen` 会在加载 `conf/wpgen.toml` 时，若检测到 `[output].connect`：
 - 从 `ENGINE_CONF.sink_root` 向上查找最近的 `connectors/sink.d/` 目录。
  - 读取目标连接器并与 `params` 合并（仅允许 `allow_override` 中的键）。
  - 文件类连接器：输出格式 `fmt` 由连接器决定（支持 `raw/json/kv/proto-text`）。
  - 非文件类：输出格式固定为 JSON（由后端决定/接收）。
  - 当 `[generator].speed = 0`（无限速）且连接器为 `tcp_sink` 时：若未显式设置 `params.max_backoff`，将默认开启“发送队列感知 backoff”；有限速则关闭。

一致性说明
- 语义同 docs/10-user/02-config/04-sinks_config.md：
  - 连接器存在即启用；是否使用取决于是否被 `connect` 引用。
  - 覆写键必须在 `allow_override` 白名单中；误写 `params`/`params_override` 嵌套将报错提示。
  - 非文件类 route 中的 `fmt` 配置将被忽略（统一由后端决定）。

生成控制（与运行期一致）
- count：总产出条数。启动时按 parallel 精确分配到各个 worker；余数前置分配，总量严格等于 count。
- speed：全局速率；`speed=0` 表示无限制；否则每 worker= floor(speed/parallel)，单位批量自适应。
- parallel：生成 worker 并行数；对 `blackhole_sink`，消费端也会随之并行；其它 sink 默认单消费者。

验证
- 检查配置：`wpgen conf check`
- 生成数据（示例）：
  - 文件：`wpgen sample -n 10000`（写入 `params.base + params.file`）
  - Kafka：将 `connect = "kafka_main"`，并 `params = { topic = "demo" }`
