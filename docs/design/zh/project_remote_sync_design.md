# 远程规则版本更新设计

- 状态: Draft
- 范围: `wproj init --repo`、`wproj conf update`、`wproj engine reload`、HTTP admin API `POST /admin/v1/reloads/model`

## 需求归纳

本设计以以下需求为准：

1. 工程配置中需要提供远程 repo 地址和总开关
2. 使用者不需要也不应该直接使用 Git
3. `wproj` 需要提供统一入口：`wproj conf update`
4. 运行时在收到“更新并重载”指令时，必须先完成 conf update
5. 手工更新和运行时 reload 不能各自维护一套同步逻辑
6. HTTP admin API 需要提供“是否更新”和“版本号”参数
7. `wproj init` 需要支持直接从远端初始化工程

## 设计结论

把“远程工程同步”定义为一个共享的项目能力，而不是某个单独二进制的私有行为。

- 配置放在项目级配置文件中，由 `wproj` 和 `wparse` 共同读取
- `wproj init --repo` 负责首次初始化时提供远端引导参数并触发首次同步
- `wproj conf update` 是显式运维入口
- `wproj engine reload --update` 和 HTTP admin API reload 是运行时入口
- 三类入口复用同一个 conf update 核心流程

用户面对的是“从远程版本源同步规则”这个动作，而不是 `git clone` / `git pull`。

## 非目标

- 不向用户暴露底层仓库操作细节
- 不设计成通用仓库管理工具
- 不解决规则编写问题
- 不替代 `wproj self update`
- 首版不做多 remote、多分支编排
- 当前不提供独立的 runtime restart 设计

## 配置边界

该能力属于项目级可选增强配置。

放置位置：

```text
conf/wparse.toml
```

规则：

- `wparse.toml` 继续作为唯一必需配置
- `[project_remote]` 作为可选配置段存在
- 未配置 `[project_remote]` 时，`wparse` 仍可正常启动
- `wproj conf update`、`wproj engine reload --update`、HTTP admin API reload 都读取同一个配置段

## 配置模型

建议在 `conf/wparse.toml` 中新增可选段：

```toml
[project_remote]
enabled = true
repo = "https://github.com/wp-labs/editor-monitor-conf.git"
init_version = "1.4.2"
```

字段说明：

- `enabled`: 总开关；关闭时不执行远程同步
- `repo`: 远程仓库地址
- `init_version`: 首次初始化版本；仅在本地尚未初始化过远端状态时使用

最小必需字段：

- `enabled`
- `repo`

建议字段：

- `init_version`

tag 约定：

- 若远端发布 tag 为 `v1.4.2`，则动作参数或配置写 `1.4.2`
- 系统内部按语义版本解析并匹配对应 tag

可用于联调和测试的仓库示例：

- `https://github.com/wp-labs/editor-monitor-conf.git`

## 外部接口

### 1. 首次初始化入口

```bash
wproj init --repo <REPO> [--version <VERSION>]
```

语义：

- `wproj init --repo` 先初始化本地工程骨架
- `--repo` / `--version` 只作为首次同步引导参数
- 然后复用 `wproj conf update` 的同步、校验、回滚流程完成首次同步
- 首次同步完成后，以远端仓库中的工程配置为准

版本选择规则：

- 若显式传入 `--version`，则必须解析到唯一 release tag
- 若未传入 `--version`，则先尝试远端最新 release tag
- 若远端没有 release tag，则回退到远端默认分支 `HEAD`

HEAD 回退时的结果语义：

- `resolved_tag = "HEAD@<branch>"`
- `current_version = "<branch>"`

### 2. 显式更新入口

```bash
wproj conf update [--version <VERSION>]
```

语义：

- `wproj conf update` 负责执行已配置远端的版本同步
- 用户不需要判断当前是首次部署还是日常更新
- `wproj conf update` 不隐含自动 reload
- 成功后是否继续执行 reload，由显式运行时动作决定

版本选择规则：

- 若命令显式传入 `--version`，则以该版本为准
- 若首次初始化且未显式传入版本，则优先使用 `init_version`
- 若非首次且未显式传入版本，则优先解析最新 release tag
- 若远端没有 release tag，则回退到远端默认分支 `HEAD`

约束：

- 显式 `--version` 仍然只支持按 tag 版本升级或回退
- 自动回退到 `HEAD` 只发生在未显式指定版本时

### 3. 运行时 reload 入口

CLI：

```bash
wproj engine reload [--update] [--version <VERSION>]
```

HTTP：

```http
POST /admin/v1/reloads/model
```

请求体增加：

- `update: bool`
- `version: string | null`

运行时语义：

- `update = false`：只对当前工作目录执行 `LoadModel`
- `update = true`：先执行 conf update，成功后再执行 reload
- `version` 非空：本次 conf update 使用该版本
- `version` 为空：按默认版本选择规则解析目标版本

默认版本选择规则与 `wproj conf update` 保持一致：

- 首次初始化优先 `init_version`
- 非首次优先最新 release tag
- 无 release tag 时回退远端默认分支 `HEAD`

非法组合：

