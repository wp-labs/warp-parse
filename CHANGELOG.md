# Changelog


[English](./CHANGELOG.en.md) | 中文

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.35.0-alpha] - 2026-10-28

### Added
- **文件 Sink 支持按容量滚动与归档压缩**（升级到 wp-motor v1.38.0）：新增 `max_size`（单文件最大字节数，支持 `10MB` / `1GiB` 写法，`0` = 不限）、`max_rolls`（保留历史文件个数，默认 `3`）与 `roll_compress`（`none` / `gzip` / `zip`）。
  - 超限时先滚动再写，历史文件按 `out.dat.1` / `out.dat.1.gz` / `out.dat.1.zip` 编号，超出保留个数的自动删除最旧文件。
  - 滚动时收尾并重建压缩/加密流，每个历史文件都能独立解压解密；归档压缩在阻塞线程执行，失败自动回退为不压缩归档，不丢数据。
  - `protocol = "arrow"` 不支持滚动，配置 `max_size` / `max_rolls` / `roll_compress` 会直接报配置错误。

### Note
- 现有项目的 connector 定义需把 `max_size` / `max_rolls` / `roll_compress` 加入 `allow_override` 才能覆盖。

## [0.34.2-alpha] - 2026-10-27

### Added
- **升级到 wp-motor v1.37.1**，修复 OML 工程里实例级全局 tag 丢失的问题（见下）。

### Fixed
- **配了 `sink_group.oml` 之后 `WP_GLOBAL_TAGS` 丢失**：OML 会用模型输出重建记录，解析阶段盖上的实例级全局 tag 不再出现在输出里。现在在变换后补盖，`oml = ["..."]` 与不配 OML 的字段集一致。
- **全局 tag 优先级最高**：与模型里写死的同名字段或 sink `tags` 同名时，最终只保留一份并使用 `WP_GLOBAL_TAGS` 的值，不再出现重复键；OML 转换失败进入 error sink 的记录也会带上全局 tag。

## [0.34.1-alpha] - 2026-10-26

### Added
- **升级到 wp-motor v1.37.0**，带上救援链路的两处修复（见下）。

### Fixed
- **救援回放投错 sink 组**：回放原先只按 sink 名匹配目标，自动命名时不同组的同名 sink（都是 `[0]`）都会被当成候选，救援数据按通道余量被分流到别的组。现在从救援文件路径解析组名，按「组 + 名」精确匹配。
- **回放把背压当成 sink 故障**：分发通道满原先被当成发送失败并切换候选，既放大上面的错投，也可能让整份救援文件回放中断。现在改为在同一个候选上等待重试，只有通道关闭等真实错误才换候选。
- **救援生命周期日志看不见**：`rescue file finalized`、`recovered from send failures`、`breaker open, skipped` 统一为 WARN，且每次故障各一条（原来会被 60s 限流吞掉）。

## [0.34.0-alpha] - 2026-10-25

### Changed
- **移除 admin API 配置里的 `[admin_api.auth].mode`**：鉴权方式固定为 Bearer Token，该字段没有实际作用。`wpadm conf` 生成和迁移的配置不再写入 `mode`，`[admin_api.auth]` 现在只剩 `token_file`。
  - 手工保留 `mode = "bearer_token"` 的旧配置在加载时会报 `unknown field`，删掉该行即可。

## [0.33.0-alpha] - 2026-10-23

### Added
- **运行状态目录可配置**：新增 `[runtime].state_dir`（写在 `wparse.local.toml` 或 `conf/wparse.toml`）和 `--state-dir` 参数。默认仍是 `<work_root>/.run`，现有工程无需改动。
  - pid、单实例锁、`rule_mapping.dat`、`authority.sqlite` 以及 `project_remote` 的缓存/备份/state 全部跟随该目录；work root 可以挂成只读，容器可以把状态放到独立卷。
  - 优先级：`--state-dir` > `wparse.local.toml` > `conf/wparse.toml` > `.run`。daemon 启动日志会打印实际生效的目录。
  - 非法值直接启动失败：不能是 work root 本身、它的上级目录，或 `project_remote` 托管目录（`conf`/`models`/`topology`/`connectors`）内部；未展开的 `${VAR}` 同样报错。
- **admin API token 工具**：新增 `wpadm token generate|show|check`；客户端支持 `--token-env` 和 `$WPARSE_ADMIN_TOKEN`，便于容器与 CI 使用。

### Changed
- **token 文件权限改为安全下限**：`0600`/`0640`/`0440`/`0444` 均可使用，支持 Docker/Kubernetes 只读 secret 挂载；group/other 可写仍拒绝。允许符号链接但校验解析后的目标（Kubernetes projected secret）。
- **`/admin/v1/runtime/status` 新增 `token_fp`**（token 的 SHA-256 指纹前 8 位，不含 token 本身），引擎启动日志也会打印，便于确认客户端与引擎是否使用同一个 token。
- **升级 wp-motor 1.34.0**。

### Fixed
- token 文件为空或包含多行时直接报错；此前会静默产生一个永远无法匹配的 token。
- 未授权请求日志限流（10 秒窗口），避免扫描器刷日志。

## [0.32.4-alpha] - 2026-10-22

### Fixed
- **迁移时不再改写 admin API token 路径**：`wproj conf update` 把遗留 `[admin_api]` 搬到 `wparse.local.toml` 时保留原 `token_file`（例如项目内 `runtime/admin_api.token`），避免 daemon 启动后读取另一个 token 并返回 401。

## [0.32.3-alpha] - 2026-10-22

### Fixed
- 修复 daemon 集成测试中端口预留可能复用的竞态，确保 CI 里 admin API 与 TCP source 使用不同端口。

## [0.32.2-alpha] - 2026-10-22

### Fixed
- **默认示例 WPL 规则不可解析**：`docker/default_setting/models/wpl/parse.wpl` 使用了当前解析器不支持的旧语法，默认工程启动后没有任何 WPL 规则；已更新为可解析的 nginx 示例。daemon 集成测试的启动等待放宽到 60s，避免慢速 CI 误判。

## [0.32.1-alpha] - 2026-10-22

