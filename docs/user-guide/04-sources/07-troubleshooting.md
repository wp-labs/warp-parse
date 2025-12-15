# Source 故障排除

本文档提供 warp-flow 系统中数据源常见问题的诊断和解决方案。

## 通用故障排除流程

### 1. 基础诊断检查清单
```bash
# 1. 检查配置文件语法
wpgen source validate wpsrc.toml

# 2. 检查连接器配置
wpgen connectors validate

# 3. 查看系统日志
journalctl -u wpgen -f

# 4. 检查进程状态
ps aux | grep wpgen

# 5. 检查端口占用
netstat -tlnp | grep :1514
```

### 2. 调试模式启动
```bash
# 启用详细日志
RUST_LOG=debug wpgen source start wpsrc.toml

# 启用跟踪日志
RUST_LOG=trace wpgen source start wpsrc.toml

# 输出到控制台便于调试
wpgen source start wpsrc.toml --no-daemon
```

### 3. 配置验证工具
```bash
# 完整配置验证
wpgen source validate wpsrc.toml --verbose

# 测试特定源
wpgen source test --source-id kafka_main

# 连接器连通性测试
wpgen connectors test --id file_src
```

## File 源故障排除

### 常见错误和解决方案

#### 1. 文件不存在错误
```
Error: No such file or directory (os error 2)
```

**症状**: 系统无法找到指定的文件

**诊断步骤**:
```bash
# 检查文件是否存在
ls -la /path/to/file.log

# 检查文件路径配置
cat wpsrc.toml | grep -A 10 "file_1"

# 检查相对路径
pwd
find . -name "file.log"
```

**解决方案**:
- 确认文件路径正确
- 使用绝对路径避免路径混淆
- 检查文件权限

#### 2. 权限不足错误
```
Error: Permission denied (os error 13)
```

**症状**: 系统没有读取文件的权限

**诊断步骤**:
```bash
# 检查文件权限
ls -la /path/to/file.log

# 检查运行用户
ps aux | grep wpgen

# 测试文件访问
sudo -u wpgen_user cat /path/to/file.log
```

**解决方案**:
```bash
# 修改文件权限
chmod 644 /path/to/file.log

# 修改文件所有者
chown wpgen_user:wpgen_group /path/to/file.log

# 将用户添加到相应组
usermod -a -G log_group wpgen_user
```

#### 3. 编码错误
```
Error: Invalid base64 encoding
Error: Invalid hex encoding
```

**症状**: 文件内容编码与配置不匹配

**诊断步骤**:
```bash
# 检查文件内容
file /path/to/file.log
head -1 /path/to/file.log | hexdump -C

# 测试编码
echo "test" | base64
echo "test" | hexdump -C
```

**解决方案**:
```toml
# 修正编码配置
[sources.params_override]
encode = "text"      # 对于文本文件
# encode = "base64"  # 对于 base64 编码文件
# encode = "hex"     # 对于十六进制文件
```

#### 4. 文件被占用
```
Error: Resource busy (os error 16)
```

**症状**: 文件被其他进程占用

**诊断步骤**:
```bash
# 查看文件占用
lsof /path/to/file.log
fuser /path/to/file.log
```

**解决方案**:
```bash
# 停止占用进程
kill -9 <PID>

# 使用文件复制
cp /path/to/file.log /path/to/file_copy.log
# 配置指向复制文件
```

### 性能问题

#### 1. 读取缓慢
**症状**: 文件读取速度明显低于预期

**诊断步骤**:
```bash
# 检查磁盘I/O
iostat -x 1
iotop

# 检查文件系统
df -h
mount | grep /path/to
```

**解决方案**:
- 使用 SSD 存储
- 优化文件系统
- 增加缓冲区大小

#### 2. 内存占用过高
**症状**: 处理大文件时内存使用过高

**诊断步骤**:
```bash
# 检查内存使用
ps aux | grep wpgen
pmap $(pgrep wpgen)
```

**解决方案**:
- 使用流式处理
- 减少批处理大小
- 增加系统内存

## Kafka 源故障排除

### 常见错误和解决方案

