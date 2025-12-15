# WPL 规则语言

WPL (Warp Processing Language) 是 Warp Flow 解析子系统（warp-parse）使用的规则语言，主要用于描述字段抽取、协议解析与简单判定逻辑。该目录文档与 `crates/wp-lang` 的解析实现保持一致。

## 内容概览

- [WPL 语言基础](./01-wpl_basics.md)
- [WPL 语法（EBNF）](./02-wpl_grammar.md)
- [变更记录](./03-CHANGELOG.md)

## 快速示例

```wpl
package demo {
  rule http_access {
    |decode/base64|unquote/unescape|
    (
      digit:status,
      time_3339:recv_time,
      ip:client_ip,
      http/request<[,]>,
      http/agent"
    )
  }
}
```

## 相关文档

- 语法实现参考：`crates/wp-lang/src/parser/`
- 数据类型定义：`crates/wp-data-model/src/model/types/meta.rs`
