# DataRecord 机制数据字段

## 概述

Warp Flow 系统在解析数据时，会自动向 DataRecord 追加一些机制数据字段，用于追踪数据的来源和处理路径。这些机制数据字段以 `wp_` 前缀标识，为系统提供了数据溯源和调试能力。

## 机制数据字段列表

### 1. wp_event_id

- **字段类型**: 字符串 (String)
- **描述**: 事件的唯一标识符
- **来源**: 从 SourceEvent.event_id 获取
-**用途**: 追踪单个事件在系统中的完整处理流程

### 2. wp_src_key

- **字段类型**: 字符串 (String)
- **描述**: 数据源的标识符
- **来源**: 从 SourceEvent.src_key 获取
- **用途**: 标识数据来源于哪个数据源（如 "syslog_1", "file_reader" 等）

### 3. wp_src_ip

- **字段类型**: IP 地址 (IP)
- **描述**: 数据源的客户端 IP 地址
- **来源**: 从 SourceEvent.ups_ip 获取
- **用途**: 记录发送数据的客户端 IP 地址，用于审计和定位

## 字段追加机制

### 触发条件

机制数据的追加由 `ParseOption` 中的 `gen_msg_id` 参数控制：

```rust
// src/core/parser/setting.rs
#[derive(Default, Getters)]
pub struct ParseOption {
    gen_msg_id: bool,  // 控制是否生成机制数据字段
    stat_req: Vec<StatReq>,
}
```

当 `gen_msg_id = true` 时，系统会在解析成功时自动追加机制数据字段。

### 实现位置

机制数据的追加在 `src/core/parser/workshop/packet_parser.rs` 中实现：

```rust
// 第36-42行
if *setting.gen_msg_id() {
    tdo_crate.set_id(event.event_id);              // 设置 wp_event_id
    tdo_crate.append(Field::from_chars("wp_src_key", event.src_key.as_str()));
    if let Some(ups_ip) = event.ups_ip {
        tdo_crate.append(Field::from_ip("wp_src_ip", ups_ip));
    }
}
```

## 配置方法

### 1. 创建 ParseOption 时启用

```rust
let parse_option = ParseOption::new(
    true,  // 启用 gen_msg_id
    stat_req_vec
);
```

### 2. 在 WPL 规则中使用

这些机制数据字段可以在 WPL 规则中直接使用：

```wpl
rule example {
    # 使用 wp_src_key 进行条件判断
    wp_src_key == "syslog_1" => {
        # 处理来自 syslog_1 的数据
    }

    # 使用 wp_src_ip 进行过滤
    wp_src_ip == 192.168.1.100 => {
        # 处理来自特定 IP 的数据
    }

    # 输出到 sink 时包含这些字段
    output => {
        # wp_event_id, wp_src_key, wp_src_ip 会自动包含在输出中
    }
}
```

## 使用场景

### 1. 数据溯源

通过 `wp_event_id` 可以追踪数据在系统中的完整处理路径：
- 数据采集
- 解析处理
- 规则匹配
- 输出到 sink

### 2. 问题诊断

当出现数据问题时，可以通过这些字段快速定位：
- `wp_src_key`: 确定数据来源
- `wp_src_ip`: 确定发送方
- `wp_event_id`: 关联日志中的处理记录

### 3. 数据分析

这些字段可以用于数据分析：
- 按数据源统计 (`wp_src_key`)
- 按 IP 统计 (`wp_src_ip`)
- 事件流分析 (`wp_event_id`)

## 注意事项

1. **性能影响**: 启用机制数据会轻微增加内存使用和处理时间
2. **存储空间**: 这些字段会占用额外的存储空间
3. **隐私考虑**: `wp_src_ip` 可能涉及隐私信息，使用时需要遵守相关法规
4. **字段名冲突**: 避免在 WPL 规则中定义与机制数据同名的字段

## 示例输出

```json
{
  "wp_event_id": "evt_20241201_001234",
  "wp_src_key": "syslog_prod",
  "wp_src_ip": "10.0.1.100",
  "message": "<34>Oct 11 22:14:15 mymachine su: 'su root' failed for lonvick on /dev/pts/8",
  "timestamp": "2024-10-11T22:14:15Z",
  "severity": "error",
  "facility": "auth"
}
```

## 相关文档

- [数据源配置](./01-sources_basics.md)
- [Syslog 数据源](./04-syslog_source.md)
- [WPL 规则语法](../05-wpl/README.md)