### Fixed
- **远程同步后模型文件可能缺失**：远端缓存 checkout 改为 hard reset，确保被清理过的 `models/` 等目录在同步前完整恢复，避免个别环境下 daemon 启动时报“未找到任何 WPL 规则”。

## [0.32.0-alpha] - 2026-10-22

### Changed
- **管理面配置移出远程托管目录**：`[admin_api]` 从 `conf/wparse.toml` 拆到项目根目录 `wparse.local.toml`。`project_remote` 的 infra 同步会整体替换 `conf/`，旧写法会在每次远程发布后把 `admin_api.bind` 重置为远端模板值，导致远程控制端口失效。
  - 优先级：`wparse.local.toml` > `conf/wparse.toml` 中遗留的 `[admin_api]`。
  - `wproj init` 自动创建本地覆盖文件；`wproj conf update` 在同步前自动迁移遗留配置。
  - 旧工程未迁移时仍可读取 `conf/wparse.toml`，启动时输出告警。

### Dependencies
- 升级 `wp-motor` `v1.32.0` → `v1.33.0`（`wproj init` 不再把 `[admin_api]` 写进 `conf/wparse.toml`）。

## [0.31.1] - 2026-10-21

### Fixed
- **解析失败的事件被归因到兜底规则**：`wp-motor` 的 `parse_event` 在 `Err` 分支上每条 `DataError` 都无条件覆盖候选，因此一次全部失败的事件几乎总是被归到排在最后的 `#[late]` 兜底规则上，`missed_packets` 里报告的 `depth` 也退化成 `1`。现在按「已消费比例」归因到真正走得最远的那条规则，同进度时保留命中率更高的先到候选。
  - 同一次改动也删掉了死代码 `MultiParser::hit_count_sort()` / `WplPipeline.access_cnt`。

### Dependencies
- 升级 `wp-motor` `v1.31.0` → `v1.32.0`（比例化 Miss 归因）。
- `wp-lang` `0.8.1` → `0.8.2`（新增 `WplEvaluator::proc_ref_progress` / `ParseProgress`）。

## [0.31.0] - 2026-10-20

### Added
- **`#[late]` 兜底规则分层排序**（随 `wp-motor v1.31.0`）：标了 `#[late]` 的规则仍参与 `parse_event` 自动匹配，但**固定排在普通规则之后**；命中率排序改为分层键 `(late, hit_cnt desc, index asc)` —— 层内仍按命中次数降序，但**不能跨层提升**。
  - 解决兜底规则（如 `(chars\0)`）因命中面广、命中数涨高后被提升到具体规则之前、抢走事件的问题。
  - 装配期（`MultiParser::new`）先做一次固定分层，避免首个 `OPTIMIZE_TIMES` 窗口前受文件声明顺序影响。
  - 与 `#[no_match]` 同时声明时 `no_match` 优先（规则完全不自动匹配）。
- **实例级全局 tag**：启动时读取 `WP_GLOBAL_TAGS`（`owner_id=team-a,env=prod` 或 JSON），盖到所有解析成功的数据上。
  - 新增 `<work_root>/.env` / `<work_root>/.warp_parse/.env` 加载（进程环境变量优先，`.env` 不覆盖）。
  - 新增 `warp_parse::load_instance_env()`；`wparse` 在构建 `WpApp` 前加载。

### Changed
- **依赖 `wp-motor v1.31.0`**：`wp-engine` / `wp-config` / `wp-cli-core` / `wp-proj` 同步到 `v1.31.0`。`#[late]` 与 `WP_GLOBAL_TAGS` 的实现都在 `wp-engine` 内，`warp-parse` 代码本身不直接引用新 API；要生效需使用该版本（或将上述 git tag 一并升级）。

## [0.30.0] - 2026-10-17

### Added
- **OML 内联脱敏**：随 `wp-motor` 1.30.0 引入 `mask` / `hash` / `tokenize` / `remove`，直接写在赋值 pipe 中，所有 sink 只会看到脱敏后的数据。
  - `mask(kind)` 支持 IP、手机号、身份证、邮箱、域名、姓名、地址、公司、密钥消息与文本；`mask(ip)` 映射到保留地址池，输出仍是合法 IP，可继续 join / group by。
  - `hash([algo])` 提供同值同结果的 HMAC（SHA-256/SHA-1/MD5）；`tokenize()` 生成可逆令牌（还原由独立工具完成）；`remove` 直接删除字段。
  - 支持 `object { ... }` 内部与批量目标（如 `*_mail : chars = read() | mask(mail)`），`fmt` / `match` / `select` 等表达式也可直接接 pipe。
- **零配置脱敏密钥**：`mask` / `hash` 使用 `pii_salt`，`tokenize` 使用 `pii_key`，从 `.warp_parse/sec_key.toml` 的 `sec_pii_salt` / `sec_pii_key` 加载；OML 与 `wparse.toml` 均无需密钥配置。

### Changed
- **依赖升级**：`wp-motor` v1.29.0 → v1.30.0（`wp-engine` / `wp-config` / `wp-oml` / `wp-cli-core` / `wp-proj` 同步）。
- 脱敏失败一律 fail-closed：密钥缺失或为空、类型不匹配时不输出原值；`mask(ip)` 无密钥时只有显式 `allow_unsalted:true` 才允许开发期降级。

## [0.29.1] - 2026-10-14

### Fixed
- **修复 TCP 源队头阻塞（wp-labs/warp-parse#367）**：随 `wp-core-connectors` 0.10.0 → 0.10.2，某条连接上的半帧/长记录不再阻塞其它连接的读取；同时带上错误隔离、长度前缀上限/溢出、EOF 尾行、acceptor 解耦等健壮性修复。

### Changed
- **依赖更新**：刷新 `Cargo.lock`（含 `wp-core-connectors` 0.10.2 及若干传递依赖版本更新）。

## [0.29.0] - 2026-10-12

### Changed
- **依赖升级**：`wp-motor` v1.28.0 → v1.29.0、`wp-connectors` v0.22.0 → v0.23.0；二者又随 `wp-specs` 0.12 → 0.13（`ParamMap` 升级到 `wp-connector-api` 0.14），依赖图中不再同时存在两份 `wp-connector-api`（0.13 + 0.14）。

