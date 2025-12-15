# OML 语法（EBNF）

本文基于源码 crates/wp-oml 的解析实现（winnow 解析器组合器）梳理了 OML 的实际可用语法，并以 EBNF 形式给出。词法细节（如数据类型、JSON 路径、SQL 运算符等）复用 `wp_parser` 与 `wpl` 的既有解析能力，本文在必要处做抽象约定。

> 相关实现入口：`crates/wp-oml/src/parser/oml_conf.rs`、`oml_aggregate.rs`、`match_prm.rs`、`map_prm.rs`、`pipe_prm.rs`、`sql_prm.rs`、`fmt_prm.rs` 等。

## 顶层结构

```ebnf
oml              = header, sep_line, aggregate_items, [ sep_line, privacy_items ] ;

header           = "name", ":", name, eol,
                   [ "rule", ":", rule_path, { rule_path }, eol ] ;
sep_line         = "---" ;
name             = path ;                       (* 例如: test *)
rule_path        = wild_path ;                  (* 例如: wpx/abc, wpx/efg *)

aggregate_items  = aggregate_item, { aggregate_item } ;
aggregate_item   = target_list, "=", eval, ";" ;

target_list      = target, { ",", target } ;
target           = target_name, [ ":", data_type ] ;
target_name      = wild_key | "_" ;            (* 允许带通配符 '*'；'_' 表示匿名/丢弃 *)
data_type        = type_ident ;                 (* 复用 wpl::take_datatype: auto|ip|chars|digit|time|obj|array|... *)
```

## 求值（右侧表达式）

```ebnf
eval             = take_expr
                 | read_expr
                 | fmt_expr
                 | pipe_expr
                 | map_expr
                 | collect_expr
                 | match_expr
                 | sql_expr
                 | value_expr
                 | fun_call ;

(* 变量获取：take/read 支持统一参数形态；可跟缺省体 *)
take_expr        = "take", "(", [ arg_list ], ")", [ default_body ] ;
read_expr        = "read", "(", [ arg_list ], ")", [ default_body ] ;
; 说明：'@' 仅作为变量获取语法糖用于 fmt/pipe/collect 的 var_get 位置，不支持缺省体，不作为独立求值表达式

arg_list         = arg, { ",", arg } ;
arg              = "option", ":", "[", key, { ",", key }, "]"
                 | ("in"|"keys"), ":", "[", key, { ",", key }, "]"
                 | "get",    ":", simple
                 | json_path ;                 (* 见 wp_parser::atom::take_json_path *)

default_body     = "{", "_", ":", gen_acq, [ ";" ], "}" ;
gen_acq          = take_expr | read_expr | value_expr | fun_call ;

(* 常量值：类型名+括号包裹的字面量 *)
value_expr       = data_type, "(", literal, ")" ;

(* 内置函数（零参占位）：Time::* 家族 *)
fun_call         = ("Time::now"
                   |"Time::now_date"
                   |"Time::now_time"
                   |"Time::now_hour"), "(", ")" ;

(* 字符串格式化，至少 1 个参数 *)
fmt_expr         = "fmt", "(", string, ",", var_get, { ",", var_get }, ")" ;
var_get          = ("read" | "take"), "(", [ arg_list ], ")"
                 | "@", ident ;                  (* '@ref' 等价 read(ref)，不支持缺省体 *)

(* 管道 *)
pipe_expr        = ["pipe"], var_get, "|", pipe_fun, { "|", pipe_fun } ;   (* var_get 支持 '@ref' *)
pipe_fun         = "arr_get",    "(", unsigned, ")"
                 | "obj_get",    "(", ident,   ")"
                 | "base64_de",  "(", [ encode_type ], ")"
                 | "sxf_get",    "(", alnum*,  ")"
                 | "path_get",   "(", ("name"|"path"|"default"), ")"
                 | "url_get",    "(", ("domain"|"host"|"uri"|"path"|"params"|"default"), ")"
                 | "to_timestamp_zone", "(", [ "-" ], unsigned, ",", ("ms"|"us"|"ss"|"s"), ")"
                 | "base64_en" | "html_escape_en" | "html_escape_de"
                 | "str_escape_en" | "json_escape_en" | "json_escape_de"
                 | "to_timestamp" | "to_timestamp_ms" | "to_timestamp_us"
                 | "to_json" | "to_string" | "skip_if_empty" ;

encode_type      = ident ;                     (* 例如: Utf8/Gbk/... *)

(* 聚合到对象：map 内部为子赋值序列；分号可选但推荐 *)
map_expr         = "object", "{", map_item, { map_item }, "}" ;
map_item         = map_targets, "=", sub_acq, [ ";" ] ;
map_targets      = ident, { ",", ident }, [ ":", data_type ] ;
sub_acq          = take_expr | read_expr | value_expr | fun_call ;

(* 聚合到数组：从 VarGet 收集（支持 in/option 通配） *)
collect_expr     = "collect", var_get ;

(* 模式匹配：单源/双源两种形态，支持 in/!=/== 与缺省分支 *)
match_expr       = "match", match_source, "{", case1, { case1 }, [ default_case ], "}"
                 | "match", "(", var_get, ",", var_get, ")", "{", case2, { case2 }, [ default_case ], "}" ;
match_source     = var_get ;
case1            = cond1, "=>", calc, [ "," ], [ ";" ] ;
case2            = "(", cond1, ",", cond1, ")", "=>", calc, [ "," ], [ ";" ] ;
default_case     = "_", "=>", calc, [ "," ], [ ";" ] ;
calc             = read_expr | take_expr | value_expr | collect_expr ;

cond1            = "in", "(", value_expr, ",", value_expr, ")"
                 | "!", value_expr
                 | value_expr ;                 (* 省略运算符表示等于 *)
```

