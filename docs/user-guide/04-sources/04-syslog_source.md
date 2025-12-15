# Syslog 源配置

本文档详细介绍如何配置和使用 warp-flow 系统的 Syslog 数据源。

## 概述

Syslog 源用于接收和解析标准的 Syslog 协议消息，支持 UDP 和 TCP 两种传输协议，以及多种 Syslog 格式。

## 连接器定义

### UDP Syslog 连接器

```toml
# connectors/source.d/syslog_udp.toml
[[connectors]]
id = "syslog_udp_src"
type = "syslog"
allow_override = ["addr", "port", "protocol", "header_mode", "prefer_newline"]

[connectors.params]
addr = "0.0.0.0"
port = 1514
protocol = "udp"
header_mode = "parse"
prefer_newline = false
tcp_recv_bytes = 10485760
```

### TCP Syslog 连接器

```toml
# connectors/source.d/syslog_tcp.toml
[[connectors]]
id = "syslog_tcp_src"
type = "syslog"
allow_override = ["addr", "port", "protocol", "header_mode", "prefer_newline"]

[connectors.params]
addr = "127.0.0.1"
port = 1514
protocol = "tcp"
header_mode = "parse"
prefer_newline = false
tcp_recv_bytes = 10485760
```

### 高性能 Syslog 连接器

```toml
# connectors/source.d/syslog_high_perf.toml
[[connectors]]
id = "syslog_high_perf"
type = "syslog"
allow_override = ["addr", "port", "protocol", "header_mode", "prefer_newline", "tcp_recv_bytes"]

[connectors.params]
addr = "0.0.0.0"
port = 1514
protocol = "tcp"
header_mode = "parse"
prefer_newline = true
tcp_recv_bytes = 104857600  # 100MB 缓冲区
```

## 支持的参数

### 基础网络参数

#### addr (必需)
监听地址

```toml
[sources.params_override]
addr = "0.0.0.0"    # 监听所有接口
addr = "127.0.0.1"   # 仅本地接口
addr = "10.0.0.100"  # 特定接口
```

#### port (必需)
监听端口

```toml
[sources.params_override]
port = 1514          # 自定义端口
port = 514           # 标准 syslog 端口 (需要 root 权限)
```

#### protocol (必需)
传输协议

```toml
[sources.params_override]
protocol = "udp"     # UDP 协议 (低延迟)
protocol = "tcp"     # TCP 协议 (可靠传输)
```

### 消息处理参数

#### header_mode
头部处理模式

```toml
[sources.params_override]
header_mode = "parse"   # 解析+注入标签+剥离头部
header_mode = "strip"   # 仅剥离头部，不注入标签
header_mode = "keep"    # 保留头部，原样透传
```

#### prefer_newline
优先按换行进行分帧（对纯换行流量更高效）。混合场景下保持正确性（遇到合法长度前缀但数据未齐时等待）。

```toml
[sources.params_override]
prefer_newline = true   # 优先换行分帧
prefer_newline = false  # 默认：先尝试长度前缀，再回退换行
```

### TCP 专用参数

#### tcp_recv_bytes
TCP 接收缓冲区大小

```toml
[sources.params_override]
tcp_recv_bytes = 10485760     # 10MB (默认)
tcp_recv_bytes = 104857600    # 100MB (高性能)
tcp_recv_bytes = 1048576      # 1MB (低内存)
```

## 配置示例

### 基础 UDP 配置
```toml
# wpsrc.toml
[[sources]]
enable = true
key = "syslog_udp_1"
connect = "syslog_udp_src"
tags = ["protocol:udp", "env:production"]

[sources.params_override]
addr = "0.0.0.0"
port = 1514
protocol = "udp"
```

### 基础 TCP 配置
```toml
# wpsrc.toml
[[sources]]
enable = true
key = "syslog_tcp_1"
connect = "syslog_tcp_src"
tags = ["protocol:tcp", "env:production"]

[sources.params_override]
addr = "127.0.0.1"
port = 1514
protocol = "tcp"
process_header = true
prefer_newline = true
```

### 双协议配置
```toml
# wpsrc.toml
[[sources]]
enable = true
key = "syslog_udp_collector"
connect = "syslog_udp_src"
tags = ["protocol:udp", "type:collector", "tier:edge"]

[sources.params_override]
addr = "0.0.0.0"
port = 1514
protocol = "udp"
process_header = true
prefer_newline = false

[[sources]]
enable = true
key = "syslog_tcp_aggregator"
connect = "syslog_tcp_src"
tags = ["protocol:tcp", "type:aggregator", "tier:core"]

[sources.params_override]
addr = "127.0.0.1"
port = 1515
protocol = "tcp"
process_header = true
prefer_newline = true
tcp_recv_bytes = 104857600
```

### 开发环境配置
```toml
# wpsrc.toml
[[sources]]
enable = true
key = "dev_syslog"
connect = "syslog_udp_src"
tags = ["env:development", "team:backend"]

[sources.params_override]
addr = "127.0.0.1"
port = 1514
protocol = "udp"
process_header = false
prefer_newline = false
```

## 数据处理特性

### 1. Syslog 格式支持

#### RFC3164 格式 (传统 BSD Syslog)
```
<34>Oct 11 22:14:15 mymachine su: 'su root' failed for lonvick on /dev/pts/8
```

#### RFC5424 格式 (现代 Syslog)
```
<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"] BOMAn application event log entry
```

### 2. 解析字段

当 `header_mode = "parse"` 时，系统会解析并添加以下标签：

