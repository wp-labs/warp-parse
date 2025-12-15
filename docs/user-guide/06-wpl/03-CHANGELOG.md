# WPL 语法与实现变更记录

本文档记录 WPL（warp-parse 规则语言）的语法、解析与文档层面的显著变更，便于规则作者与集成方迁移与审阅。

## 2025-10-23

变更主题：@plugin 语法统一为代码块内联形式

- 变更内容
  - 仅支持代码块内联形式：`@plugin(id: <id>) { <Express> }`。
  - 移除旧的字符串内联形式：不再支持 `@plugin(id: <id>, rule: "<Express>")`。
  - 打印/格式化输出同步：展示为
    ```wpl
    @plugin(id:<id>) {
      (<express ...>)
    }
    ```
- 受影响范围
  - 解析器：`crates/wp-lang/src/parser/wpl_rule.rs` 中 `plugin` 解析改为读取 `{ ... }` 并复用 `wpl_express`。
  - AST 展示：`crates/wp-lang/src/ast/plugin.rs` 的 DebugFormat/Display 输出同步更新。
  - 测试与基准：仓库内使用字符串形式的 `@plugin` 已更新为代码块写法（测试、基准已修）。
  - 文档：
  - 规范（EBNF）：`docs/10-user/06-wpl/02-wpl_grammar.md` 已体现 `Statement = PluginBlock | Express`；新增 `PluginBlock` 产生式。
  - 门面说明：`docs/10-user/06-wpl/02-wpl_grammar.md`、`docs/10-user/06-wpl/01-wpl_basics.md` 更新为仅代码块形式。
- 迁移指引
  - 示例迁移
    - 旧：`@plugin(id: demo, rule: "(json(_@_origin,_@payload/packet_data))")`
    - 新：`@plugin(id: demo) { (json(_@_origin,_@payload/packet_data)) }`
  - 快速排查项目中的旧写法
    - `rg -n "@plugin\(.*rule\s*:"`（查找包含 `rule:` 的 @plugin 即为旧写法）
  - 兼容性
    - 本次为不兼容变更（breaking change），旧写法会解析失败；请尽快统一迁移。

变更主题：EBNF 与文档对齐

- 变更内容
  - 收敛 EBNF 到单一 Markdown 文档：`docs/10-user/06-wpl/02-wpl_grammar.md`；
  - 在基础文档 `01-wpl_basics.md` 补充/修订：
    - 明确分组与字段、管道、子字段等写法示例；
    - 给出 `@plugin` 代码块内联示例；
    - 强调分隔符写法（逐字符反斜杠转义，如 `\\,`、`\\!\\|`）。

- 重要澄清（非新语法，但强约束）
  - 计数字段格式 `^N` 仅适用于 `chars` 与 `_` 类型；用于其他类型会被拒绝。
  - 组后缀 `[N]` 的长度会应用到组内所有字段；
  - 子字段键默认是 `*`，使用 `@ref/path` 可显式指定键。

变更主题：分组关键字统一（移除 one_of，使用 alt）

- 变更内容
  - 语义上 `one_of` 与 `alt` 等价；为避免歧义，仅保留 `alt`。
  - 解析实现与文档移除 `one_of`：
    - EBNF：`group_meta = "alt" | "opt" | "some_of" | "seq"`；
    - 解析器：不再识别 `one_of`；
    - 运行时：删除 `GroupOneOf` 实现；
    - 测试：用例改为 `alt(...)`。
- 迁移指引
  - 全局替换：`one_of(` → `alt(`。

变更主题：预处理步骤标准化（不再兼容旧名）

- 变更内容
  - 本记录为中间过渡状态，已被 2025-10-24 的“命名空间化”变更取代（见下）。
  - 最终仅支持命名空间写法：`decode/base64`、`unquote/unescape`、`decode/hex`。
  - 兼容旧名的装配逻辑已移除；出现旧名将报 `UnSupport(<name>)`。
- 受影响范围
  - 实现：`crates/wp-lang/src/eval/runtime/vm_unit.rs` 仅匹配命名空间名；
  - 文档：`docs/10-user/06-wpl/01-wpl_basics.md`、`docs/10-user/06-wpl/02-wpl_grammar.md`、`docs/10-user/06-wpl/README.md` 已统一为命名空间写法。
- 迁移指引
  - 规则中替换：
    - `|base64|` → `|base64_decode|`
    - `|esc_quota|`（或 `|unquote|`）→ `|unquote_unescape|`
    - `|hex|` → `|hex_decode|`
  - 快速排查：`rg -n "\|(base64|hex|esc_quota)\|" <your-rules-root>`
  - 提醒：配置文件中的 `encode = "hex"`（例如 `usecase/use_plugin/source/wpsrc.toml`）为数据源编码配置，非 WPL 步骤名，不在本次变更范围。

