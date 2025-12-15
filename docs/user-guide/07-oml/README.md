# OML 对象模型语言

OML（Object Modeling Language）用于在 Warp Flow 中对解析后的记录进行组装与聚合，提供 read/take 取值、对象与数组聚合（object/collect）、条件匹配（match）、字符串格式化（fmt）、管道转换（pipe）与 SQL 查询拼装等能力。

注意：从当前版本起，引擎默认不启用“隐私/脱敏”运行期处理；本章中涉及“隐私段”的语法仅作为 DSL 能力说明，若需要脱敏，请在业务侧或自定义插件/管道中实现。

## 内容概览

- [OML 语言基础](./01-oml_basics.md)
- [OML 使用示例](./02-oml_examples.md)
- [OML 语法（EBNF）](./03-oml_grammar_ebnf.md)
- [OML DSL 改进建议（草案）](./04-oml_dsl_changes_proposal.md)

## 特性概览

- 取值与缺省：`read(...)`（非破坏）/`take(...)`（破坏）+ 默认体 `{ _ : <值/函数> }`
- 对象/数组聚合：`object { ... }`、`collect read(keys:[...])`
- 条件匹配：`match read(x) { ... }` 与二元匹配 `match (read(a), read(b)) { ... }`
- 管道与格式化：`read(x) | to_json | base64_en`，`fmt("{}-{}", @a, read(b))`
- SQL：`select <cols from table> where <cond>;`（主体白名单校验，严格模式可通过 `OML_SQL_STRICT=0` 关闭）
- 批量目标：目标名含 `*` 时按批量模式求值（仅支持 take/read）
- 隐私段：末尾通过第二个 `---` 声明字段隐私处理器映射

## 快速示例

```oml
name : example
---
user_id        = read(user_id) ;
occur_time:time= Time::now() ;
values : obj   = object {
  cpu_free, memory_free : digit = take() ;
};
ports : array  = collect read(keys:[sport,dport]) ;
ports_json     = read(ports) | to_json ;
full           = fmt("{}-{}", @user, read(city)) ;
name,pinying   = select name,pinying from example where pinying = read(py) ;
---
src_ip : privacy_ip
pos_sn : privacy_keymsg
```

## 相关文档

- [WPL 规则语言](../06-wpl/README.md)
- [配置指南概述](../02-config/README.md)
- [Schema 参考文档](../../80-reference/schemas/README.md)

提示：read/take 的差异见《OML 语言基础》；完整语法见《OML 语法（EBNF）》；端到端示例见《OML 使用示例》。