- `update = false` 且 `version` 非空 -> 直接拒绝
- `project_remote.enabled = false` 且 `update = true` -> 更新失败，reload 不继续

示例：

```json
{
  "update": true,
  "version": "1.4.3",
  "wait": true,
  "timeout_ms": 15000,
  "reason": "rule update and reload"
}
```

## 统一核心流程

`wproj init --repo`、`wproj conf update`、`wproj engine reload --update` 和 HTTP admin API reload 必须复用同一个核心模块。

该核心流程负责：

1. 读取项目同步配置
2. 解析本次动作的目标版本
3. 在独立 remote 目录中完成版本更新
4. 为当前工作目录中的受管目录白名单生成 backup
5. 将 remote 目录中的受管目录白名单复制到当前工作目录
6. 执行更新后检查
7. 成功后返回结构化结果
8. 失败时用 backup 还原受管目录白名单

运行时 reload 仅在 conf update 成功后才继续执行 `LoadModel`。

## 目录模型

本设计不在当前工作目录内直接执行仓库切换，而是采用三目录模型：

- `remote`: 远程工程缓存目录，用于 clone、fetch、版本切换
- `current`: 当前运行工作目录，即 `wparse` 实际读取的目录
- `backup`: 当前工作目录的备份目录，用于失败回滚

约束：

- Git 操作只发生在 `remote` 目录
- `current` 目录始终保持“可运行快照”语义
- 检查或 reload 失败时，必须能从 `backup` 恢复
- 目录切换采用“受管目录白名单”，而不是“整个工作目录全量覆盖”

受管目录白名单：

- `conf/`
- `models/`
- `topology/`
- `connectors/`

不参与切换的运行态目录：

- `data/`
- `logs/`
- `.run/`
- `runtime/`

规则：

- backup 只备份白名单目录
- remote -> current 只复制白名单目录
- restore 只还原白名单目录
- 若目标版本删除了白名单中的某个文件或目录，current 中对应内容也必须删除

## 统一更新流程

无论入口来自 `wproj init --repo`、`wproj conf update` 还是 `reload + update`，统一采用以下流程：

1. 根据配置定位 remote 目录
2. 在 remote 目录执行远端更新并切到目标版本
3. 将 current 中的受管目录白名单备份到 backup
4. 将 remote 中的受管目录白名单复制到 current
5. 执行更新后检查
6. 若调用方要求 reload，则继续执行 `LoadModel`
7. 若检查失败或 reload 失败，则用 backup 还原受管目录白名单

对于 `wproj init --repo`，进入上述流程前还需：

1. 生成本地工程骨架
2. 将 `--repo` / `--version` 作为首次同步引导参数传入核心流程

## 基于发布版本的同步语义

对用户隐藏底层仓库实现之后，系统需要提供稳定的版本同步语义：

- 首次初始化时，同步到 `init_version`、显式指定版本，或自动解析出的默认目标
- 后续更新时，同步到本次动作指定版本，或自动解析出的默认目标
- 默认模式优先选择 release tag；没有 release tag 时回退默认分支 `HEAD`
- 当本地目录状态不满足安全约束时，直接失败

## 为什么要区分 `init_version`、动作版本和当前版本

如果发布是通过版本 tag 管理，那么：

- 初始化关注“第一次应该从哪个版本起步”
- 更新动作关注“这一次要切到哪个版本”
- 运行状态关注“当前实际上跑在哪个版本”

建议分层：

- `init_version`: 固定配置，只用于首次初始化
- `version`: 动作参数，只用于本次 `conf update` 或 `reload --update`
- `current_version`: 状态字段，只记录当前结果

这样既支持首次部署、日常升级、显式回退，也能表达“当前实际上落在默认分支 HEAD”这种非 tag 状态。

## manual update 语义

`wproj conf update` 的语义是：

1. 将 remote 目录同步到本次动作指定的目标版本
2. 备份当前工作目录中的受管目录白名单到 backup
3. 用 remote 中的受管目录白名单覆盖当前工作目录
4. 执行更新后检查
5. 成功则返回更新结果
6. 失败则恢复 backup

`wproj conf update` 不隐含自动 reload。

## reload 语义

`wproj engine reload --update` 或 HTTP admin API `update = true` 的语义是：

1. 执行一次 conf update
2. 若 conf update 失败，则整次请求失败，reload 不继续
3. 若 conf update 成功，则对更新后的 current 目录执行 `LoadModel`
4. 若 reload 失败，则回滚受管目录白名单

如果 `update = false`：

- 只执行当前目录的 `LoadModel`
- 不触发远端同步

## 最小校验门禁

无论由哪个入口触发，同步完成后都需要最小校验门禁。

首版建议：

- 至少执行 WPL 相关校验
- 运行时更新入口在进入 reload 前必须完成这一步

等价目标：

```bash
wproj check --what wpl --fail-fast
```

## 结果模型

建议统一输出结构化结果，至少包含：

