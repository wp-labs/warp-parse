# Source 性能优化

本文档介绍 warp-flow 系统中数据源的性能优化策略和最佳实践。

## 性能指标

### 关键性能指标 (KPIs)

#### 1. 吞吐量指标
- **输入速率**: 每秒处理的数据包数量
- **数据量速率**: 每秒处理的数据字节数
- **峰值吞吐量**: 系统能处理的最大吞吐量

#### 2. 延迟指标
- **端到端延迟**: 数据从接收到输出的总时间
- **处理延迟**: 数据在源处理阶段的时间
- **网络延迟**: 网络传输造成的延迟

#### 3. 资源使用指标
- **CPU 使用率**: 处理数据的 CPU 消耗
- **内存使用**: 缓冲区和数据结构的内存占用
- **网络带宽**: 网络接口的使用情况
- **文件 I/O**: 磁盘读写操作的性能

#### 4. 可靠性指标
- **错误率**: 处理失败的数据比例
- **重试率**: 需要重试的操作比例
- **可用性**: 系统正常运行时间比例

## 不同源类型的性能优化

### 1. File 源优化

#### 文件组织优化
```toml
# ✅ 使用较小的文件 (< 1GB)
[[sources]]
key = "rotated_logs"
connect = "file_src"
[sources.params_override]
base = "/var/log/nginx"
file = "access.log.2024-01-01"

# ❌ 避免超大文件
# file = "access.log.all_time"
```

#### 缓冲区配置
```toml
# 高性能文件连接器
[[connectors]]
id = "file_high_perf"
type = "file"
allow_override = ["base", "file", "encode", "buffer_size"]
[connectors.params]
buffer_size = 81920    # 80KB 缓冲区
read_ahead = true      # 启用预读
```

#### 编码选择
```toml
# 性能编码选择
[sources.params_override]
encode = "text"         # 最快：无需解码
# encode = "base64"     # 中等：需要解码
# encode = "hex"        # 最慢：需要解码
```

#### 文件系统优化
```bash
# 使用高性能文件系统
# ext4/xfs: 一般性能
# tmpfs: 内存文件系统，最快
# NFS: 网络文件系统，最慢
```

### 2. Kafka 源优化

#### 批量消费配置
```toml
# 高吞吐量配置
[sources.params_override.config]
max_poll_records = "1000"         # 增加批处理大小
fetch_min_bytes = "102400"        # 100KB 最小抓取
fetch_max_wait_ms = "100"         # 100ms 最大等待
max_poll_interval_ms = "300000"   # 5分钟最大轮询间隔
```

#### 连接池优化
```toml
# 连接优化配置
[sources.params_override.config]
session_timeout_ms = "60000"      # 1分钟会话超时
heartbeat_interval_ms = "5000"    # 5秒心跳间隔
connections_max_idle_ms = "540000" # 9分钟连接空闲超时
```

#### 内存管理
```toml
# 内存使用优化
[sources.params_override.config]
queued_min_messages = "100000"    # 增加队列大小
queued_max_messages_kbytes = "1048576"  # 1GB 队列限制
```

#### 分配策略
```toml
# 优化的消费者组配置
[[sources]]
key = "kafka_optimized"
connect = "kafka_main"
[sources.params_override]
group_id = "optimized_group"
[sources.params_override.config]
partition_assignment_strategy = "range"  # 或 "roundrobin"
enable_auto_commit = "false"             # 手动提交偏移量
```

### 3. Syslog 源优化

#### 协议选择
```toml
# UDP: 高吞吐量，可能丢包
[[sources]]
key = "syslog_high_throughput"
connect = "syslog_udp"
[sources.params_override]
protocol = "udp"

# TCP: 可靠传输，保证顺序
[[sources]]
key = "syslog_reliable"
connect = "syslog_tcp"
[sources.params_override]
protocol = "tcp"
```

#### 缓冲区配置
```toml
# 高性能 TCP 缓冲区
[sources.params_override]
tcp_recv_bytes = 104857600    # 100MB 接收缓冲区

# 低内存配置
[sources.params_override]
tcp_recv_bytes = 1048576      # 1MB 接收缓冲区
```

#### 网络优化
```toml
# 网络层优化
[sources.params_override.config]
tcp_nodelay = true           # 禁用 Nagle 算法
tcp_keepalive = true         # 启用 TCP Keepalive
so_rcvbuf = 1048576         # 设置接收缓冲区
```

## 系统级优化

### 1. 操作系统配置

#### 文件描述符限制
```bash
# 增加文件描述符限制
echo "* soft nofile 65536" >> /etc/security/limits.conf
echo "* hard nofile 65536" >> /etc/security/limits.conf
```