## [0.28.0] - 2026-10-12

### Changed
- **升级到 orion-error 0.9 世代**：`orion-error` 0.8 → 0.9、`orion_conf` 0.7 → 0.8、`orion-sec` 0.6 → 0.7、`orion-variate` 0.13 → 0.14。
- **依赖升级**：`wp-motor` v1.27.0 → v1.28.0、`wp-connectors` v0.21.0 → v0.22.0、`wp-connector-api` 0.13 → 0.14、`wp-parse-api` 0.12 → 0.13、`wp-conf-base` 0.6 → 0.7、`wp-log` 0.4 → 0.5、`wp-error` 0.12 → 0.13、`wp-knowledge` 0.18 → 0.19、`wp-self-update` 0.3 → 0.4。

### Fixed
- **`wproj check` 的 OML 校验升级为整文件严格解析**：随 `wp-motor` 1.28.0，OML 组件对每个 `.oml` 文件走官方解析器全量解析并做完整消费检查 —— 跨行语法错误、缺失 `name`、以及多余的尾随内容（如误入文件末尾的 `def label_for(...)`）都会被报出并定位到 `文件:行:列`；「模型重名」「未声明 rule」由硬失败降级为告警，与真正的语法错误区分（wp-labs/warp-parse#366）。

## [0.27.1] - 2026-09-26

### Added
- **TCP 连接器 TLS 与压缩/加密**：随 `wp-motor` 1.27.0（`wp-core-connectors` 0.8 → 0.9），`tcp_src`/`tcp_sink` 支持 TLS；`tcp`/`file` 连接器支持 `compression`（gzip/zstd）+ `encryption`（aes-256-gcm/sm4-gcm，含国密 SM4-GCM）。

## [0.27.0] - 2026-09-22

### ⚠️ BREAKING CHANGES
- **同步 `wp-motor` 1.27.0**：整数类型改名 `Value::Digit` → `Value::Int`、`DataType::Digit` → `DataType::Int`（serde wire 标签 `"digit"` → `"int"`、`Field::from_digit` → `from_int`）；OML/条件表达式的类型渲染随之改为 `int`（如 `cpu_free : digit` → `cpu_free : int`）。解析仍接受 `digit`，WPL 规则仍以 `digit` 呈现。

### Added
- **`wparse daemon` 单实例保护**：启动对 `<work-root>/.run/.lock` 取 `flock`，同一 work-root 的第二实例拒绝启动（wp-labs/warp-parse#365）。

### Changed
- **依赖升级**：`wp-motor` v1.26.0 → v1.27.0、`wp-knowledge` 0.16 → 0.18、`wp-error` 0.11 → 0.12、`wp-connectors` v0.20 → v0.21、`wp-connector-api` 0.12 → 0.13、`git2` 0.20 → 0.21、`rand` 0.9 → 0.10、`serial_test` 3.2 → 4.0。

## [0.26.0] - 2026-09-08（稳定版，收敛自 0.25.4–0.25.22 系列）

### Added
- **OML 内网富化与 IP 脱敏**：`intranet_ip`（内/外）、`access_direct`（访问方向 L2L/L2W/W2L/W2W）、`intranet_replace`（内网 IP 脱敏：同族占位或显式替换值）、`on_fail` 兑底；内网网段由 `knowdb.toml [intranet_nets]` 知识化配置。
- **OML 时间与数据组织**：`Time::to_ts/from_ts` 六函数（秒/毫秒/微秒，`zone` 可选）；嵌套 `object {}`、`array {}` 聚合、`static` 块支持嵌套对象/数组。
- **OML SQL 增强**：`select` 支持 `order by`/`limit`；`from <provider>.<schema>.<table>` 多 SQL provider（PostgreSQL/MySQL）路由；PostgreSQL `postgres_session` 连接级初始化以稳定执行计划。
- **Parser**：`wp_event_md5` 事件指纹字段（可配/可关）；`copy_event_parse` 产出独立旁路 record 并按目标 rule 路由到自己的 sink；`#[no_match]` 规则声明。
- **Sink**：JSON/CSV 输出默认携带 `wp_stream_tag`/`wp_event_id` 运行时元字段，支持组级 `wp_meta_disable`。
- **CLI 与工程**：`wproj` 统一更名 `wpadm`（保留兼容）；`wpgen` 支持 `[models].wpl` 目录配置、connector 参数类型校验。

### Changed
- **吞吐与解析提速**：Kafka Source 批量接收（wp-connectors v0.20）；TCP 源读取吞吐修复（v0.8.3 → v0.8.4，约 19.4 万/s → 50.6 万/s）；不限速（rate=0）自动限速收敛提速（parse_to_blackhole 短压 30 万行 ~5.5s → ~1.5s）。
- **错误提示结构化**：OML/配置加载失败定位到具体文件与解析位置。
- **依赖升级**：`wp-motor` v1.22.6 → v1.25.14、`wp-knowledge` v0.13 → v0.16.3、`wp-connectors` v0.14 → v0.20、`wp-core-connectors` v0.3.3 → v0.8.4、`wp-lang`/`wp-primitives` 同步升级。

### Fixed
- 字符串→IP 自动转换支持 IPv6，空/非法输入不再以字符串透传（wp-labs/warp-parse#358）；字段缺失/空值等正常缺失不再刷告警与解析诊断（wp-labs/warp-parse#360）。
- `ip_to_biguint` 结果作 SQL 参数时本地缓存可命中，相同查询不再重复访问数据库（wp-labs/warp-parse#359）。
- OML 嵌套 `object` / `read` / `take` 非法参数不再静默丢弃（加载即报错）；`time_timestamp` 数字 `0` 正确解析为 Unix epoch；WPL `ip` 支持 IPv4-mapped IPv6 解析。

> 演进过程见 0.25.4–0.25.22 各版本 tag。

## [0.24.11] - 2026-06-25

### Changed
- **Dependencies**: 升级 `wp-motor` 从 `v1.22.8` 到 `v1.22.9`，同步 release tag / origin head 解析修正，以及相关基础设施更新。
- **Dependencies**: 升级 `wp-lang` 从 `0.3.3` 到 `0.3.5`，同步 `kvarr_raw`、`kvarr` 重复 key 惰性处理、parser-only 性能基准和 WPL 文档更新。
- **Lockfile**: 刷新 `Cargo.lock` 中的传递依赖版本。

