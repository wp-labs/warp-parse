# OML 使用示例

这里给出与实现一致的 OML 示例，覆盖常见能力：取值与缺省、map/collect、match、pipe、fmt、SQL、批量目标、隐私段等。

> 语法参考：见《OML 语法（EBNF）》；解析实现：`crates/wp-oml/src/parser/*`。

## 1. 最小可运行示例

```oml
name : minimal
---
# 从输入记录读取 user_id；未指定类型时默认为 auto
user_id = read(user_id) ;
# 调用内置时间函数
occur_time : time = Time::now() ;
```

要点：
- 顶部 `name : <名字>`，紧随三杠分隔线 `---` 后进入条目区；每条以分号结尾。
- 目标未显式声明类型时，默认为 `auto`。

## 2. 取值（read/take）与缺省体

```oml
name : defaults
---
# 直接读字段；字段不存在时，走缺省体 `_ : <获取/常量/函数>`
pos_sn = read() { _ : chars(FALLBACK) } ;
# 指定获取键（等价于 read(get:user)）
user = read(user) ;
# JSON 路径也可直接写入括号：
path_val = read(/details[0]/name) ;
# 注：历史别名已隐藏于文档，解析保留兼容（不推荐使用）
```

参数说明：
- 仅支持四类参数：`option:[k1,k2,...]`、`in:[k1,k2,...]`、`get:<simple>`、JSON 路径（如 `/a/b/[0]/c`）。

## 3. map（对象聚合）

```oml
name : object_example
---
values : obj = object {
  # 多目标 + 指定类型
  cpu_free, memory_free : digit = read() ;
  # 也可省略类型（默认为 auto）
  process, disk_used = read() ;
};
```

说明：
- `object { ... }` 内部为一组 `targets = <取值>` 的子赋值，分号可省略但推荐保留。

## 4. collect（数组聚合）

```oml
name : collect_example
---
# 从多个字段收集为数组
ports : array = collect read(keys:[sport,dport]) ;
# 使用通配可收集一批：
all_vals : array = collect read(keys:[metrics/*]) ;
```

## 5. match（模式匹配）

单源匹配：
```oml
name : match_single
---
quarter : chars = match read(month) {
  in (digit(1), digit(3))   => chars(Q1),
  in (digit(4), digit(6))   => chars(Q2),
  in (digit(7), digit(9))   => chars(Q3),
  in (digit(10), digit(12)) => chars(Q4),
  _ => chars(UNKNOWN) ;
};
```

双源匹配：
```oml
name : match_double
---
X : chars = match ( read(city1), read(city2) ) {
  (ip(127.0.0.1), ip(127.0.0.100)) => chars(bj),
  _ => chars(sz) ;
};
```

注意：分支末尾逗号和分号均为可选，建议统一使用分号。

## 6. pipe（管道）

```oml
name : pipe_example
---
ports : array = collect read(keys:[sport,dport]) ;
ports_json = read(ports) | to_json ;
first_port = read(ports) | arr_get(0) ;
raw_uri     = read(http_uri) ;
host        = read(raw_uri) | url_get(host) ;
# 时间戳转换，支持时区和单位
occur_ms    = read(occur_time) | to_timestamp_zone(0,ms) ;
# Base64 编解码（可选编码参数，缺省为 Utf8）
raw_b64     = read(payload) | base64_en ;
payload     = read(raw_b64) | base64_de(Utf8) ;
```

## 7. fmt（格式化字符串）

```oml
name : fmt_example
---
# 支持 @ref（等价于 read(ref)）与 read/get 混用
full = fmt("{}-{}", @user, read(city)) ;
```

说明：`fmt` 至少需要 1 个参数；`@ref` 在此处仅作为 `read(ref)` 的语法糖，不支持缺省体。

## 8. SQL（查询）

```oml
name : sql_example
---
# where 中可使用 read/take/Time::now/常量
name,pinying = select name,pinying from example where pinying = read(py) ;

# 使用 KnowDB 内置 UDF（例如 IPv4 区间匹配），推荐整数比较写法
from_zone = select zone from zone
  where ip_start_int <= ip4_int(read(src_ip))
    and ip_end_int   >= ip4_int(read(src_ip)) ;
```

提示：实现对 `select <列 from 表>` 做了基础合法化（仅允许 `[A-Za-z0-9_.]` 与 `*`），建议显式列出列与单表名。

## 9. 批量目标（通配符 *）

```oml
name : batch_example
---
# 目标名含 * 时进入批量模式；右值仅支持 take/read
aler* : auto = take() ;
```


## 11. 注释

```oml
name : with_comment
---
// 单行注释会在解析前被移除
version = chars(1.0.0) ;
```

## 12. 组合示例（多能力同用）

```oml
name : csv_example
---
# 取值与缺省
version : chars = Time::now() ;
pos_sn           = read() { _ : chars(FALLBACK) } ;

# map 聚合
values : obj = object {
  cpu_free, memory_free : digit = read() ;
};

# collect + pipe
ports : array = collect read(keys:[sport,dport]) ;
ports_json      = read(ports) | to_json ;
first_port      = read(ports) | arr_get(0) ;

# match
quarter : chars = match read(month) {
  in (digit(1), digit(3))   => chars(Q1),
  in (digit(4), digit(6))   => chars(Q2),
  in (digit(7), digit(9))   => chars(Q3),
  in (digit(10), digit(12)) => chars(Q4),
  _ => chars(QX) ;
};

# SQL
a2,b2  = select name,pinying from example where pinying = read(py) ;
---
# 隐私（说明）
引擎默认不启用运行期隐私/脱敏处理；以下示例仅演示 OML 语言的隐私段写法。
src_ip : privacy_ip
pos_sn : privacy_keymsg
```

