# 文件源配置

本文档详细介绍如何配置和使用 warp-flow 系统的文件数据源。

## 概述

文件源用于从本地文件系统读取数据，支持多种编码格式和灵活的路径配置。

## 连接器定义

### 基础文件连接器

```toml
# connectors/source.d/00-file-default.toml
[[connectors]]
id = "file_src"
type = "file"
allow_override = ["base", "file", "encode"]

[connectors.params]
base = "./data/in_dat"
file = "gen.dat"
encode = "text"
```

## 支持的参数

### 路径配置

#### 方式一：直接路径 (path)
```toml
[[sources]]
key = "file_direct"
connect = "file_src"
[sources.params_override]
path = "/var/log/access.log"
```

#### 方式二：基础目录 + 文件名 (base + file)
```toml
[[sources]]
key = "file_composed"
connect = "file_src"
[sources.params_override]
base = "/var/log"
file = "access.log"
```

### 编码格式

#### Text 编码 (默认)
```toml
[sources.params_override]
encode = "text"
```

#### Base64 编码
```toml
[sources.params_override]
encode = "base64"
```

#### Hex 编码
```toml
[sources.params_override]
encode = "hex"
```

## 配置示例

### 基础文件读取
```toml
# wpsrc.toml
[[sources]]
enable = true
key = "access_log"
connect = "file_src"
tags = ["env:production", "type:access_log"]

[sources.params_override]
base = "/var/log/nginx"
file = "access.log"
encode = "text"
```

### 多文件源配置
```toml
# wpsrc.toml
[[sources]]
enable = true
key = "nginx_access"
connect = "file_src"
tags = ["service:nginx", "type:access"]
[sources.params_override]
base = "/var/log/nginx"
file = "access.log"
encode = "text"

[[sources]]
enable = true
key = "nginx_error"
connect = "file_src"
tags = ["service:nginx", "type:error"]
[sources.params_override]
base = "/var/log/nginx"
file = "error.log"
encode = "text"

[[sources]]
enable = true
key = "app_binary"
connect = "file_src"
tags = ["service:app", "type:binary"]
[sources.params_override]
path = "/var/log/app/app.log"
encode = "base64"
```

### 开发环境配置
```toml
# wpsrc.toml
[[sources]]
enable = true
key = "dev_file_1"
connect = "file_src"
tags = ["env:dev", "src_ip:10.0.0.1"]
[sources.params_override]
base = "./test_data"
file = "sample.log"
encode = "text"
```

## 数据处理特性

### 1. 逐行读取
文件源采用逐行读取模式，每行作为独立的数据包处理。

### 2. 自动标签添加
系统会自动为每个数据包添加访问路径相关的标签：
- `source_path`: 文件路径
- `source_name`: 文件名
- `source_type`: 源类型 (file)

### 3. 编码处理
- **text**: 直接读取文本内容
- **base64**: 读取后进行 Base64 解码
- **hex**: 读取后进行十六进制解码

## 性能考虑

### 1. 文件大小
- 建议单个日志文件不超过 1GB
- 使用日志轮转机制管理大文件

### 2. 读取性能
- 文件源采用流式读取，内存占用较低
- 对于高频写入的文件，考虑使用缓冲机制

### 3. 网络文件系统
- 支持 NFS 等网络文件系统
- 注意网络延迟对读取性能的影响

## 故障排除

### 常见问题

#### 1. 文件不存在
```
Error: No such file or directory (os error 2)
```
**解决方案**: 检查文件路径是否正确，确保文件存在且可读

#### 2. 权限不足
```
Error: Permission denied (os error 13)
```
**解决方案**: 检查文件权限，确保运行用户有读取权限

#### 3. 编码错误
```
Error: Invalid encoding
```
**解决方案**: 检查 `encode` 参数设置，确保与文件实际编码匹配

### 调试技巧

#### 1. 验证配置
```bash
wpgen source validate wpsrc.toml
```

#### 2. 测试文件读取
```bash
# 使用小文件测试配置
echo "test log line" > test.log
# 在配置中指向测试文件
```

#### 3. 查看详细日志
```bash
# 启用调试日志
RUST_LOG=debug wpgen source start wpsrc.toml
```

## 最佳实践

### 1. 路径管理
- 使用相对路径便于环境迁移
- 为不同环境配置不同的基础路径
- 避免使用硬编码的绝对路径

### 2. 标签规范
```toml
tags = [
    "env:production",           # 环境
    "service:nginx",           # 服务名称
    "type:access_log",         # 日志类型
    "datacenter:dc1"           # 数据中心
]
```

### 3. 连接器复用
- 创建通用的文件连接器模板
- 通过参数覆盖适应不同场景
- 减少重复配置

### 4. 监控配置
```toml
tags = [
    "monitor:file_source",
    "alert_on_error:true"
]
```

## 相关文档

- [源配置基础](./01-sources_basics.md)
- [Kafka 源配置](./03-kafka_source.md)
- [Syslog 源配置](./04-syslog_source.md)
- [连接器管理](./05-connectors.md)