#### 网络配置
```bash
# 网络缓冲区优化
echo 'net.core.rmem_max = 134217728' >> /etc/sysctl.conf
echo 'net.core.wmem_max = 134217728' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_rmem = 4096 65536 134217728' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_wmem = 4096 65536 134217728' >> /etc/sysctl.conf
sysctl -p
```

#### I/O 调度器
```bash
# 对于高 I/O 负载
echo noop > /sys/block/sda/queue/scheduler  # SSD
echo deadline > /sys/block/sda/queue/scheduler  # 传统硬盘
```

### 2. JVM 优化 (如果适用)

#### 堆内存配置
```bash
# 生产环境 JVM 参数
JAVA_OPTS="-Xms4g -Xmx8g -XX:+UseG1GC -XX:MaxGCPauseMillis=200"
```

#### GC 调优
```bash
# G1GC 配置
JAVA_OPTS="-XX:+UseG1GC -XX:MaxGCPauseMillis=200 -XX:G1HeapRegionSize=16m"
```

## 监控和调试

### 1. 性能监控配置
```toml
# 启用性能监控
[[sources]]
key = "monitored_source"
connect = "kafka_main"
tags = [
    "monitor:performance",
    "metrics:prometheus",
    "alert_threshold:1000",  # 每秒1000条消息阈值
    "alert_latency:100"      # 100ms延迟阈值
]
```

### 2. 指标收集
- **吞吐量指标**: `source_throughput_total`, `source_throughput_rate`
- **延迟指标**: `source_processing_latency`, `source_end_to_end_latency`
- **错误指标**: `source_errors_total`, `source_retries_total`
- **资源指标**: `source_cpu_usage`, `source_memory_usage`

### 3. 性能分析工具
```bash
# 系统性能分析
top -p $(pgrep wpgen)        # CPU 使用情况
iostat -x 1                 # I/O 性能
netstat -i                  # 网络接口统计
sar -n DEV 1                # 网络设备统计

# 应用级分析
perf top -p $(pgrep wpgen)  # 性能热点分析
strace -p $(pgrep wpgen)    # 系统调用跟踪
```

## 性能测试

### 1. 基准测试
```bash
# 文件源基准测试
dd if=/dev/zero of=test_data.log bs=1024 count=1000000
wpgen source benchmark --file test_data.log

# Kafka 源基准测试
wpgen source benchmark --kafka --topic benchmark --messages 1000000
```

### 2. 负载测试
```bash
# 并发负载测试
wpgen source load_test --concurrent 10 --duration 300s

# 峰值负载测试
wpgen source stress_test --rate 10000 --duration 60s
```

### 3. 容量规划
```bash
# 容量评估工具
wpgen source capacity_plan --growth_rate 0.2 --period 365d
```

## 故障排除

### 性能问题诊断

#### 1. 高 CPU 使用率
```bash
# 诊断步骤
1. top -p $(pgrep wpgen)           # 查看CPU使用
2. perf top -p $(pgrep wpgen)      # 找出CPU热点
3. 检查配置是否有死循环或低效操作
```

#### 2. 高内存使用
```bash
# 诊断步骤
1. ps aux | grep wpgen            # 查看内存使用
2. pmap $(pgrep wpgen)            # 查看内存映射
3. 检查是否有内存泄漏
```

#### 3. 网络瓶颈
```bash
# 诊断步骤
1. netstat -i                     # 查看网络统计
2. iftop -i eth0                  # 查看网络流量
3. tcpdump -i eth0 port 9092      # 抓包分析
```

#### 4. 磁盘 I/O 瓶颈
```bash
# 诊断步骤
1. iostat -x 1                   # 查看I/O统计
2. iotop                         # 查看进程I/O
3. 检查磁盘健康状态
```

## 最佳实践总结

### 1. 配置优化
- 选择合适的连接器类型
- 优化缓冲区大小
- 配置适当的批量大小
- 启用性能监控

### 2. 架构设计
- 使用水平扩展
- 实现负载均衡
- 设计故障恢复机制
- 规划容量扩展

### 3. 运维管理
- 建立监控告警
- 定期性能测试
- 维护性能基线
- 持续优化调整

### 4. 安全考虑
- 在性能和安全之间找到平衡
- 监控异常性能波动
- 设置合理的资源限制
- 定期安全审计

## 相关文档

- [性能优化指南](../05-performance/README.md)
- [源配置基础](./01-sources_basics.md)
- [故障排除指南](../09-FQA/troubleshooting.md)
- [监控和指标](../../80-reference/README.md)
