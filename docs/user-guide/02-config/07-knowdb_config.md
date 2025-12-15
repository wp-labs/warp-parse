# KnowDB 配置指南
<!-- 角色：使用配置者 | 最近验证：2025-12-11 -->

本指南描述知识库（KnowDB）的目录式配置与装载规范。

适用范围
- 初始化权威库（CSV → SQLite），用于 wparse/wproj 等工具在启动时装载

核心原则
- SQL 外置：每张表的 DDL/DML 均放在对应目录下的 .sql 文件中
- 安全：运行期只允许访问配置里声明过的表名；SQL 仅支持 {table} 占位符
- 默认可用：多数字段可省略，内置默认值与自动探测能满足常见场景

目录布局（推荐）
```
models/knowledge/
  knowdb.toml                     # 本配置
  example/
    create.sql
    insert.sql
    # clean.sql                  # 可选；默认 DELETE FROM {table}
    data.csv                     # 单一数据文件（表目录根）
  address/
    create.sql
    insert.sql
    data.csv
```

顶层配置（models/knowledge/knowdb.toml）
```toml
version = 2
# 注：`wproj knowdb init` 生成的是“最小化”配置，不包含以下默认段落；
# 如需修改默认行为，可手动追加：
# base_dir = "."                      # 相对 knowdb.toml 所在目录，默认 "."
# [default]                            # 表级 load 默认（可在每张表覆盖）
# transaction = true
# batch_size  = 2000
# on_error    = "fail"                 # fail | skip
# [csv]                                 # CSV 方言默认（可在每张表覆盖）
# has_header  = true
# delimiter   = ","
# encoding    = "utf-8"
# trim        = true

[[tables]]
name = "example"
# dir 省略时等于 name；此示例即使用目录 models/knowledge/example
# data_file 省略时使用表目录下的 data.csv
columns.by_header = ["name", "pinying"]

# 如需更多表，追加 [[tables]] 段落
```

SQL 文件规范
- create.sql：建表语句，必须存在；可使用占位符 `{table}`；允许包含多条语句（如 `CREATE INDEX`）
- insert.sql：插入语句，必须存在；参数位置用 `?1..?N`；允许 `{table}`
- clean.sql：可选；若不存在，装载前默认执行 `DELETE FROM {table}`

列映射（columns）
- 推荐 `by_header=[..]`，按 CSV 表头名映射到 `insert.sql` 中的列
- 若 `has_header=false`，必须提供 `by_index=[..]`
- 可选增强（实现层）：若未配置 columns，且 `insert.sql` 显式了列清单，可解析 insert 的列名作为 `by_header`

CSV 方言（默认可省略）
- 默认：`has_header=true`、`delimiter=","`、`encoding="utf-8"`（容忍 BOM）、`trim=true`
- 继承：表级 `[[tables.csv]]` 覆盖全局 `[csv]`；未设置则使用默认
- 非 UTF-8 需在 `encoding` 显式指定并确保 Loader 支持（或预处理转码）

装载策略（默认可省略）
- 默认：`transaction=true`、`batch_size=2000`、`on_error="fail"`
- on_error:
  - fail：遇到坏行（缺列/解析失败）即失败回滚
  - skip：跳过坏行并计数告警

自动探测（当 data_file 未配置）
- 使用 `{base_dir}/{tables.dir}/data.csv`
- 不存在则报错

安全约束
- 运行时（facade/query_cipher/SQL 评估）仅允许使用 `[[tables]].name` 中声明的表名
- SQL 模板仅允许 `{table}` 占位符；禁止其它动态拼接