## 13. 易错提醒
- 参数键仅支持：`option`、`in`、`get` 与 JSON 路径，其他键不被支持。
- 顶层条目必须以分号 `;` 结束；`map` 内分号建议保留以统一风格。
- `match` 分支的逗号与分号在实现中都是可选的，建议统一使用分号。
- 目标带 `*` 时进入批量模式，右值限定为 `take/read`。
- `@ref` 仅在 fmt/pipe/collect 的 `var_get` 位置作为 `read(ref)` 的语法糖使用，不支持缺省体；不作为独立右值表达式。

## 14. `read` 与 `take` 的差异（非破坏 vs 破坏）

```oml
name : read_take_diff
---
# 准备：src 中存在 A1=hello, B1=world

# 非破坏性读取：可重复
X1 = read(A1) ;   # X1 <- hello
X2 = read(A1) ;   # X2 <- hello（仍可读到）

# 破坏性读取：取走后 src 移除
Y1 = take(B1) ;   # Y1 <- world（同时从 src 移除 B1）
Y2 = take(B1) ;   # 取不到（B1 已不在 src）
```

说明：
- `read` 先从目标记录 `dst` 查找，再从输入 `src` 克隆值，不移除原值；
- `take` 从 `src` 取走值（并移除），因此后续再 `take` 同名键将失败；
- 若希望“消费式”读一次，选用 `take`；若希望“只读复制”，选用 `read`。

## 15. 实战示例：HTTP URL 拆解与对象聚合

```oml
name : http_access_model
---
# 1) 基础取值与预处理
raw_uri      = read(http_uri) ;
host         = read(raw_uri) | url_get(host) ;
path         = read(raw_uri) | url_get(path) ;
domain       = read(raw_uri) | url_get(domain) ;

# 2) 端口列表聚合与取第一个
ports : array = collect read(keys:[sport,dport]) ;
first_port    = read(ports) | arr_get(0) ;

# 3) 事件时间（演示 to_timestamp_zone，按 UTC 毫秒输出）
occur_ms : digit = read(occur_time) | to_timestamp_zone(0,ms) ;

# 4) 对象聚合：集中输出核心字段
summary : obj = object {
  s_ip  : ip    = read(src_ip) ;
  d_ip  : ip    = read(dst_ip) ;
  host  : chars = read(host) ;
  path  : chars = read(path) ;
  port  : digit = read(first_port) ;
  t_ms  : digit = read(occur_ms) ;
};

# 5) 字符串拼接展示
title = fmt("{} {}:{}{}", @domain, read(s_ip), read(port), read(path)) ;
---
# 6) 隐私处理：IP 脱敏（示例）
src_ip : privacy_ip
dst_ip : privacy_ip
```

要点：
- `url_get(host|path|domain)` 用于快速拆解 URL；
- `collect read(keys:[...])` 可把多个源字段聚合为数组，便于后续处理；
- `arr_get(0)` 取数组首元素；
- `to_timestamp_zone(0,ms)` 将时间标准化为 UTC 毫秒；
- 对象聚合 `object { ... }` 便于将多字段集中输出为结构化对象；
- 末尾隐私段通过键名绑定处理器（引擎默认未启用）。

## 16. 实战示例：SQL Enrich（基于 IP 的地域信息）

```oml
name : http_access_enrich
---
# 输入拆解（可与 15 同用）
raw_uri  = read(http_uri) ;
host     = read(raw_uri) | url_get(host) ;
path     = read(raw_uri) | url_get(path) ;

# 基于 src_ip 的地域增强（严格模式下，主体需满足白名单：列/表为 [A-Za-z0-9_.]+ 或 '*'）
geo_country, geo_city = select country,city from ip_geo where ip = read(src_ip) ;

# 汇总展示
title = fmt("{} {} {}", @host, read(geo_country), read(geo_city)) ;
```

输入/输出样例（示意）：

| 角色 | 字段       | 值                                  | 说明 |
|------|------------|-------------------------------------|------|
| 输入 | src_ip     | 203.0.113.5                         | 源 IP |
| 输入 | http_uri   | http://example.com/path?a=1&b=2     | 原始 URI |
| 输入 | occur_time | 2025-10-24 12:00:00                 | 事件时间 |
| 输入 | sport      | 514                                 | 源端口（字符串） |
| 输入 | dport      | 80                                  | 目的端口（字符串） |
| 输出 | host       | example.com                          | 解析自 URL |
| 输出 | path       | /path                                | 解析自 URL |
| 输出 | geo_country| CN                                   | SQL enrich 结果 |
| 输出 | geo_city   | Beijing                              | SQL enrich 结果 |
| 输出 | title      | example.com CN Beijing               | 格式化结果 |

提示：
- 需要在运行期配置并初始化查询提供者（KnowDB Provider），SQL 才会返回结果；
- `OML_SQL_STRICT=0` 可临时关闭主体白名单校验用于调试，但不建议在生产使用；
- 多列返回按目标顺序映射：`a,b = select col1,col2 ...` 中 `col1->a`、`col2->b`。