#### 1. 连接失败
```
Error: Failed to connect to Kafka cluster
Error: Connection refused
Error: Timeout while connecting to Kafka
```

**症状**: 无法连接到 Kafka 集群

**诊断步骤**:
```bash
# 检查网络连通性
telnet kafka1 9092
nc -zv kafka1 9092

# 检查 Kafka 集群状态
kafka-broker-api-versions.sh --bootstrap-server kafka1:9092

# 检查 DNS 解析
nslookup kafka1
dig kafka1
```

**解决方案**:
```toml
# 修正 broker 地址
[sources.params_override]
brokers = ["kafka1:9092", "kafka2:9092"]
```

#### 2. 认证失败
```
Error: SASL authentication failed
Error: SSL handshake failed
```

**症状**: 认证配置不正确

**诊断步骤**:
```bash
# 测试 Kafka 连接
kafka-console-consumer.sh \
  --bootstrap-server kafka1:9092 \
  --topic test_topic \
  --consumer.config /path/to/consumer.properties
```

**解决方案**:
```toml
# 检查认证配置
[sources.params_override.config]
security_protocol = "SASL_SSL"
sasl_mechanisms = "PLAIN"
sasl_username = "correct_username"
sasl_password = "correct_password"
```

#### 3. 主题不存在
```
Error: Topic not found
Error: Unknown topic or partition
```

**症状**: 指定的主题不存在

**诊断步骤**:
```bash
# 列出所有主题
kafka-topics.sh --bootstrap-server kafka1:9092 --list

# 查看主题详情
kafka-topics.sh --bootstrap-server kafka1:9092 --describe --topic your_topic
```

**解决方案**:
```bash
# 创建主题
kafka-topics.sh --bootstrap-server kafka1:9092 \
  --create --topic your_topic --partitions 3 --replication-factor 2
```

#### 4. 消费者组问题
```
Error: Consumer group rebalance failed
Error: Max poll interval exceeded
```

**症状**: 消费者组协调问题

**诊断步骤**:
```bash
# 查看消费者组状态
kafka-consumer-groups.sh --bootstrap-server kafka1:9092 \
  --describe --group your_group_id
```

**解决方案**:
```toml
# 调整消费者组配置
[sources.params_override.config]
session_timeout_ms = "30000"
heartbeat_interval_ms = "3000"
max_poll_interval_ms = "300000"
```

### 性能问题

#### 1. 消费延迟高
**症状**: 消费者滞后于生产者

**诊断步骤**:
```bash
# 检查消费者延迟
kafka-consumer-groups.sh --bootstrap-server kafka1:9092 \
  --describe --group your_group_id
```

**解决方案**:
```toml
# 增加批处理大小
[sources.params_override.config]
max_poll_records = "1000"
fetch_min_bytes = "102400"
```

#### 2. 频繁重平衡
**症状**: 消费者组频繁进行重平衡

**诊断步骤**:
```bash
# 查看重平衡日志
grep "Rebalance" /var/log/wpgen.log
```

**解决方案**:
```toml
# 调整超时配置
[sources.params_override.config]
session_timeout_ms = "60000"
max_poll_interval_ms = "300000"
```

## Syslog 源故障排除

### 常见错误和解决方案

#### 1. 端口占用
```
Error: Address already in use
Error: Permission denied
```

**症状**: 端口被其他进程占用或权限不足

**诊断步骤**:
```bash
# 检查端口占用
netstat -tlnp | grep :1514
lsof -i :1514

# 检查端口权限
# 端口 < 1024 需要 root 权限
```

**解决方案**:
```toml
# 使用非特权端口
[sources.params_override]
port = 1514  # 而不是 514
```

#### 2. 网络连接问题
```
Error: Connection refused
Error: Network unreachable
```

**症状**: 客户端无法连接到 Syslog 服务

**诊断步骤**:
```bash
# 测试网络连接
telnet localhost 1514
nc -v localhost 1514

# 检查防火墙
iptables -L -n
ufw status
```

**解决方案**:
```bash
# 配置防火墙
iptables -A INPUT -p udp --dport 1514 -j ACCEPT
iptables -A INPUT -p tcp --dport 1514 -j ACCEPT
```