### Fixed
- **Project Remote**: 修复 `project_remote` 在解析本地 tag、origin URL 和 remote HEAD 目标时的健壮性，避免部分 Git 状态下误判或报错。

## [0.24.10] - 2026-06-25

### Changed
- **Dependencies**: 升级 `wp-motor` 从 `v1.22.8` 到 `v1.22.9`，同步 sink 批处理成功路径 record id 开销基准和错误日志路径的轻量化调整。
- **Dependencies**: 升级 `wp-lang` 从 `0.3.3` 到 `0.3.4`，同步 parser-only 性能基准集合、空 pipe 快路径和 quoted `chars` 解析热路径优化。

## [0.24.9] - 2026-06-23

### Added
- **Source Rate Limit**: 同步上游 `wp-motor v1.22.7`，新增 source 侧全局输入限速；`performance.rate_limit_rps = 0` 表示自动限速，`> 0` 表示所有 source 共享固定 EPS 上限。
- **Memory Profiles**: 新增统一内存 profile 支持，可通过 `WP_MEMORY_PROFILE=standard|low|throughput` 控制运行时队列、水位、批大小和网络/文件缓冲等内存相关参数。

### Changed
- **Dependencies**: 升级 `wp-motor` 从 `v1.22.6` 到 `v1.22.7`。
  - 默认 `performance.rate_limit_rps` 从固定值改为 `0` 自动限速。
  - 自动限速根据 picker pending 水位、parser 背压和 RSS 增长保护动态调整输入速率。
  - source 限速等待前移到进入 pending 之前，减少限速场景下 pending/RSS 先膨胀。
  - benchmark `wparse.toml` 使用 `${RATE_LIMIT_RPS:0}`，benchmark 脚本默认用输入速率同步设置 wparse 限速。
- **DebugView**: Debug 输出改为有界队列，队列满时记录丢弃计数并抽样告警，避免无界队列造成 RSS 增长。

## [0.24.8] - 2026-06-19

### Changed
- **Dependencies**: 升级 `wp-motor` 从 `v1.22.4` 到 `v1.22.6`
  - Generator 发送改为动态批量（`BatchSizePolicy`），TCP sink 下 `wpgen` CPU ~300% → ~15%
  - 修复 `wp-core-connectors` crate 名称引用错误（连字符 → 下划线）

## [0.24.7] - 2026-05-26

### Changed
- **Dependencies**: 升级 `wp-lang` 从 `0.3.2` 到 `0.3.3`。

## [0.24.6] - 2026-05-26

### Changed
- **Dependencies**: 升级 `wp-connectors` 从 `v0.14.0` 到 `v0.14.2`（v0.14.1 新增达梦数据库 Source/Sink 支持；v0.14.2 将 dmdb feature 移入 wp-connectors-exp 分类）；升级 `wp-lang` 从 `0.3.1` 到 `0.3.2`（修复 kvarr 中 `<[,]>` 在日志数据不存在或为空时未报错的问题）。

### Fixed
- **Event ID**: 修复 `wp_event_id` 相关问题，确保事件 ID 生成逻辑正确。

## [0.24.5] - 2026-05-22

### Changed
- **Dependencies**: 升级 `wp-model-core` 从 `0.8.7` 到 `0.8.9`；升级 `tokio` 从 `1.52.2` 到 `1.52.3`、`openssl` 从 `0.10.79` 到 `0.10.80`、`serde_json` 从 `1.0.149` 到 `1.0.150`、`aws-lc-rs` 从 `1.16.3` 到 `1.17.0`、`os_info` 从 `3.14.0` 到 `3.15.0` 等多项传递依赖。

### Fixed
- **Model Core**: 修复 `wp-model-core` 相关 bug。

## [0.24.4] - 2026-05-19

### Changed
- **Dependencies**: 升级 `wp-motor` 从 `v1.22.3` 到 `v1.22.4`，同步引入 `ip4_to_int` 修复——新增字符串 IPv4 地址解析支持，IPv6 地址改返回 Null 而非静默透传。

## [0.24.3] - 2026-05-18

### Added
- **SQL/Route**: 新增 SQL 查询按表名路由到本地 SQLite 或外部 Provider 的能力——支持配置 `knowdb.toml` 的 `[[tables]]` 和 `[provider.tables]`，解析 SQL 时自动识别 `FROM` 子句中的表名并分发查询。
- **KnowDB/Config**: 新增 `uses_external_provider_only()` 判定，纯外部 provider 配置不再删除本地 authority 文件。

### Changed
- **Dependencies**: 升级 `wp-motor` 从 `v1.22.2` 到 `v1.22.3`，同步引入 SQL 路由、KnowDB 外部 Provider 支持、`sanitize_sql_body` 子查询与别名语法增强等改进。

## [0.24.2] - 2026-05-13

### Added
- **Sinks/Sync**: 同步上游 `wp-motor` 新增的 `SinkTerminal` 批量写入方法（`send_to_sink_batch`、`try_send_to_sink_batch`），降低统计切片过多造成的反压。

### Changed
- **Dependencies**: 升级 `wp-motor` 从 `v1.22.1` 到 `v1.22.2`，同步引入 sink 批量写入能力。

## [0.24.1] - 2026-05-12

### Fixed
- **OML/SQL**: 同步上游修复，当 SQL 参数全部为 Null 时跳过实际查询，避免对空参数的不必要远程调用。
- **OML/Extract**: 同步上游修复，`SingleEvalExp` 提取字段时跳过 `Value::Null`，不再为 Null 值创建目标字段。
- **OML/SQL**: 同步上游修复知识库相关查询 bug。

### Changed
- **Knowledge Base**: 同步上游知识库查询优化。
- **Dependencies**: 升级 `wp-motor` 从 `v1.22.0` 到 `v1.22.1`，同步引入 OML/SQL 查询优化与知识库查询改进。

## [0.24.0] - 2026-05-08