```json
{
  "data": "原始消息内容",
  "tags": {
    "source_type": "syslog",
    "syslog_priority": 34,        // 数值优先级
    "syslog_facility": 4,         // 设施代码
    "syslog_severity": 2,         // 严重性级别
    "syslog_hostname": "mymachine",
    "syslog_app_name": "su",
    "syslog_proc_id": "1234",     // 进程ID (RFC5424)
    "syslog_msg_id": "ID47",      // 消息ID (RFC5424)
    "syslog_timestamp": "Oct 11 22:14:15"
  }
}
```

### 3. 设施和严重性映射

#### 设施 (Facility) 映射
| 代码 | 设施 | 描述 |
|------|------|------|
| 0    | kern | 内核消息 |
| 1    | user | 用户级消息 |
| 2    | mail | 邮件系统 |
| 3    | daemon | 系统守护进程 |
| 4    | auth | 安全/认证消息 |
| ...  | ...  | ... |

#### 严重性 (Severity) 映射
| 代码 | 级别 | 描述 |
|------|------|------|
| 0    | emerg | 紧急 - 系统不可用 |
| 1    | alert | 警报 - 必须立即采取行动 |
| 2    | crit | 严重 - 严重情况 |
| 3    | err | 错误 - 错误情况 |
| 4    | warning | 警告 - 警告情况 |
| 5    | notice | 通知 - 正常但重要的情况 |
| 6    | info | 信息 - 信息性消息 |
| 7    | debug | 调试 - 调试级消息 |

## 性能优化

### 1. 协议选择
```toml
# UDP: 低延迟，可能丢包
protocol = "udp"

# TCP: 可靠传输，保证顺序
protocol = "tcp"
```

### 2. 缓冲区优化
```toml
# 高流量环境
tcp_recv_bytes = 104857600  # 100MB

# 低内存环境
tcp_recv_bytes = 1048576    # 1MB
```

### 3. 分帧/头部处理优化
```toml
# 高性能场景：
prefer_newline = true         # 纯换行流量常见，降低固定开销
header_mode = "strip"         # 仅去头，减少解析与标签注入

# 分析场景：
header_mode = "parse"         # 解析并注入协议相关元信息
prefer_newline = false        # 混合/长度前缀较多时保持默认顺序
```

升级说明（Breaking）
- 移除：`strip_header`、`attach_meta_tags`、`process_header`；改用 `header_mode`（`keep|strip|parse`）。

## 集成配置

### 与 Rsyslog 集成
```bash
# /etc/rsyslog.d/99-warpflow.conf
*.* @@127.0.0.1:1514;RSYSLOG_TraditionalFileFormat
```

### 与 Logstash 集成
```ruby
# logstash.conf
output {
  syslog {
    host => "127.0.0.1"
    port => 1514
    protocol => "tcp"
    facility => "local0"
    severity => "info"
  }
}
```

### 与 Docker 集成
```bash
# Docker daemon 配置
{
  "log-driver": "syslog",
  "log-opts": {
    "syslog-address": "udp://localhost:1514",
    "syslog-facility": "local0",
    "tag": "docker/{{.Name}}"
  }
}
```

## 监控和指标

### 1. 内置指标
- 消息接收速率 (messages/second)
- 错误率 (invalid messages/second)
- 网络延迟
- 缓冲区使用率

### 2. 健康检查
```toml
tags = [
    "monitor:syslog_source",
    "health_check:enabled",
    "alert_on_error:true"
]
```

## 故障排除

### 常见问题

#### 1. 端口占用
```
Error: Address already in use
```
**解决方案**:
- 检查端口是否被其他进程占用
- 使用不同的端口
- 停止冲突的服务

#### 2. 权限不足
```
Error: Permission denied
```
**解决方案**:
- 使用 > 1024 的端口 (不需要 root 权限)
- 以适当权限运行进程
- 检查防火墙设置

#### 3. 消息格式错误
```
Error: Invalid syslog format
```
**解决方案**:
- 检查发送端配置
- 验证 Syslog 格式规范
- 调整解析设置

#### 4. TCP 连接问题
```
Error: Connection refused
```
**解决方案**:
- 确认端口监听状态
- 检查网络连接
- 验证防火墙规则

### 调试技巧

#### 1. 测试 UDP 发送
```bash
# 发送测试 UDP 消息
echo "<34>test message" | nc -u localhost 1514
```

#### 2. 测试 TCP 发送
```bash
# 发送测试 TCP 消息
echo "<34>test message" | nc localhost 1514
```

#### 3. 启用详细日志
```bash
RUST_LOG=debug wpgen source start wpsrc.toml
```

#### 4. 网络抓包
```bash
# 抓取 syslog 流量
sudo tcpdump -i any -n port 1514 -A
```

## 最佳实践

### 1. 端口规划
```toml
# 不同环境使用不同端口
# 开发环境
port = 1514

# 测试环境
port = 1515

# 生产环境
port = 514  # 标准 syslog 端口
```

### 2. 协议选择
- **UDP**: 适用于高频、可容忍丢包的场景
- **TCP**: 适用于可靠性要求高的场景

### 3. 标签规范
```toml
tags = [
    "protocol:tcp",
    "env:production",
    "datacenter:dc1",
    "rack:rack01",
    "service_type:syslog_collector"
]
```

### 4. 安全配置
```toml
# 限制监听接口
addr = "127.0.0.1"  # 仅本地访问

# 使用防火墙限制访问
# iptables -A INPUT -p tcp --dport 1514 -s 10.0.0.0/8 -j ACCEPT
```

## 相关文档

- [源配置基础](./01-sources_basics.md)
- [文件源配置](./02-file_source.md)
- [Kafka 源配置](./03-kafka_source.md)
- [连接器管理](./05-connectors.md)