## SQL 表达式

```ebnf
sql_expr        = "select", sql_body, "where", sql_cond, ";" ;
sql_body        = sql_safe_body ;              (* 源码对白名单化：仅 [A-Za-z0-9_.] 与 '*' *)
sql_cond        = cond_expr ;
cond_expr       = cmp, { ("and" | "or"), cmp }
                 | "not", cond_expr
                 | "(", cond_expr, ")" ;
cmp             = ident, sql_op, cond_rhs ;
sql_op          = sql_cmp_op ;                 (* 见 wp_parser::sql_symbol::symbol_sql_cmp *)
cond_rhs        = read_expr | take_expr | fun_call | sql_literal ;
sql_literal     = number | string ;
```

严格模式说明：
- 严格模式（默认开启）：当主体 `<cols from table>` 不满足白名单规则时，解析报错（不再回退原文）。
- 兼容模式：设置环境变量 `OML_SQL_STRICT=0`，若主体非法则回退原文（不推荐）。
- 白名单规则：
  - 列清单：`*` 或由 `[A-Za-z0-9_.]+` 组成的列名（允许点号作限定）；不支持函数、聚合、别名。
  - 表名：`[A-Za-z0-9_.]+`（单表，不支持 join/子查询）。
  - `from` 大小写不敏感；多余空白允许。

错误示例（严格模式）：
- `select a, b from table-1 where ...` → invalid table 标识符（含 `-`）。
- `select sum(a) from t where ...` → 列清单含函数。
- `select a from t1 join t2 ...` → 不支持 join。

## 隐私段（说明）
注：引擎默认不启用运行期隐私/脱敏处理；以下为 DSL 语法能力说明，供需要的场景参考。

```ebnf
privacy_items   = privacy_item, { privacy_item } ;
privacy_item    = ident, ":", privacy_type ;
privacy_type    = "privacy_ip" | "privacy_specify_ip" | "privacy_id_card" | "privacy_mobile"
                 | "privacy_mail" | "privacy_domain" | "privacy_specify_name"
                 | "privacy_specify_domain" | "privacy_specify_address"
                 | "privacy_specify_company" | "privacy_keymsg" ;
```

## 词法与约定（来自 wp_parser/wpl 抽象）

```ebnf
path            = ident, { ("/" | "."), ident } ;
wild_path       = path | path, "*" ;          (* 允许通配 *)
wild_key        = ident, { ident | "*" } ;    (* 允许 '*' 出现在键名中 *)
type_ident      = ident ;                      (* 如 auto/ip/chars/digit/time/obj/array/... *)
ident           = letter, { letter | digit | "_" } ;
key             = ident ;
string          = "\"", { any-but-quote }, "\"" ;
literal         = string | number | ip | bool | datetime | ... ;
json_path       = "/" , ... ;                 (* 如 /a/b/[0]/1 *)
simple          = ident | number | string ;
unsigned        = digit, { digit } ;
eol             = { " " | "\t" | "\r" | "\n" } ;

letter          = "A" | ... | "Z" | "a" | ... | "z" ;
digit           = "0" | ... | "9" ;
alnum           = letter | digit ;
```

## 典型示例（与实现一致）

```oml
name : csv_example
---
# 基本取值与缺省
version : chars = Time::now() ;
pos_sn           = read() { _ : chars(FALLBACK) };

# map 聚合
values : obj = object {
  cpu_free, memory_free : digit = read();
};

# collect 数组聚合 + 管道
ports : array = collect read(keys:[sport,dport]);
ports_json      = pipe read(ports) | to_json ;
first_port      = pipe read(ports) | arr_get(0) ;

# match
quarter : chars = match read(month) {
  in (digit(1), digit(3))   => chars(Q1),
  in (digit(4), digit(6))   => chars(Q2),
  in (digit(7), digit(9))   => chars(Q3),
  in (digit(10), digit(12)) => chars(Q4),
  _ => chars(QX) ;
};

# SQL（where 中可混用 read/take/Time::now/常量）
name,pinying = select name,pinying from example where pinying = read(py) ;
---
# 隐私配置（按键绑定处理器枚举）
src_ip : privacy_ip
pos_sn : privacy_keymsg
```

## 备注
- 注释：源码通过 CommentParser 预处理支持 `//` 单行注释。
- 目标通配：当目标名含 `*` 时走批量模式（BatchEval），对应实现见 `oml_aggregate.rs`。
- 语法错误提示：关键位置均带上下文与期望串，以便定位（参见 `keyword.rs` 与各 parser `.context(...)`）。
- 本文档仅刻画实际生效语法；示例与 tests 完整对齐，可参阅 `crates/wp-oml/tests/test_case.rs`。
- 读取语义：`read` 为非破坏性（可反复读取，不从 src 移除）；`take` 为破坏性（取走后从 src 移除，后续不可再取）。
