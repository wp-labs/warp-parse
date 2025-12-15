# Syslog Sink 配置

本文档与代码实现对齐，描述 Syslog Sink 的实际可用参数与示例。

## 概述

Syslog Sink 将数据以 RFC3164 文本格式发送到 Syslog 服务器，支持 UDP 与 TCP。消息格式固定：`<PRI>TIMESTAMP HOSTNAME TAG: MESSAGE`，其中 `TAG` 固定为 `wpgen`。

## 连接器定义

使用仓库内置模板（`connectors/sink.d/10-syslog-udp.toml`、`11-syslog-tcp.toml`）：

```toml
[[connectors]]
id = "syslog_udp_sink"
type = "syslog"
allow_override = ["addr", "port", "protocol"]
[connectors.params]
addr = "127.0.0.1"
port = 1514
protocol = "udp"

[[connectors]]
id = "syslog_tcp_sink"
type = "syslog"
allow_override = ["addr", "port", "protocol"]
[connectors.params]
addr = "127.0.0.1"
port = 1514
protocol = "tcp"
```

说明：代码侧仅使用 `addr/port/protocol` 三个键；文档中未出现的参数（facility/severity/hostname/...）在当前实现中不生效。

## 可用参数（路由 `params`）

- `addr`：Syslog 服务器地址（IP 或主机名）。
- `port`：端口（1–65535）。
- `protocol`：`udp` 或 `tcp`（大小写不敏感）。

## 配置示例

1) 基础 UDP 输出
```toml
version = "2.0"
[sink_group]
name = "/sink/syslog_basic"
oml  = ["application_logs"]

[[sink_group.sinks]]
name = "syslog_output"
connect = "syslog_udp_sink"
params = { addr = "syslog.example.com", port = 1514, protocol = "udp" }
```

2) 高可用 TCP（主备）
```toml
version = "2.0"
[sink_group]
name = "/sink/syslog_ha"
parallel = 2

[[sink_group.sinks]]
name = "primary"
connect = "syslog_tcp_sink"
params = { addr = "syslog-primary.example.com", port = 514, protocol = "tcp" }

[[sink_group.sinks]]
name = "backup"
connect = "syslog_tcp_sink"
params = { addr = "syslog-backup.example.com", port = 514, protocol = "tcp" }
```

3) 按条件分流
```toml
version = "2.0"
[sink_group]
name = "/sink/syslog_by_cond"

[[sink_group.sinks]]
name = "error"
connect = "syslog_tcp_sink"
filter = "./error_filter.wpl"
params = { addr = "syslog-errors.example.com", port = 514, protocol = "tcp" }

[[sink_group.sinks]]
name = "info"
connect = "syslog_udp_sink"
filter = "./info_filter.wpl"
params = { addr = "syslog-info.example.com", port = 1514, protocol = "udp" }
```

## 故障排除与调试
- 连接被拒绝：`nc -vz <host> <port>` 检查连通性；确认防火墙放行端口。
- 丢包（UDP）：可切换 `protocol = "tcp"` 以换取可靠性。
- 验证消息：`sudo tcpdump -i any -n port 514 -A` 抓包，或用 `logger -p local0.info -t warpflow "test"` 验证服务器侧。