变更主题：注解值范围放宽（tag 值支持通用引号字符串）

- 变更内容
  - `tag(key:"val")` 的 `val` 从受限 key 改为通用 `quoted_string`（可包含空格、中文与转义）。
  - 解析器：`utils::take_tag_kv` 改用 `quot_str` 解析引号字符串。
- 受影响范围
  - 规范：`docs/10-user/06-wpl/02-wpl_grammar.md` 将 `tag_kv` 改为 `key ":" (quoted_string | raw_string)`。
  - 基础：`docs/10-user/06-wpl/01-wpl_basics.md` 增加示例（中文、转义）。
  - 测试：`crates/wp-lang/src/parser/wpl_anno.rs` 增加空格/中文/转义用例。
- 兼容性
  - 向后兼容：旧写法（值为简化 key）仍可解析；现在可接受更宽字符集的值。

## 计划中（未落地）
- EBNF 进一步收敛（不影响现有语法；落地前会另行公告）：
  - 为 `ExactPath` 明确不包含 `[` `]` `*` 的字符集；
  - `Sep` 以标准 EBNF 形式表述（`Sep = SepChar , { SepChar }`，`SepChar = '\\' , AnyChar`）；
  - `SymbolContent` 的字符类与转义在 EBNF 中补充更精细的定义；
  - `SubField` 去除 `Length`（当前实现仅顶层字段解析该项）。
- 提案（待评审）：关键词与符号规范化
  - 文档：`docs/10-user/06-wpl/PROPOSAL_keywords_symbols.md`
  - 要点：group_meta 别名、预处理步骤命名空间化（已落地）、类型别名建议、sep/format 语法糖、数量词语法糖、原始字符串扩展、ReservedKeyword 与 ident/path_ident 规范化、错误信息建议。
  - 进展：
    - group_meta 规范名切换为：`alt` `opt` `some_of` `seq`；`order` 视为历史写法（解析兼容、文档主推 `seq`）。
    - 类型别名试点保留：`time/timestamp`(=time_timestamp) `time/epoch`(=time_timestamp) `time/rfc3339`(=time_3339) `time/rfc2822`(=time_2822) `json/strict`(=exact_json) `proto/text`(=proto_text) `http/user_agent`(=http/agent) `object`(=obj) `symbol/peek`(=peek_symbol)

- 复用与模块化（提案）
  - 动机：大规则复用差，复制粘贴多；希望抽出公共片段在多处复用，便于团队协作与维护。
  - 建议：提供宏/模板与包含能力
    - 宏：`macro name(args){ body }`；在 rule 表达式内调用并展开。
      - 调用建议：`name!(args)`，避免与类型/函数同名冲突。
      - 形态标注（可选）：`macro(group|fields|subfields|express) name(args){ ... }`，便于在对应位置校验展开结果。
      - 参数替换：支持 `$name`/`$1` 形式占位符，替换为字面 WPL 片段；字符串参数推荐用 `r#"..."#`。
      - 命名空间：包级定义与引用，支持 `pkg::name!(...)`；文件内优先、包内可见、跨文件可 `include`。
      - 限制：禁止递归展开；最大展开深度（如 32）；错误回溯保留“宏源位置”。
    - 包含：`include "path.wpl"` 将宏与规则导入当前包上下文（冲突同名报错或显式覆盖）。
  - 详细草案与示例见：`docs/10-user/06-wpl/DSL.md#宏与模块化复用`。

### DSL 设计改进提案（路线图）
- 量词语法糖：支持 `field?`、`field+`、`field{n}`、`field{n,m}`；组级提供 `repeat(group){n,m}`/`some_of(elem){n,m}`。旧写法（`N*`、`^N`）继续有效。
- 原始字符串字面量：增加 `r"..."`，在注解值/格式等处减少转义负担。
- 错误上下文增强：在 json/kv/array 等协议解析失败时，错误上下文带上“字段路径+组/字段索引+预期类型”（例如 `group[2]/field[5] <json> $.items/[3]/name`）。
- 组合子统一：新增别名 `choice(...)`（等价 `alt(...)`）、`optional(elem)`（等价 `opt(elem)`）、`repeat(elem,min=?,max=?)`（覆盖部分 `some_of` 用法），文档统一范式。
- 严格/宽松模式：在字段/协议级支持 `strict(true|false)` 或注解 `#[strict]`，宽松模式下保留原值或跳过并记录索引。
- 模块化复用：提供 `include "path.wpl"` 与宏 `macro name(args){...}`，支持参数化复用。
- Array 可配置范围/分隔（按需）：最小支持自定义分隔符或范围 `<beg,end>`；默认行为保持现状。
- 联合/自动子类型（按需）：`array/(ip|digit)` 或 `array/alt(ip,digit)`；`array/auto` 仅在白名单基础类型上尝试，注意性能。
- 深度/长度防御：为 array 增加最大元素/最大嵌套限制（与 json 的 MAX_DEPTH 思路一致）。
- 版本化与弃用策略：增加 `#![wpl(version="2")]` 或文件头版本字段；对弃用项在文档与工具中提供提示与脚本迁移。
- 工具链：AST 级格式化、语言服务器（诊断/补全/跳转）、快速模拟器（仅预处理/字段试跑）。