#### 3. 消息格式错误
```
Error: Invalid syslog format
Error: Parse error in syslog message
```

**症状**: 接收到的消息不符合 Syslog 格式

**诊断步骤**:
```bash
# 监听原始消息
tcpdump -i any -n port 1514 -A

# 测试消息格式
echo "<34>test message" | nc -u localhost 1514
```

**解决方案**:
```toml
# 调整解析配置
[sources.params_override]
header_mode = "keep"
prefer_newline = true
```

### 性能问题

#### 1. 消息丢失
**症状**: UDP 模式下消息丢失

**诊断步骤**:
```bash
# 检查网络统计
netstat -su | grep "packet receive errors"
```

**解决方案**:
```toml
# 使用 TCP 模式
[sources.params_override]
protocol = "tcp"
```

#### 2. 缓冲区溢出
**症状**: 高流量时消息丢失

**诊断步骤**:
```bash
# 检查系统缓冲区
netstat -s | grep "buffer"
```

**解决方案**:
```toml
# 增加缓冲区大小
[sources.params_override]
tcp_recv_bytes = 104857600  # 100MB
```

## 配置问题

### 1. TOML 语法错误
```
Error: Failed to parse TOML configuration
```

**诊断步骤**:
```bash
# 验证 TOML 语法
python3 -c "import toml; toml.load(open('wpsrc.toml'))"

# 使用在线验证器
# https://www.toml-lint.com/
```

**常见语法错误**:
```toml
# ❌ 错误：缺少引号
key = unquoted_value

# ❌ 错误：多余的逗号
list = ["item1", "item2", ]

# ✅ 正确
key = "quoted_value"
list = ["item1", "item2"]
```

### 2. 连接器引用错误
```
Error: Connector 'unknown_connector' not found
```

**诊断步骤**:
```bash
# 列出可用连接器
wpgen connectors list

# 检查连接器文件
ls -la connectors/source.d/
```

**解决方案**:
```toml
# 使用正确的连接器ID
[sources]
connect = "file_src"  # 确保此连接器存在
```

### 3. 参数覆盖错误
```
Error: Parameter 'invalid_param' not in allow_override list
```

**诊断步骤**:
```bash
# 查看连接器配置
cat connectors/source.d/your_connector.toml
```

**解决方案**:
```toml
# 在连接器中添加参数到 allow_override
[[connectors]]
allow_override = ["valid_param1", "valid_param2"]
```

## 监控和日志

### 1. 启用详细日志
```bash
# 设置日志级别
export RUST_LOG=debug
export RUST_LOG=wpgen=trace

### 输出格式与位置
- 日志格式：`{时间} [LEVEL] [target] {message}`（不包含 `<module:line>`）
- 默认位置：`./logs/<binary>.log`，超过 10MB 按 10 份滚动（gz）
wpgen source start wpsrc.toml 2>&1 | tee wpgen.log
```

### 2. 监控关键指标
```bash
# 实时监控日志
tail -f /var/log/wpgen/wpgen.log

# 监控系统资源
htop
iotop
nethogs
```

### 3. 性能分析
```bash
# CPU 性能分析
perf top -p $(pgrep wpgen)

# 内存使用分析
valgrind --tool=massif wpgen source start wpsrc.toml
```

## 预防措施

### 1. 配置管理
- 使用版本控制管理配置文件
- 建立配置变更审批流程
- 定期备份配置文件

### 2. 监控告警
```toml
# 配置监控标签
tags = [
    "monitor:enabled",
    "alert_on_error:true",
    "alert_threshold:1000"
]
```

### 3. 测试验证
- 部署前充分测试
- 建立预发布环境
- 定期进行故障演练

### 4. 文档维护
- 记录常见问题和解决方案
- 维护故障排除手册
- 定期更新文档

## 相关文档

- [源配置基础](./01-sources_basics.md)
- [性能优化指南](./06-performance.md)
- [连接器管理](./05-connectors.md)
- [故障排除指南](../09-FQA/troubleshooting.md)