### Added
- **HTTP Source**: 新增 HTTP Source 连接器支持（`http` feature），支持通过 HTTP 接口接收外部数据推送。
- **Postgres Source**: 新增 Postgres Source 特性继承支持；低版本兼容性适配。
- **Project Remote**: 新增双仓库模式支持（`[project_remote.models]` + `[project_remote.infra]`），支持 `models/` 和 `infra/` 从两个独立的 Git 仓库分别同步，通过 `--group models|infra` 逐组更新。

### Changed
- **Dependencies**: 升级 `wp-motor` 从 `v1.20.0` 到 `v1.22.0`，同步引入：`take()` 字段优先级修复、SQL `IN (...)` 参数绑定修复、错误处理链路优化、`WarpProject::load()` 语义恢复、`stable_code` 双语错误提示、`orion-error 0.8` 升级适配等全部上游改进。
- **Dependencies**: 升级 `wp-connectors` 从 `v0.12.1` 到 `v0.14.0`（新增 HTTP Source、PostgreSQL Source 连接器）；
- **Error Handling**: 全面升级错误治理体系 — `orion-error` 升级到 `0.8` 主线，，错误信息附带路径上下文，CLI 报错更完整可读。
- **Admin API**: 支持监听地址配置修改；reload 接口新增 `group` 参数支持双仓库模式分组更新；status 接口在双仓库模式下返回分组版本信息。

### Fixed
- **OML/Take**: 同步上游修复，`take(...)` 可正确消费目标记录中已生成的字段，并修正同名字段取值顺序。
- **OML/SQL Parser**: 同步上游修复，增强 `group_concat(...)`、`string_agg(...)`、`IN (...)`、`take(field)` 与 `__temp_var` 等 SQL 参数解析场景。
- 修复部分场景下运行时稳定性问题。

## [0.22.5] - 2026-04-26

### Changed
- **wproj/Check**: 大幅提升检查深度与广度 — 新增 `wpgen` 配置检查（`output.connect` 引用、`rule_root` 路径、`sample_pattern`、`logging.file_path`）；语义词典新增空词/重复词/空类别校验；source/sink 目录缺失、source 文件/GLOB 不匹配等降级为 warning，避免临时状态误判。
- **Dependencies**: 升级 `wp-motor` 到 `v1.20.7`，同步 `wproj check` 增强与校验链路改进。
- **CLI Errors**: 统一 `-q/--quiet` 参数处理，改善安静模式下的诊断输出。

### Fixed
- **wproj/Check JSON**: 修复 stdout 污染 `--json` 输出的问题。
- **wpgen/Schema**: 拒绝缺失 `output.connect` 的配置，移除示例中废弃的 `mode`/`duration_secs` 字段。
- **wpgen/Level**: 支持 compound 日志级别格式（如 `info,ctrl=info`）。
- **Config/Panic**: 修复引擎配置含未知 TOML 字段时 `with_source` 导致的 panic。
- **Project Loading**: wpgen.toml 缺失时不再阻塞项目加载。
- **OML/WPL Lint**: 额外语义检查改为非阻断 lint。


## [0.22.4] - 2026-04-26

### Changed
- **Dependencies**: 升级 `wp-motor` 到 `v1.20.6`，同步上游改进；升级 `wp-knowledge` 从 `0.11.4` 到 `0.11.6`，改善 MySQL/PostgreSQL 知识库连接稳定性与字段类型兼容性（新增 `BYTEA`、`ENUM`、`UUID` 等类型支持），并支持连接池细粒度配置。

### Fixed
- **Security**: 修复 `load_sec_dict` 中因错误类型不匹配导致的潜在 panic（`with_std_source` → `with_struct_source`），确保 sec_key 加载失败时能正常报告错误而非崩溃。

### Added
- **Audit**: 新增 `.cargo/audit.toml`，忽略 RUSTSEC-2023-0071（rsa crate Marvin Attack 时序侧信道 — 仅影响 loopback TLS 且默认关闭，实际风险低，计划 2026-07-25 再评估）。


## [0.22.3] - 2026-04-22

### Changed
- **Dependencies**: 升级 `wp-motor` 到 `v1.20.5`，同步错误诊断、配置加载与工程管理链路的稳定性改进。
- **Admin API**: Admin API 与 client profile 加载改用核心引擎统一配置 loader，复用环境变量与路径解析语义。
- **CLI Errors**: `wparse`、`wpgen`、`wproj`、`wprescue` 的报错信息进一步统一；现在会更稳定地保留失败原因、相关路径和上游错误线索，便于直接定位问题。
- **工程管理体验**: 配置加载、project remote 与工程管理相关命令的错误输出进一步收敛；同类问题会更一致地显示，不再出现一部分路径过于简略、一部分路径信息过多的情况。

### Fixed
- **wproj/Engine**: 改进 engine status/reload 的请求、token、header 与响应解析错误，失败时会提供更完整的原因信息，减少”只知道失败但不知道为什么”的情况。
- **wproj/Conf Update**: 修复配置更新后校验失败时错误信息过短的问题；现在会更完整地显示校验链路，便于定位具体失败点。
- **Project Remote**: 修复部分远端工程同步与状态持久化失败场景下的错误链问题，避免出现报错被错误重包、信息不完整或行为不稳定的情况。

## [0.22.2] - 2026-04-16

### Added
- **CLI Usage Docs**: 新增按工具拆分的 CLI 使用文档，包括 `wparse`、`wpgen`、`wproj`、`wprescue` 的专题页。
- **Operations/Overview Docs**: 新增 `overview/` 与 `operations/` 目录结构，补充产品概览、运行时管理面使用说明、远端工程同步与热更新 SOP 的重组版本。

### Changed
- **Dependencies**: 升级 `wp-motor` 远端依赖从 `v1.20.0` 到 `v1.20.1`，并同步升级 `wp-connectors` 从 `v0.12.1` 到 `v0.12.2`。
- **Docs Layout**: 使用类文档重组为 `overview/`、`cli/`、`operations/` 分层目录；

## [0.22.0] - 2026-03-31

