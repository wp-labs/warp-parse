# Prometheus Sink 配置

本文档与代码实现对齐。当前 Prometheus Sink 为“自暴露”型 Exporter，会在本地启动一个 HTTP 服务（默认仅支持 Counter 类计数），对外暴露 `/metrics`。

## 连接器定义

使用仓库模板（`connectors/sink.d/40-prometheus.toml`）：

```toml
[[connectors]]
id = "prometheus_sink"
type = "prometheus"
allow_override = ["endpoint", "source_key_format", "sink_key_format"]
[connectors.params]
endpoint = "127.0.0.1:35666"   # 监听地址（对外暴露 /metrics）
source_key_format = "(?P<source_type>.)_(?P<access_source>.)"
sink_key_format   = "(?P<rule>.)_(?P<sink_type>.)_sink"
```

说明：不支持 Pushgateway/自定义 `metric_name`/`metric_type`/`labels`。内置指标名固定：
- `wparse_receive_data`（从数据源接收条数，带源标签）
- `wparse_parse_success`、`wparse_parse_all`（解析成功/总量）
- `wparse_send_to_sink`（发送到 sink 的条数，带 sink 标签）

## 可用参数（路由 `params`）

- `endpoint`：Exporter 监听的 `host:port`（如 `127.0.0.1:35666`）。
- `source_key_format`：可选，用于从 key 中提取 `source_type/access_source` 的正则（具名分组）。
- `sink_key_format`：可选，用于从 key 中提取 `rule/sink_type` 的正则（具名分组）。

## 配置示例

启动 Exporter 并暴露指标：
```toml
version = "2.0"
[sink_group]
name = "/sink/prom_exporter"
oml  = ["metrics"]

[[sink_group.sinks]]
name = "prom"
connect = "prometheus_sink"
params = { endpoint = "0.0.0.0:35666" }
```

验证：`curl http://127.0.0.1:35666/metrics`
