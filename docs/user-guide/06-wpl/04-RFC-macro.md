
#宏与模块化复用

本文档描述 WPL 在“复用与模块化”方向的设计提案，包含：宏（macro）机制与包含（include）。目标是降低复制粘贴、提升大规则可维护性，同时保证语法简单、行为直观。

## 设计目标
- 复用：将通用字段组合/片段抽象为可参数化模板，在多条 rule 中重复使用；
- 模块化：允许跨文件组织宏与规则，通过 include 聚合；
- 渐进兼容：默认不影响现有语法；宏为可选能力，采用显式调用；
- 易于实现与诊断：展开在装配期完成，错误信息能定位到“调用点+宏定义”双重上下文。

## 宏（Macro）

### 语法草案

1) 宏定义
```
macro [kind] name(params) { body }

// kind（可选）：限定展开形态，便于在相应位置做语义校验
//   - group     -> body 应为一个 group 片段： ( ... )[N][SEP]
//   - fields    -> body 应为字段列表片段：    f1, f2, ...
//   - subfields -> body 应为子字段列表片段：  s1, s2, ...
//   - express   -> body 应为完整表达式：      [preproc] group (, group)*

// 示例：
macro group http_pair(prefix) {
  (ip@sip: ${prefix}_sip, ip@dip: ${prefix}_dip, http/request", http/agent")
}
```

2) 宏调用
```
// 推荐使用 `name!(args)`，避免与已有函数/类型关键字冲突
name!(args)

// 示例：
rule sample {
  |decode/base64|unquote/unescape|
  http_pair!("client"),
  http_pair!("server")
}
```

3) 参数与占位符
- 形参形式：`name(param1, param2=default, ...)`，默认值可选；
- 实参形式：允许 `quoted_string` 或 `raw_string(r#"..."#)`；
- 占位符替换：在宏体内使用 `$param` 或 `${param}` 进行字面替换；
  - 占位符替换后，宏体必须形成合法的 WPL 片段（由 `kind` 限定的片段类型）；
  - 若无 `kind`，默认按“token 片段”展开，再由后续解析器完整解析（易用但弱约束）。

4) 作用域与命名空间
- 宏定义位于包内（package）或文件顶层；
- 调用解析顺序：文件内宏 > 包内宏；
- 跨文件引用使用命名空间：`pkg::name!(...)`；
- 不允许递归宏；最大展开深度建议 32（防御失控展开）。

5) 卫生（Hygiene）
- 首版不做自动改名（无 gensym）；展开片段中的字段命名冲突由规则作者自行避免；
- 后续可考虑提供内置函数 `gensym(prefix)` 生成唯一后缀，用于字段名去冲突：`${prefix}_${gensym("id")}`。

### EBNF 草案（节选）
```
macro_def   = "macro" [ ws kind ] ws name ws "(" [ params ] ")" ws "{" ws macro_body ws "}" ;
kind        = "group" | "fields" | "subfields" | "express" ;
params      = param { ws "," ws param } ;
param       = name [ ws "=" ws default_val ] ;
default_val = quoted_string | raw_string | key ;

macro_call  = name "!" ws "(" [ args ] ")" ;
args        = arg { ws "," ws arg } ;
arg         = quoted_string | raw_string | key ;

// 展开阶段：
// - 在构建 AST 之前，先收集 macro_def 并从源码流中剔除其定义；
// - 遇到 macro_call 时进行替换，得到新的源码流；
// - 替换后的源码进入既有解析流程（wpl_express / group / fields / subfields）。
```

### 错误与诊断
- 未定义宏/重名：在调用处报错；
- 实参与形参个数不符：在调用处报错并显示定义位置；
- 展开后语法错误：同时展示调用点与宏定义片段，便于定位；
- 超出展开深度或递归：阻断并报错（含调用链）。

### 示例

1) fields 级宏
```
macro fields base_fields(app) {
  digit:id, time_3339:recv, chars:app_${app}
}

rule x {
  ( base_fields!("waf"), http/request", http/agent" )
}
```

2) subfields 级宏
```
macro subfields ids() { digit@uid, digit@pid }

rule y {
  ( kv( ids!() ), json(chars@name) )
}
```

3) express 级宏
```
macro express waf_line() {
  |decode/base64|unquote/unescape|
  (ip@sip, ip@dip, http/request", http/agent")
}

rule waf_case { waf_line!() }
```

## include（包含）

```
include "common_macros.wpl"
include "../shared/http_macros.wpl"
```

- 语义：将包含文件中的宏/规则合并到当前包命名空间；
- 冲突：同名宏或 rule 冲突时报错；或提供 `include as` 起别名（按需）。

## 兼容性与渐进落地
- 新增关键字：`macro`、`include`、`name!` 调用形式；不会影响现有规则；
- 首批落地可仅支持 `fields`/`group` 两种 `kind`，逐步扩展到 `subfields`/`express`；
- 工具链支持：
  - 语法高亮与格式化：将 `macro_def/macro_call/include` 纳入词法，保持美观打印；
  - 诊断：宏展开时保留“源位置信息”，在错误报告中显示“调用点/定义点”。

---

如需调整设计或落地优先级，请在变更提案 PR 中补充你的场景与示例片段。