### Added
- **Self Update**: 新增可执行安装的 `wproj self update`，并引入独立 `warp-self-update` crate，统一承载 manifest 解析、版本比较、资产下载、安装与回滚逻辑；支持 `sha256` 校验、健康检查失败回滚，以及 `--yes`、`--dry-run`、`--force` 等控制参数。
- **Admin API Dev Docs**: 新增独立的 Admin API 接口开发文档，单独说明 `GET /admin/v1/runtime/status`、`POST /admin/v1/reloads/model` 的请求/响应、状态码、并发冲突和 `update/version` 语义。
- **Runtime Status**: Admin API `GET /admin/v1/runtime/status` 与 `wproj engine status` 新增 `project_version`，用于返回当前工作目录实际使用的工程配置版本。
- **Reload Runtime**: 随核心引擎升级引入 reload 结果结构化输出、事件驱动 drain，以及 `reload_timeout_ms` 配置能力。
- **OML Async Runtime**: 随核心引擎与知识库运行时升级，将模型加载与知识库查询切到异步执行链路。
- **Knowledge Runtime**: 新增 PostgreSQL / MySQL 知识库支持，并补充统一缓存与 telemetry。
- **wproj/model**: `wproj model route` 新增异步 OML 模型收集链路。
- **Observability**: 随核心引擎升级补充 metrics 固定标签与统一 tag 命名支持，便于监控系统稳定消费运行时指标。
- **wp-connectors**: 同步引入 VictoriaMetrics 相关能力更新。
- **wp-lang**: 同步引入 `json_like` 支持。

### Changed
- **Remote Project Sync**: 远端初始化入口统一为 `wproj init --repo <REPO> [--version <VERSION>]`，移除 `--remote`；未显式指定 `--version` 时优先选择最新 release tag，若远端没有 release tag，则自动回退到默认分支 `HEAD`，并统一 `wproj conf update` 与 admin reload/update 链路的参数、帮助与状态输出行为。
- **CLI/Paths**: `wproj data validate`、sink/source 统计与 rescue 输出统一为更短的相对路径风格，提升终端可读性。

### Fixed
- **Project Init Admin Token Path**: 修复 `wproj init` 在骨架已包含 `[admin_api]` 时未规范 token 路径的问题；当前会统一收敛为项目内 `runtime/admin_api.token`，避免遗留 `${HOME}/.warp_parse/admin_api.token`。


## [0.20.2] - 2026-03-20

### Fixed
- ** update orion-sec , sec_key 保持一致

## [0.20.1] - 2026-03-16

### Fixed
- **Event ID**: 同步上游 `wp-motor` 修复，统一 `wp_event_id` 生成逻辑，并避免运行时重启后回退到进程内种子导致的重复 ID。
- **Kafka Source**: 同步上游 `wp-connectors` 修复，Kafka source 改用共享 `wp_event_id` 生成器，不再使用进程内自增序列。

## [0.20.0] - 2026-03-12

### Added
- 新增支持 HTTP Sink。
- 新增支持 ES Sink。
- 新增支持 Postgres Sink。
- 新增支持 Doris Sink。
- 新增支持 ClickHouse Sink。

### Changed
- **wp-motor**: 核心引擎依赖从 `v1.17.8` 升级到 `v1.18.0`。
- **Dependencies**: 核心依赖升级到新主线（`orion-error 0.6`、`wp-connector-api 0.8`、`wp-error 0.8`、`wp-log 0.2` 等）。
- **Dependencies**: 同步引入 `rand 0.10`、`toml 1.0` 等依赖更新。
- **Runtime Connectors**: 为规避升级期间 API 不兼容，社区外部连接器注册调整为暂时跳过并输出告警日志。


## [0.18.4] - 2026-03-04

### Changed
- 升级 `wp-motor` 核心引擎从 v1.17.5 到 v1.17.6
- `wp-motor` v1.17.6 主要增强观测与统计链路（背压指标、聚合语义修正、热路径优化），并修复 parser 退出与 recovery failover 稳定性问题

## [0.18.3] - 2026-02-27

### Changed
- 升级 `wp-motor` 核心引擎从 v1.17.4-alpha 到 v1.17.5-alpha
- 升级 `wp-connectors` 从 v0.7.7-beta 到 v0.7.8-beta
- 更新项目依赖到最新版本

## [0.18.2] - 2026-02-20

### Changed
- 升级 `wp-motor` 核心引擎从 v1.17.0-alpha 到 v1.17.4-alpha，主要变化包括：
  - **Sinks/Buffer**：新增 sink 级别批量缓冲区，支持可配置 `batch_size` 参数；小包进入待发缓冲区定期刷新，大包自动旁路直接发送（零拷贝）
  - **Sinks/Config**：新增 `batch_timeout_ms` 配置项（默认 300ms），控制缓冲区定期刷新间隔
  - **Sinks/File**：移除 `BufWriter` 和 `proc_cnt` 定期刷新，改为直接写入 `tokio::fs::File`；上游批量组装使用户空间缓冲冗余
- 升级 `wp-connectors` 从 v0.7.6-beta 到 v0.7.7-beta，主要变化包括：
  - **Doris**：使用新协议
  - 更新 `reqwest` 从 0.12 到 0.13
  - 更新 `env_logger` 从 0.10 到 0.11

## [0.18.1] - 2026-02-13

### Changed
- 升级 `wp-motor` 核心引擎从 v1.17.0-alpha 到 v1.17.2-alpha，主要变化包括：
  - **wp-lang**：`kv`/`kvarr` key 解析支持括号类字符 `()`、`<>`、`[]`、`{}`

## [0.18.0] - 2026-02-12

### Changed
- 升级 `wp-motor` 核心引擎从 v1.15.5 到 v1.17.0-alpha，主要变化包括：
  - **OML Match 增强**：新增 OR 条件语法 `cond1 | cond2 | ...`，支持单源和多源匹配，兼容值匹配和函数匹配
  - **OML Match 增强**：多源匹配不再限制源字段数量（之前限制为 2/3/4 个）
  - **OML NLP**：新增 `extract_main_word` 和 `extract_subject_object` 管道函数，用于中文文本分析
  - **OML NLP**：新增可配置 NLP 词典系统，支持通过 `NLP_DICT_CONFIG` 环境变量自定义词典
  - **WPL 新功能**：新增分隔符模式语法 `{…}`，支持通配符（`*`、`?`）、空白匹配器（`\s`、`\h`、`\S`、`\H`）和保留组 `(…)`，用于在单个声明中表达复杂分隔符逻辑
  - **Bug 修复**：修复 kvarr 模式分隔符解析问题