最小可运行示例
1) 目录
```
models/knowledge/knowdb.toml
models/knowledge/example/{create.sql, insert.sql, data.csv}
```
2) create.sql
```sql
CREATE TABLE IF NOT EXISTS {table} (
  id      INTEGER PRIMARY KEY,
  name    TEXT NOT NULL,
  pinying TEXT NOT NULL
);
```
3) insert.sql
```sql
INSERT INTO {table} (name, pinying) VALUES (?1, ?2);
```
4) data.csv
```
name,pinying
令狐冲,linghuchong
任盈盈,renyingying
```
5) knowdb.toml（最小化）
```toml
version = 2
base_dir = "./models/knowledge"
[[tables]]
name = "example"
dir  = "example"
columns.by_header = ["name", "pinying"]
[tables.expected_rows]
min = 1
max = 100
```

常见错误与排障
- 缺少 create.sql / insert.sql：启动时失败并指向缺失文件
- `has_header=false` 但未提供 `by_index`：装载报错
- `expected_rows.min` 未满足：数据不足，装载失败
- 数据源未找到：既未配置 `data_file`，也不存在默认路径 `data.csv`
- 运行期 SQL 访问未声明的表：安全校验失败

与应用的关系
- wparse/wproj 等会在启动处加载 knowdb：创建权威库并设置 Query Provider
- 曾用于隐私模块的 `query_cipher(table)`（加载单列表词表）在当前版本默认不启用；如需脱敏请在业务侧实现

内置 SQL 函数（UDF）
- 运行时注册：
  - 导入阶段（权威库写连接）与查询阶段（线程克隆的只读连接）均自动注册。
  - 可在 `INSERT/SELECT/WHERE` 中直接使用（DDL 不涉及）。
- 签名与语义：
  - `ip4_int(text) -> integer`：点分 IPv4 转 32 位整数；容忍空白/引号；非法返回 `0`。
  - `ip4_between(ip_text, start_text, end_text) -> integer`：是否在闭区间 `[start,end]` 内（1/0）。
  - `cidr4_min(text) -> integer`：CIDR 起始地址（含），如 `10.0.0.0/8`。
  - `cidr4_max(text) -> integer`：CIDR 结束地址（含）。
  - `cidr4_contains(ip_text, cidr_text) -> integer`：IP 是否落在 CIDR 段内（1/0）。
  - `ip4_text(integer|string) -> text`：32 位整数转点分 IPv4（便于调试/展示）。
  - `trim_quotes(text) -> text`：去除两端成对引号（' 或 "），容忍前后空白；未成对则原样返回（去掉空白）。
- 导入示例（insert.sql）：
  ```sql
  INSERT INTO {table} (ip_start_int, ip_end_int, zone)
  VALUES (ip4_int(?1), ip4_int(?2), trim_quotes(?3));
  ```
- 查询示例（普通 SQL）：
  ```sql
  -- 区间命中（推荐整数比较写法，避免在 WHERE 中直接比较函数返回）
  SELECT zone FROM zone
  WHERE ip_start_int <= ip4_int(:ip)
    AND ip_end_int   >= ip4_int(:ip)
  LIMIT 1;

  -- CIDR 命中
  SELECT zone FROM zone
  WHERE cidr4_contains(:ip, :cidr) = 1;

  -- 调试回显
  SELECT ip4_text(ip_start_int) AS ip_start, ip4_text(ip_end_int) AS ip_end, zone
  FROM zone
  LIMIT 5;
  ```
- OML 中的 SQL 精确求值：
  - OML 的 `select … from … where …;` 语法对列段做了标识符白名单限制，不建议在列段直接写函数。
  - 推荐在上游产出数值型 IP（如 `src_ip_int`），在 OML 的 where 中用整数比较：
    ```sql
    from_zone: chars = sql(
      select zone from zone
      where ip_start_int <= read(src_ip_int)
        and ip_end_int   >= read(src_ip_int);
    )
    ```
- 注意事项：
  - 目前非法 IPv4/CIDR 输入返回 `0`（或匹配失败），为提高导入韧性；如需严格行为可定制。
  - SQLite 原生已提供 `lower/upper/trim` 等字符串函数，可与上述 UDF 组合使用。
