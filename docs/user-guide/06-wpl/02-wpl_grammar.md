# WPL 语法（EBNF）

本页给出 WPL 的形式化语法。权威实现以 `crates/wp-lang` 解析器为准；此处与源代码保持同步，并在必要处补充备注。

```ebnf
; WPL 语法（EBNF）
; 基于 crates/wp-lang 下解析实现（winnow）整理
; 说明：本文件给出语法产生式与必要的词法约定。除显式标注外，token 之间允许可选空白 `ws`。

wpl_document     = { package_decl } ;

package_decl     = [ annotation ] "package" ws? ident ws? "{" ws? rule_decl+ ws? "}" ;

rule_decl        = [ annotation ] "rule" ws? rule_name ws? "{" ws? statement ws? "}" ;

statement        = plg_pipe_block | express ;

plg_pipe_block   = ["@"]? "plg_pipe" ws? "(" ws? "id" ws? ":" ws? key ws? ")" ws? "{" ws? express ws? "}" ;

express          = [ preproc ] group { ws? "," ws? group } ;

preproc          = "|" ws? preproc_step { ws? "|" ws? preproc_step } ws? "|" ;   ; 至少一个步骤，且以 '|' 结尾
preproc_step     = builtin_preproc | plg_pipe_step ;
builtin_preproc  = ns "/" name ;
plg_pipe_step    = "plg_pipe" ws? "/" ws? key ;                   ; 通过注册表查找自定义扩展
ns               = "decode" | "unquote" ;                        ; 命名空间白名单
name             = ("base64" | "hex") | "unescape" ;             ; 步骤名白名单

group            = [ group_meta ] ws? "(" ws? field_list_opt ws? ")" [ ws? group_len ] [ ws? group_sep ] ;
group_meta       = "alt" | "opt" | "some_of" | "seq" ;
group_len        = "[" number "]" ;
group_sep        = sep ;

; 列表：允许空、允许尾随逗号
field_list_opt   = [ field { ws? "," ws? field } [ ws? "," ] ] ;

field            = [ repeat ] data_type [ symbol_content ]
                   [ subfields ]
                   [ ":" ws? var_name ]
                   [ length ]
                   [ format ]
                   [ sep ]
                   { pipe } ;                              ; 允许多个管道

repeat           = [ number ] "*" ;                        ; "*ip" 或 "3*ip"
length           = "[" number "]" ;                       ; 仅顶层字段支持（子字段不支持）

; 复合字段（如 kv/json 等）的子字段列表
subfields        = "(" ws? subfields_opt ws? ")" ;
subfields_opt    = [ subfield { ws? "," ws? subfield } [ ws? "," ] ] ;
subfield         = [ opt_datatype | data_type ]
                   [ symbol_content ]
                   [ "@" ref_path ]
                   [ ":" ws? var_name ]
                   [ format ]
                   [ sep ]
                   { pipe } ;

opt_datatype     = "opt" "(" ws? data_type ws? ")" ;     ; 声明该子字段为可选

; 字段数据类型（与 wp-data-model::DataType 对应）
data_type        = builtin_type | ns_type | array_type ;

builtin_type     = "auto" | "bool" | "chars" | "symbol" | "peek_symbol"
                   | "digit" | "float" | "_" | "sn"
                   | "time" | "time_iso" | "time_3339" | "time_2822" | "time_timestamp"
                   | "ip" | "ip_net" | "domain" | "email" | "port"
                   | "hex" | "base64"
                   | "kv" | "json" | "exact_json"
                   | "url"
                   | "proto_text" | "obj"
                   | "id_card" | "mobile_phone" ;

ns_type          = path_ident ;                               ; 例如 http/request、http/status 等
; 注：实现层面建议对白名单前缀（如 "http/"）做校验，以避免任意路径膨胀语言面。

array_type       = "array" [ "/" key ] ;                 ; 如："array" 或 "array/ip"

; 仅当 data_type 为 symbol/peek_symbol 时允许携带内容
symbol_content   = "(" symbol_chars ")" ;

; 字段显示/抽取格式
format           = scope_fmt | quote_fmt | field_cnt ;
scope_fmt        = "<" any_chars "," any_chars ">" ;   ; 作用域首尾定界，如 <[,]>
quote_fmt        = '"' ;                                ; 等价首尾均为 '"'
field_cnt        = "^" number ;                          ; 仅 chars/_ 合法（实现约束）

; 分隔符（高/中优先级，原样拼接）。语法为反斜杠转义的字符序列，长度>=1
sep              = sep_char , { sep_char } ;             ; 例："\\," => ","；"\\!\\|" => "!|"
sep_char         = '\\' , any_char ;

; 字段级管道：函数调用或嵌套分组
pipe             = "|" ws? ( fun_call | group ) ;

; 预置函数（wpl_fun）：
fun_call         = exists | exists_chars | chars_not_exists | exists_chars_in
                   | exists_digit | exists_digit_in | exists_ip_in | str_mode ;
exists           = "exists" "(" ws? key ws? ")" ;
exists_chars     = "exists_chars" "(" ws? key ws? "," ws? path ws? ")" ;
chars_not_exists = "chars_not_exists" "(" ws? key ws? "," ws? path ws? ")" ;
exists_chars_in  = "exists_chars_in" "(" ws? key ws? "," ws? path_array ws? ")" ;
exists_digit     = "exists_digit" "(" ws? key ws? "," ws? number ws? ")" ;
exists_digit_in  = "exists_digit_in" "(" ws? key ws? "," ws? number_array ws? ")" ;
exists_ip_in     = "exists_ip_in" "(" ws? key ws? "," ws? ip_array ws? ")" ;
str_mode         = "str_mode" "(" ws? free_string ws? ")" ;    ; 读到 ',' 或 ')' 截止

path_array       = "[" ws? path { ws? "," ws? path } ws? "]" ;
number_array     = "[" ws? number { ws? "," ws? number } ws? "]" ;
ip_array         = "[" ws? ip_addr { ws? "," ws? ip_addr } ws? "]" ;

annotation       = "#[" ws? ann_item { ws? "," ws? ann_item } ws? "]" ;
ann_item         = tag_anno | copy_raw_anno ;
tag_anno         = "tag" "(" ws? tag_kv { ws? "," ws? tag_kv } ws? ")" ;
tag_kv           = ident ":" ( quoted_string | raw_string ) ;      ; 键为标识符；值为字符串
copy_raw_anno    = "copy_raw" "(" ws? "name" ws? ":" ws? ( quoted_string | raw_string ) ws? ")" ;

; 词法与辅助记号 --------------------------------------------------------
field_name       = var_name ;
rule_name        = exact_path ;
key              = key_char { key_char } ;              ; [A-Za-z0-9_./-]+
var_name         = var_char { var_char } ;              ; [A-Za-z0-9_.-]+
ref_path         = ref_char { ref_char } ;              ; [A-Za-z0-9_./\-.[\]*]+
; 标识符与路径标识符（推荐写法）
ident            = ( letter | '_' ) { letter | digit | '_' | '.' | '-' } ;
path_ident       = ident { "/" ident } ;

exact_path       = exact_path_char { exact_path_char } ; ; 不含 '[' ']' '*'
exact_path_char  = letter | digit | '_' | '.' | '/' | '-' ;
path             = key | ref_path ;

number           = digit { digit } ;
digit            = '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' ;

key_char         = letter | digit | '_' | '.' | '/' | '-' ;
var_char         = letter | digit | '_' | '.' | '-' ;
ref_char         = key_char | '[' | ']' | '*' ;

letter           = 'A'..'Z' | 'a'..'z' ;

quoted_string    = '"' { escaped | char_no_quote_backslash } '"' ;
raw_string       = 'r' '#' '"' { any_char } '"' '#' ;          ; r#"..."#，内部不处理转义（内容可包含 '"'）
char_no_quote    = ? any char except '"' ? ;
escaped          = '\\' ( '"' | '\\' | 'n' | 't' | 'r' | 'x' hex hex ) ;
char_no_quote_backslash = ? any char except '"' and '\\' ? ;
hex              = '0'..'9' | 'a'..'f' | 'A'..'F' ;

free_string      = { fchar } ;                          ; 直至 ',' 或 ')'（不含）
fchar            = ? any char except ',' and ')' ? ;

symbol_chars     = { schar } ;                          ; 允许除 ')' 与 '\\' 外字符，或使用 '\)' 转义
schar            = char_no_close_paren_backslash | '\\' ')' ;
char_no_close_paren_backslash = ? any char except ')' and '\\' ? ;
any_chars        = { any_char } ;
any_char         = ? any character ? ;

ip_addr          = quoted_string | ipv4 | ipv6 ;        ; 支持 IPv4/IPv6 裸字面量或带引号
ipv4             = digit1 "." digit1 "." digit1 "." digit1 ;
digit1           = digit { digit } ;
ipv6             = ? valid IPv6 literal (RFC 4291), including compressed forms like "::1" ? ;

ws               = { ' ' | '\t' | '\n' | '\r' } ;

;保留关键字（不可作为标识符使用；由实现侧进行冲突校验）
ReservedKeyword  = "package" | "rule" | "alt" | "opt" | "some_of" | "seq" | "order"
                 | "tag" | "copy_raw" | "include" | "macro" ;


```
## 语义与实现注意事项（非语法）：
 - preproc 管道（例如 |decode/base64|unquote/unescape|）出现在 express 起始处，独立于字段级 pipe。
 - group 后可跟 [n] 与分隔符 sep：长度会应用到组内所有字段；sep 仅存储在组上，具体组合策略见实现。
 - format 中的 field_cnt（^n）仅适用于 chars/_ 类型；其它类型将被拒绝（实现约束）。
 - symbol/peek_symbol 可携带 symbol_content，如 symbol(boy)；peek_symbol 等价于 symbol，且仅改变“窥探”语义。
 - subfields 中未显式 "@ref" 时，键默认为 "*"（通配键）。
 - sep 写法需以反斜杠转义每个字符；例如 \\!\\| 代表字符串 "!|"。
 - annotation 可用于 package 与 rule；若同时存在，会在 rule 侧合并（rule 优先）。