## [0.17.1] - 2026-02-09

### Changed
- 升级 `wp-motor` 核心引擎从 v1.15.1 到 v1.15.5，主要变化包括：
  - **文档**：新增完整的英文 WPL 语法参考文档
  - **性能优化**：OML 批处理性能提升 12-17%
  - **性能优化**：OML 零拷贝优化，多阶段管道性能提升最高 32%
- 更新项目依赖到最新版本

## [0.17.0] - 2026-02-07

### Changed
- 升级 `wp-motor` 核心引擎到 v1.15.1 版本，主要变化包括：
  - **WPL 新增功能**：新增 `not()` 包装函数用于反转管道函数结果
  - **WPL 新增功能**：新增 `not()` 组包装器用于字段解析中的否定断言
  - **OML 新增功能**：引入 `static { ... }` 语法用于模型范围的常量和模板缓存，提升性能
  - **OML 配置**：新增 `enable` 配置选项，支持禁用 OML 模型
  - **Sinks/File**：新增 `sync` 参数控制磁盘刷新策略（高性能模式 vs 数据安全模式）
  - **Sinks/File**：移除 proto binary 格式支持，当前支持格式：json、csv、kv、show、raw、proto-text
  - **Bug 修复**：修复 `sync` 参数未强制数据写入磁盘的问题
  - **Bug 修复**：修复 WPL 管道函数 `f_chars_not_has` 和 `chars_not_has` 的类型检查 bug
- 更新项目依赖到最新版本

## [0.16.1] - 2026-02-05

### Changed
- 升级 `wp-motor` 核心引擎到 v1.14.1-alpha 版本，主要变化包括：
  - **WPL 管道处理器**：新增 `strip/bom` 处理器用于移除 BOM（字节顺序标记）
    - 支持 UTF-8、UTF-16 LE/BE、UTF-32 LE/BE BOM 检测和移除
    - O(1) 快速检测（仅检查前 2-4 字节）
    - 保留输入容器类型（String → String, Bytes → Bytes, ArcBytes → ArcBytes）

## [0.16.0] - 2026-02-04

### Changed
- 升级 `wp-motor` 核心引擎到 v1.14.0 版本，主要变化包括：
  - **WPL 函数增强**：新增 `starts_with` 管道函数，用于高效字符串前缀匹配
  - **OML 管道函数**：新增 `starts_with` 函数用于前缀匹配
  - **OML 管道函数**：新增 `map_to` 函数用于类型感知的条件值分配（支持 string、integer、float、boolean）
  - **OML 匹配表达式**：支持基于函数的模式匹配（`match read(field) { starts_with('prefix') => result }`）
    - 字符串匹配函数：`starts_with`、`ends_with`、`contains`、`regex_match`、`is_empty`、`iequals`
    - 数值比较函数：`gt`、`lt`、`eq`、`in_range`
  - **OML 解析器**：支持 `chars()` 等值构造器中的引号字符串（单引号和双引号）
  - **OML 转换器**：新增临时字段自动过滤功能（以 `__` 开头的字段自动转换为 ignore 类型）
  - **OML 语法简化**：管道表达式中 `pipe` 关键字现在为可选（`take(field) | func` 和 `pipe take(field) | func` 都支持）
  - **修复问题**：修复 OML 匹配表达式中 `in_range` 函数解析失败的问题
  - **修复问题**：修复 `map_to` 解析器中大整数精度丢失的问题
  - **修复问题**：修复 OML 显示输出的往返解析兼容性问题

## [0.15.8] - 2026-02-03

### Changed
- 升级 `wp-motor` 核心引擎到 v1.13.3 版本，主要变化包括：
  - **WPL 解析器**：支持 `\t`（制表符）和 `\S`（非空白字符）分隔符
  - **WPL 解析器**：支持带引号的特殊字符字段名（如 `"field.name"`、`"field-name"`）
  - **WPL 函数增强**：新增 `regex_match` 正则匹配函数
  - **WPL 函数增强**：新增 `digit_range` 数字范围验证函数
  - **WPL 函数增强**：新增 `chars_replace` 字符级字符串替换函数
  - **日志优化**：高频日志路径使用 `log_enabled!` 守卫，消除日志级别过滤时的循环开销
  - **修复问题**：修复 WPL 模式解析器的编译错误
  - **修复问题**：修复数据救援功能的数据丢失问题
  - **修复问题**：移除 Miss Sink 原始数据显示中的 base64 编码，直接显示实际内容
- 更新所有依赖到最新版本。
- **许可证变更**：项目许可证从 Elastic License 2.0 变更为 Apache 2.0。
- **文档改进**：新增 CONTRIBUTING.md 贡献指南，更新 README.md 说明文档。

## [0.15.7] - 2026-01-30

### Changed
- 升级 `wp-motor` 核心引擎到 v1.13.1 版本，主要变化包括：
  - **WPL 解析器增强**：支持 `\t`（制表符）和 `\S`（非空白字符）分隔符
  - **WPL 解析器增强**：支持带引号的特殊字符字段名（如 `"field.name"`、`"field-name"`）
  - **新增函数**：`chars_replace` 字符级字符串替换函数
  - **日志优化**：高频日志路径使用 `log_enabled!` 守卫，消除日志级别过滤时的循环开销
  - **移除功能**：Syslog UDP Source 移除 `SO_REUSEPORT` 多实例支持（安全风险及跨平台不一致）
- 升级 `wp-connectors` 到 v0.7.5-beta 版本。

## [0.15.5] - 2026-01-28

### Changed
- 升级 `wp-motor` 核心引擎到 v1.11.0-alpha 版本。
- 更新项目依赖到最新版本。

## [0.15.4] - 2026-01-27

### Changed
- 更新所有依赖到最新版本，提升稳定性和性能。

## [0.15.3] - 2026-01-23

### Fixed
- 修复 wp-motor 相关问题，提升运行时稳定性。

## [0.15.2] - 2026-01-22

