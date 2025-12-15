# TCP Sink 配置

TCP sink 用于将数据输出到任意 TCP 服务端，支持按行或长度前缀（RFC6587 风格）分帧。

## 连接器定义
```toml
[[connectors]]
id = "tcp_sink"
type = "tcp"
allow_override = ["addr", "port", "framing"]

[connectors.params]
addr = "127.0.0.1"
port = 19000
framing = "line"   # line|len
```

## 使用示例（wpgen 输出到 TCP）
```toml
# conf/wpgen.toml
[generator]
mode = "sample"
count = 10000

[output]
connect = "tcp_sink"

[output.params]
addr = "127.0.0.1"
port = 19000
framing = "line"
```

## 分帧说明
- `line`：追加 `\n` 作为消息结束符
- `len`：发送 `<len><space><payload>`（不追加 `\n`）

## 调试建议
- 使用 `nc -lk <port>` 简易验证接收端
- 与内置 `tcp_src` 联调：将源端口与 sink 端口一致即可构造回环链路

