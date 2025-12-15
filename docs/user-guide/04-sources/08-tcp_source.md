# TCP 源配置（tcp）
<!-- 角色：使用配置者 | 最近验证：2025-12-11 -->

本章介绍通用 TCP 源（kind=`tcp`）的使用方式、分帧模式与与 TCP Sink 的联动示例。

## 功能概览
- 支持三种分帧模式：
  - `line`：按换行符分帧；行末的 CR/空格/Tab 会被去除
  - `len`：长度前缀（RFC 6587 octet-counting）：`<len><SP><payload>`
  - `auto`（默认）：自动选择；默认优先 `len`，当 `prefer_newline=true` 时优先按行
- 连接生命周期：服务端监听、并发连接处理、外部停止信号优雅关闭
- 事件标签：自动注入 `access_ip`；可叠加用户自定义标签
 - 运行模式：在 batch（批处理）模式下会被忽略，不会启动监听；请在 daemon 模式使用 TCP 源。

## 连接器定义（source.d）
```toml
[[connectors]]
id = "tcp_src"
type = "tcp"
allow_override = ["addr", "port", "framing", "tcp_recv_bytes", "prefer_newline", "instances"]

[connectors.params]
addr = "0.0.0.0"
port = 19000
framing = "auto"          # auto|line|len
prefer_newline = false     # auto 时是否优先按行
tcp_recv_bytes = 10485760  # 10 MiB
# instances = 1             # 可选：多实例并行，默认 1，最大 16
```

## 源配置（models/sources/wpsrc.toml）
```toml
[[sources]]
key = "tcp_in"
connect = "tcp_src"
enable = true
tags = ["source:tcp", "type:raw"]
params_override = {
  port = 19000,
  framing = "auto",
  prefer_newline = true,
  instances = 2
}
```

> `instances` 为可选项；当设置为大于 1 时，单监听端口会以 round-robin 方式为多个 Source 实例分配连接，以提升 Parser 并行能力。

## 分帧模式详解
### line（换行）
- 适用：文本日志、人工/脚本推送、简单工具（nc/tail）链路
- 行尾会去掉 `\r`/空格/Tab；建议发送端每条以 `\n` 结尾

### len（长度前缀）
- 形如：`5 hello` → 表示下一条 payload 长度为 5 字节（不包含前缀中的空格）
- 适用：payload 可能包含换行/二进制的场景（例如多行日志、堆栈、压缩片段）
- 接收端约束：长度最大 10MB、前缀最多 10 位十进制，异常时丢弃当前尝试，避免内存膨胀

### auto（自动）
- 默认优先尝试 `len`；若 `prefer_newline=true` 则优先按行
- 若已检测到“长度前缀进行中”（读到 `<digits><SP>` 但 payload 未到齐），会继续等待，而不会回退按行，避免误切分

## 与 TCP Sink 联动（回环链路）
为了便于端到端联调，本项目提供了通用 TCP Sink（kind=`tcp`）：
- sink connectors：`connectors/sink.d/12-tcp.toml`
- sink 参数：`addr`/`port`/`framing(line|len)`
- 示例：`wpgen` 输出到 `tcp_sink`，`wparse` 以 `tcp_src` 监听同端口，实现本机回环

示例 wpgen（conf/wpgen.toml）
```toml
[generator]
mode = "sample"
count = 10000

[output]
connect = "tcp_sink"

[output.params]
addr = "127.0.0.1"
port = 19000
framing = "line"  # 或 "len"
```

## 完整用例（usecase/core/tcp_roundtrip）
目录：`usecase/core/tcp_roundtrip`
- 启动：`./case_verify.sh`
- 步骤：启动 wparse（tcp 源）→ wpgen 推送（tcp sink）→ 校验文件输出

## 常见问题（FAQ）
- 问：文本以“数字+空格”开头会不会被误判为长度前缀？
  - 答：在 `auto` 模式下，确有可能；可通过 `framing="line"` 或 `auto+prefer_newline=true` 避免
- 问：为什么推荐生产中用 `len`？
  - 答：边界明确、对二进制/多行更稳健；很多 syslog/TCP 等生产链路推荐/默认使用 octet‑counting

## 最佳实践
- 仅文本：`framing="line"`
- 多行/二进制：`framing="len"`（或 `auto` 默认）
- 快速联调：`auto + prefer_newline=true`，配合 `nc -lk <port>`