### Changed
- 从 `wp-engine` 迁移到 `wp-motor` v1.10.2-beta 版本：
  - wp-engine 项目已更名为 wp-motor，所有依赖已更新指向新仓库
  - 升级到 v1.10.2-beta 版本，包含最新的运行时特性与性能优化

## [0.15.1] - 2026-01-18

### Added
- 集成 shadow-rs 构建时信息支持 (#100)：
  - 添加 shadow-rs 作为构建依赖，在编译时生成元数据
  - 版本命令现在显示 Git commit、构建时间和 Rust 编译器版本
  - 提升部署二进制文件的可追溯性，便于问题排查

### Changed
- 更新项目依赖到最新版本。

## [0.15.0] - 2025-01-17

### Changed
- 升级 `wp-engine` 核心引擎到 v1.10.0-alpha 版本，主要变化包括：
  - **新增 KvArr 解析器**：支持键值对数组格式解析（`key=value` 或 `key:value`），支持灵活的分隔符（逗号、空格或混合），自动类型推断，重复键自动数组索引
  - **修复 meta 字段问题**：修复了 meta fields 在 sub-parser 上下文中被忽略的问题
  - **API 改进**：修复了 wp-cli-core 中 `validate_groups` 函数导出问题，现在从 `wp_cli_core::utils::validate` 模块导出
- 升级 `wp-model-core` 到 0.7.1 版本。

## [0.14.0] - 2025-01-16

### Added
- 新增 `wproj rescue stat` 命令，用于统计 rescue 目录中的数据：
  - 支持按 sink 分组统计文件数量、记录条数和文件大小
  - 支持 `--detail` 显示文件详情
  - 支持 `--json` 和 `--csv` 多种输出格式
- 新增 Doris 连接器支持，现在可以直接将数据写入 Apache Doris 数据库。
- GitHub Release 发布流程新增自动提取 CHANGELOG 功能：
  - 自动从 CHANGELOG.md 和 CHANGELOG.en.md 提取对应版本的更新内容
  - 默认展示英文 changelog，中文内容以折叠区域形式显示
  - 通过 scripts/extract-changelog.sh 脚本实现

### Changed
- 升级 `wp-engine` 核心引擎到 v1.9.0-alpha.2 版本，主要变化包括：
  - **动态速率控制模块**：新增 `SpeedProfile` 支持多种速率模式（恒定、正弦波、阶梯、突发、斜坡、随机游走、复合模式），用于模拟真实流量场景
  - **Rescue 统计模块**：新增 rescue 数据统计功能，支持按 sink 分组统计、多种输出格式（表格、JSON、CSV）
  - **wpgen.toml 配置增强**：支持在配置文件中定义 `speed_profile` 动态速率配置
  - **BlackHoleSink 增强**：新增 `sink_sleep_ms` 参数，支持控制每次 sink 操作的延迟

### Fixed
- 修复 wpgen 配置中 `speed_profile` 动态生成率未生效的问题，现在可以正确从配置文件读取并应用 sinusoidal、stepped、burst 等动态速率模式。
- 修复升级 wp-engine 后 `GenGRA` 缺少 `speed_profile` 字段导致的编译错误。
- 修复 dependabot-branch-filter 工作流中的 YAML 语法错误。
- 修复 adm.gxl 配置文件相关问题。

### Documentation
- 移除过时的技术设计和用户指南文档，清理文档结构。

[0.14.0]: https://github.com/wp-labs/warp-parse/releases/tag/v0.14.0

## [0.13.1] - 2026-01-14

### Changed
- 升级 `wp-engine` 核心引擎到 v1.8.2-beta 版本，获取最新的运行时特性与性能优化。
- 升级 `wp-connectors` 连接器到 v0.7.5-alpha 版本，提升数据源适配稳定性。
- 更新 CI 工作流，新增基于 wp-examples 仓库的集成测试步骤，确保发布质量。
- 清理未使用的模板文件 `_gal/tpl/Cargo.toml` 和工作流配置，简化项目结构。
- 更新 README 中的性能测试相关说明与示例。

[0.13.1]: https://github.com/wp-labs/warp-parse/releases/tag/v0.13.1

## [0.13.0] - 2024-05-09

> :information_source: 本次版本紧随 [wp-engine v1.8.0 changelog](https://github.com/wp-labs/wp-engine/releases/tag/v1.8.0) 调整，CLI 侧变更以适配核心引擎 API 为主，建议同时阅读引擎发布说明以了解 runtime 行为差异。

### Added
- 全新 **Field Pipe** 方案文档《docs/field-pipe-design.md》，阐述字段集合 pipe 与单字段 pipe 拆分后的执行模型，帮助使用者理解 `take/last/@key` 等 selector 与 `base64_decode` 等函数的协作方式。
- `wproj` 数据、统计、验证子命令现在会自动加载安全字典 (`EnvDict`)，无需手动设置即可获取密钥、变量等运行态配置。

### Changed
- `wproj`、`wparse`、`wprescue` 三个 CLI 统一改用 `wp_cli_core::split_quiet_args` 处理 `-q/--quiet`，并在入口注册运行时特性，保证安静模式与插件加载行为一致。
- 全量迁移到 `wp_cli_core` 的 sink/source 统计与校验实现：`stat`/`validate` 输出直接使用核心库排版，路由/OML 展示与引擎保持一致；`wpgen rule` 的直连执行也会把运行时变量下发给引擎层。
- 模板 `_gal/tpl/Cargo.toml` 与主工程 `Cargo.toml` 更新依赖，去除废弃的 `wp-cli-utils`，直接引用 `wp-cli-core` 以获得最新 CLI 能力集合。

### Fixed
- 适配 `wp-engine` v1.8.0 升级后的 API（例如 `WarpProject::init/load`、`load_warp_engine_confs`、`collect_oml_models` 等）需要显式 `EnvDict` 参数的问题，解决多处编译错误并提升运行时的配置一致性。
- 统计/验证命令在非 JSON 模式下与 `wp-cli-core` 类型不匹配导致的显示/解析崩溃，当前统一转换为核心库格式后即可正常输出。

[0.13.0]: https://github.com/wp-labs/warp-parse/releases/tag/v0.13.0