---

如需在你的规则仓库中批量迁移旧写法，建议：
- 先全局搜索 `@plugin(.*rule:` 并逐条改为代码块；
- 本地验证：`cargo test --workspace --all-features`；
- 端到端用例：`usecase/core/getting_started/case_verify.sh`（如涉及）。

遇到问题可附带报错与片段，提交到 PR 或 issue 以便定位。
## 2025-10-24

变更主题：新增原始字符串字面量 r#"..."#（并兼容 r"..." 旧写法）

- 变更内容
  - 语法支持 `r#"..."#` 原始字符串，内部不处理任何转义（例如 `\\`、`\"` 等保持原样），可直接包含 `"`；
  - 兼容旧写法 `r"..."`（仅为迁移期保留，后续可能移除）。
  - 使用场景：注解值、KV/JSON 等协议的字符串值、chars/bool 等读取字符串的场合；便于书写包含大量反斜杠或引号的文本。
- 解析实现
  - 新增解析器：`utils::quot_r_str`；
  - 接入位置：
    - 分隔读取：`crates/wp-lang/src/ast/syntax/wpl_sep.rs` 在引号优先与读取分支增加 `quot_r_str`；
    - 基础解析：`base/chars.rs`、`base/bool.rs` 的字符串分支支持原始字符串；
    - 协议解析：`protocol/{mod.rs,keyval.rs}` 允许 r"..."；
    - 注解解析：`utils::take_tag_kv` 允许 `quoted_string | raw_string`，且仅对普通字符串做转义解码。
- 文档更新
  - `docs/10-user/06-wpl/02-wpl_grammar.md`：`raw_string` 改为 `r#"..."#`；`tag_kv` 支持 `quoted_string | raw_string`；
  - `docs/10-user/06-wpl/02-wpl_grammar.md`：补充对原始字符串的说明；
  - `docs/10-user/06-wpl/01-wpl_basics.md`：在注解示例中给出 `r#"..."#` 用法；预处理步骤采用命名空间写法（`decode/base64`、`decode/hex`、`unquote/unescape`）。
- 兼容性
  - 完全向后兼容：不影响既有 `"..."` 写法；`r"..."` 为新增可选语法。

变更主题：引号字符串与注解值转义修复（Unicode 与 \xHH）

- 变更内容
  - `quoted_string` 放宽为“除 '"' 和 '\\' 外的任意字符”，支持完整 Unicode；
  - 注解 `tag` 值的反转义支持 `\"` `\\` `\n` `\t` `\r` 以及十六进制 `\xHH`；
- 实现
  - `parser/utils.rs::quot_str_impl` 改为 `none_of(['\\','"'])` + `take_escaped`；
  - `decode_escapes` 增加对 `\xHH` 的解析并按 UTF-8 组装；
- 风险
  - 行为更贴近直觉；若依赖旧的 ASCII 限制（极不常见），需按新规范调整。

变更主题：分组关键字规范化（seq 作为规范名）

- 变更内容
  - 文档主推 `seq`（历史别名 `order` 已移除）；解析器仅接受 `seq`。
  - 错误标签中移除已弃用示例（one_of），更新为 `alt,opt,some_of,seq`；
- 兼容性
  - 不再接受 `order(...)`；请统一替换为 `seq(...)`。

变更主题：预处理步骤命名空间化（移除 base64_decode/unquote_unescape/hex_decode）

- 变更内容
  - 仅支持命名空间写法：`decode/base64`、`unquote/unescape`、`decode/hex`。
  - 移除旧名：`base64_decode`、`unquote_unescape`、`hex_decode`（出现旧名将报 `UnSupport(<name>)`）。
- 迁移指引
  - 规则中替换：
    - `|base64_decode|` → `|decode/base64|`
    - `|unquote_unescape|` → `|unquote/unescape|`
    - `|hex_decode|` → `|decode/hex|`
- 受影响范围
  - 实现：`crates/wp-lang/src/eval/runtime/vm_unit.rs` 仅匹配命名空间名；各内置步骤的 `name()` 输出改为命名空间名，便于日志统一。
  - 文档：`docs/10-user/06-wpl/01-wpl_basics.md`、`README.md` 示例统一为命名空间写法；`02-wpl_grammar.md` 注解示例同步。