```json
{
  "action": "conf_update",
  "trigger": "admin_api",
  "work_root": "/srv/wp/project-a",
  "repo": "ssh://git@github.com/acme/wp-project.git",
  "update": true,
  "requested_version": "1.4.2",
  "init_version": "1.4.2",
  "current_version": "1.4.2",
  "resolved_tag": "v1.4.2",
  "sync_result": "updated",
  "from_revision": "abc1234",
  "to_revision": "def5678",
  "validation_result": "passed",
  "runtime_action": "reload",
  "runtime_result": "success"
}
```

HEAD 回退场景示例：

```json
{
  "requested_version": null,
  "current_version": "main",
  "resolved_tag": "HEAD@main"
}
```

建议的 `trigger`：

- `manual`
- `engine_reload`
- `admin_api`

建议的 `runtime_action`：

- `none`
- `reload`

## 失败语义

必须明确区分三类失败：

1. 同步失败
2. 同步成功但校验失败
3. 同步与校验成功，但 reload 失败

特别要求：

- conf update 失败时，reload 不得继续执行
- reload 失败时，不得把整次动作标记为成功
- 如果同步已经成功，状态中必须保留新 revision、`current_version`、`resolved_tag`

## 锁与状态文件

在工程目录下维护：

```text
.run/project_remote.lock
.run/project_remote_state.json
```

作用：

- 防止并发更新
- 防止更新与 reload 交叉执行
- 记录最近一次成功 revision、最近一次 `current_version`、最近一次 `resolved_tag`

“首次初始化”的判断规则：

- 以 `.run/project_remote_state.json` 是否存在为准
- 不是看工程目录是否有内容

## 安全约束

- `repo` 只能来自本地配置或 `wproj init --repo` 引导参数，不从运行时请求体注入
- 不向用户暴露底层仓库操作命令
- 外部调用方不依赖仓库操作细节
- 本地工作区不干净时固定拒绝更新
- 远端认证沿用标准 SSH key / token 机制

## 与现有能力的关系

- `wproj self update`: 更新 Warp Parse 工具自身
- `wproj init --repo`: 首次生成工程骨架并触发首轮远端同步
- `wproj conf update`: 手工触发工程配置同步
- `wproj engine reload`: 运行时激活动作；带 `--update` 时先同步、后 reload
- HTTP admin API reload: `wproj engine reload` 的远程调用面

这几个能力是分层关系，不应互相替代。

## MVP

首版建议实现：

- `conf/wparse.toml` 中的可选 `[project_remote]`
- `wproj init --repo`
- `wproj conf update`
- `wproj engine reload --update`
- HTTP admin API 的 `update` / `version` 参数
- `init_version` 配置语义
- `--version` 动作参数语义
- `current_version` 与 `resolved_tag` 状态语义
- 基于 version/tag 的版本解析与同步
- 无 tag 时自动回退到远端默认分支 `HEAD`
- 固定的脏工作区保护
- 最小 WPL 校验
- 锁文件和状态文件
- JSON 输出

后续再考虑：

- 定时轮询更新
- 多分支 / 多 remote
- 更细粒度校验策略
- 审批式发布

## 验收标准

- 使用者只需要配置 repo 和开关，不需要直接使用 Git
- `wproj init --repo` 可以直接完成首次远端初始化
- `wproj conf update` 可以完成后续更新
- `wproj engine reload --update` 与 HTTP admin API reload 会先执行 conf update
- manual update 和 runtime reload 复用同一套同步核心
- HTTP admin API 可显式指定是否更新以及目标版本
- `init_version` 仅用于首次初始化
- 升级和回退通过动作参数 `version` 完成
- `current_version` / `resolved_tag` 只写入状态，不写回固定配置
- 远端无 release tag 时，自动回退到默认分支 `HEAD`
- conf update 失败时 reload 不会继续
- 校验失败时 reload 不会继续
- 结果可结构化追踪

## 日志与排障

建议把远程更新链路的日志视为排障主入口。

关键日志：

- `project remote sync start`
- `project remote sync target resolved`
- `project remote sync tag resolved`
- `project remote sync diff`
- `project remote sync apply managed dirs`
- `project remote sync done`
- `project remote sync apply failed`
- `project remote sync rollback done`
- `wproj conf update start`
- `wproj conf update validate failed`
- `wproj conf update rollback done`
- `admin api project update start`
- `admin api project update done`
- `admin api project update failed`
- `admin api project rollback done`

建议重点关注的字段：

- `request_id`
- `work_root`
- `requested_version`
- `current_version`
- `resolved_tag`
- `from_revision`
- `to_revision`
- `changed`
- `error`

常用排障示例：

```bash
grep -E "project remote sync|wproj conf update|admin api project update" data/logs/wparse.log
```

```bash
grep -E "project remote sync apply failed|validate failed|rollback" data/logs/wparse.log
```

用途：

- 看请求想更新到哪个版本
- 看最终解析到了哪个 tag / 分支 / commit
- 看这次是否真的切换了受管目录
- 看失败发生在同步、检测还是 reload 阶段
- 看回滚是否已经执行，以及回滚是否失败

## 对应英文版

- [../en/project_remote_sync_design.md](../en/project_remote_sync_design.md)
