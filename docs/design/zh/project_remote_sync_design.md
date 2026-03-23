# 远程规则版本更新设计

- 状态: Draft
- 范围: `wproj init --remote`、`wproj conf update`、`wparse restart`、HTTP admin API 运行时指令

## 需求归纳

本设计以以下需求为准：

1. 工程配置中需要提供远程 Git repo 地址和总开关
2. 使用者不需要也不应该直接使用 Git
3. `wproj` 需要提供统一入口：`wproj conf update`
4. `wparse` 在收到重启指令时，也必须先完成 conf update
5. 手工更新和运行时重启不能各自维护一套同步逻辑
6. HTTP admin API 需要提供“是否更新”和“版本号”参数
7. `wproj init` 需要支持直接从远端初始化工程

## 设计结论

把“远程工程同步”定义为一个共享的项目能力，而不是某个单独二进制的私有行为。

- 配置放在项目级配置文件中，由 `wproj` 和 `wparse` 共同读取
- `wproj init --remote` 负责首次初始化时提供远端引导参数并触发首次同步
- `wproj conf update` 是显式运维入口
- `wparse restart` 是隐式触发入口
- 三个入口都复用同一个 conf update 核心流程

用户面对的是“更新工程配置”这个动作，而不是 `git clone` / `git pull`。
用户面对的是“从远程版本源同步规则”这个动作，而不是底层仓库操作细节。

## 非目标

- 不向用户暴露底层仓库操作细节
- 不设计成通用仓库管理工具
- 不解决规则编写问题
- 不替代 `wproj self update`
- 首版不做多 remote、多分支编排

## 配置边界

该能力属于 `wparse` 的可选增强配置，但会被 `wproj` 复用读取。

因此更合适的放置位置是：

```text
conf/wparse.toml
```

做法是：

- `wparse.toml` 继续作为唯一必需配置
- `[project_remote]` 作为可选配置段存在
- 未配置 `[project_remote]` 时，`wparse` 仍可正常启动
- `wproj conf update` 也从同一个 `wparse.toml` 读取该段

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
- `init_version`: 首次初始化版本；仅在本地尚未初始化工程内容时使用

首版最小必需字段：

- `enabled`
- `repo`

建议字段：

- `init_version`

如果仓库中的发布 tag 采用统一前缀约定，例如 `v1.4.2`，则版本解析规则由系统内置：

- 配置写 `init_version = "1.4.2"`
- 初始化时解析为 tag `v1.4.2`

如果后续需要兼容其他 tag 规范，再扩展，不在首版暴露额外模式字段。

可用于联调和测试的仓库示例：

- `https://github.com/wp-labs/editor-monitor-conf.git`

## 外部接口

### 1. 显式入口

首次远程初始化入口：

```bash
wproj init --remote <REPO> [--version <VERSION>]
```

语义：

- `wproj init --remote` 先初始化本地工程骨架
- `--remote` / `--version` 只作为首次同步的引导参数
- 若未显式传入 `--version`，则先解析远端最新发布版本
- 然后直接复用 `wproj conf update` 的同步、校验、回滚流程完成首次同步
- 首次同步完成后，以远端仓库中的工程配置为准

日常显式更新入口：

```bash
wproj conf update
```

建议支持：

```bash
wproj conf update [OPTIONS]

Options:
  -w, --work-root <DIR>
      --version <VERSION>
      --dry-run
      --json
      --reason <TEXT>
```

语义：

- `wproj conf update` 负责执行已配置远端的版本同步
- 用户不需要预先判断当前是首次部署还是日常更新
- 用户不需要直接处理仓库操作
- `--version` 用于显式指定本次更新目标版本，支持升级和回退
- 未显式指定 `--version` 时：
- 首次初始化优先使用 `init_version`
- 已初始化工程的后续更新可按“最新发布版本”规则处理

### 2. 隐式入口

`wparse` 收到 restart 指令时：

1. 读取 `conf/wparse.toml`
2. 若 `project_remote.enabled = true`
3. 解析本次 restart 指令是否显式携带版本
4. 若携带版本，则以该版本执行 conf update
5. 若未携带版本：
6. 首次初始化使用 `init_version`
7. 非首次场景按“最新发布版本”规则执行 conf update
8. conf update 成功后，再执行 restart
9. conf update 失败，则拒绝本次 restart

这条规则是本设计的核心约束。

### 3. HTTP admin API 入口

运行时管理面需要能表达两件事：

- 本次运行时动作前是否先做 conf update
- 如果要更新，更新到哪个版本

建议在管理面请求体中统一增加：

- `update`: `bool`
- `version`: `string | null`

推荐语义：

- `update = true`: 本次运行时动作前先执行 conf update
- `update = false`: 本次运行时动作不执行 conf update
- `version` 非空: 以该版本为本次 conf update 目标版本
- `version` 为空: 走默认版本选择规则

建议请求体示例：

```json
{
  "update": true,
  "version": "1.4.3",
  "wait": true,
  "timeout_ms": 15000,
  "reason": "restart with rule update"
}
```

如果未来区分 `reload` 和 `restart` 两类 endpoint，则二者都应复用这两个参数，而不是各自定义不同版本字段。

## 统一核心流程

`wproj init --remote`、`wproj conf update` 与 `wparse restart` 必须复用同一个核心模块。

建议内部抽象为：

```text
project_sync_core
```

该核心模块负责：

1. 读取项目同步配置
2. 解析本次动作的目标版本
3. 在独立 remote 目录中完成版本更新
4. 为当前工作目录中的受管目录白名单生成 backup
5. 将 remote 目录中的受管目录白名单复制到当前工作目录
6. 执行更新后检查
7. 成功后返回结构化结果
8. 失败时用 backup 还原受管目录白名单

`wproj` 和 `wparse` 只负责：

- 选择何时调用
- 决定同步成功后的后续动作是 `reload` 还是 `restart`

HTTP admin API 在 host 层只负责把请求参数映射成统一动作上下文：

- `trigger = admin_api`
- `update = true/false`
- `requested_version = <version or null>`
- `runtime_action = reload/restart`

## 目录模型

本设计不在当前工作目录内直接执行仓库切换，而是采用三目录模型：

- `remote`: 远程工程缓存目录，用于 `clone` / `pull` / 版本切换
- `current`: 当前运行工作目录，即 `wparse` 实际读取的目录
- `backup`: 当前工作目录的备份目录，用于失败回滚

约束：

- 使用者只面对 `wproj conf update` 或运行时 update/reload 指令
- Git 操作只发生在 `remote` 目录
- `current` 目录始终保持“可运行快照”语义
- reload 或检查失败时，必须能从 `backup` 恢复
- 目录切换采用“受管目录白名单”，而不是“整个工作目录全量覆盖”

建议的受管目录白名单：

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

无论入口来自 `wproj init --remote`、`wproj conf update` 还是运行时 update/reload，请统一采用以下流程：

1. 根据配置定位 remote 目录
2. 在 remote 目录执行远端更新并切到目标版本
3. 将 current 中的受管目录白名单备份到 backup
4. 将 remote 中的受管目录白名单复制到 current
5. 执行更新后检查
6. 检查成功后，若调用方要求 reload/restart，则继续后续动作
7. 若检查失败或 reload 失败，则用 backup 还原受管目录白名单

对于 `wproj init --remote`，在进入上述流程之前还要先做两步：

1. 生成本地工程骨架
2. 将 `--remote` / `--version` 作为首次同步的引导参数传入核心流程

这样可以保证：

- 更新源和运行目录职责分离
- 当前运行目录不依赖自身是 Git 工程
- rollback 是目录级回滚，而不是仓库状态回滚

## 基于发布版本的同步语义

对用户隐藏底层仓库实现之后，系统需要提供稳定的版本同步语义：

- 首次初始化时，同步到 `init_version` 或显式指定版本
- 后续更新时，同步到本次动作指定版本
- 未显式指定版本时，可按“最新发布版本”规则解析目标版本
- 当本地目录状态不满足安全约束时，直接失败

## 同步策略

首版建议只支持可预测的安全路径：

- remote 更新目标是明确的发布版本，而不是移动中的分支头
- 本次动作的目标版本必须解析到唯一发布 tag
- 白名单目录覆盖前必须先完成 backup
- 检查或 reload 失败时必须恢复 backup

这样可以保证“更新规则”对应的是一个稳定、可审计的发布版本，并且失败时能回到上一个可运行目录快照。

## 为什么要区分 `init_version`、动作版本和当前版本

如果发布是通过版本 tag 管理，那么：

- 初始化关注“第一次应该从哪个版本起步”
- 更新动作关注“这一次要切到哪个版本”
- 运行状态关注“当前实际上跑在哪个版本”

这三件事不应混成一个字段。

建议分层：

- `init_version`: 固定配置，只用于首次初始化
- `version`: 动作参数，只用于本次 `conf update` / `restart`
- `current_version`: 状态字段，只记录当前结果

这样既支持：

- 首次部署
- 日常升级
- 显式回退

又不会让配置文件长期承载一个不断变化的“目标版本”。

## restart 语义

收到 restart 指令时，`wparse` 的语义不再是“立即用本地目录重启”，而是：

1. 先把 remote 目录同步到配置指定的目标版本
2. 备份 current 中的受管目录白名单
3. 用 remote 中的受管目录白名单覆盖 current
4. 执行更新后检查
5. 检查成功后，再基于 current 目录执行 restart
6. 若 restart 失败，则用 backup 恢复受管目录白名单

如果 `project_remote.enabled = false`：

- `wparse` 可以继续本地 restart
- 但结果里必须明确标记“本次 restart 未执行远程同步”

## manual update 语义

`wproj conf update` 的语义是：

1. 将 remote 目录同步到本次动作指定的发布版本
2. 备份当前工作目录中的受管目录白名单到 backup
3. 用 remote 中的受管目录白名单覆盖当前工作目录
4. 执行更新后检查
5. 成功则返回更新结果
6. 失败则恢复 backup

`wproj conf update` 不隐含自动 reload，也不隐含 restart。

后续是否执行：

- `wproj engine reload`
- `wparse restart`

由显式运行时动作决定，而不是由配置开关决定。

版本选择规则：

- 若命令显式传入 `--version`，则以该版本为准
- 若首次初始化且未显式传入版本，则使用 `init_version`
- 若非首次且未显式传入版本，则更新到最新发布版本

HTTP admin API 的版本选择规则与 CLI 保持一致：

- 若请求体 `update = false`，则忽略 `version`
- 若 `update = true` 且 `version` 非空，则以该版本为准
- 若 `update = true` 且 `version` 为空：
- 首次初始化使用 `init_version`
- 非首次场景更新到最新发布版本

为避免歧义，建议对非法组合直接拒绝：

- `update = false` 且 `version` 非空` -> `请求非法

## 最小校验门禁

无论由哪个入口触发，同步完成后都需要最小校验门禁。

首版建议：

- 至少执行 WPL 相关校验

等价目标是：

```bash
wproj check --what wpl --fail-fast
```

如果校验失败：

- `wproj conf update` 返回失败
- `wparse restart` 不能继续执行 restart

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
  "runtime_action": "restart",
  "runtime_result": "success"
}
```

建议的 `sync_result`：

- `disabled`
- `cloned`
- `up_to_date`
- `updated`
- `init_version_missing`
- `version_not_found`
- `dirty_worktree`
- `remote_mismatch`
- `invalid_worktree`
- `validation_failed`

建议的 `trigger`：

- `manual`
- `wparse_restart`
- `admin_api`

建议的 `runtime_action`：

- `none`
- `reload`
- `restart`

## 失败语义

必须明确区分三类失败：

1. 同步失败
2. 同步成功但校验失败
3. 同步与校验成功，但 runtime 动作失败

特别要求：

- conf update 失败时，`wparse restart` 不得继续执行
- runtime 失败时，不得把整次动作标记为成功
- 如果同步已经成功，状态中必须保留新 revision 和当前 version 信息

## 锁与状态文件

建议在工程目录下维护：

```text
.run/conf_update.lock
.run/conf_update_state.json
```

作用：

- 防止并发更新
- 防止更新与重启交叉执行
- 记录最近一次成功 revision、最近一次 current_version、最近一次 trigger、最近一次失败原因

## 安全约束

- `repo` 只能来自本地配置，不从交互输入注入
- 不应向用户暴露底层仓库操作命令
- 不应让外部调用方依赖仓库操作细节
- 本地工作区不干净时固定拒绝更新，不提供配置绕过
- 远端认证沿用标准 SSH key / token 机制，但不在 CLI 上重新发明一套认证协议

## 与现有能力的关系

- `wproj self update`: 更新 Warp Parse 工具自身
- `wproj init --remote`: 首次生成工程骨架并触发首轮远端同步
- `wproj conf update`: 手工触发工程配置同步
- `wparse restart`: 运行时触发“先同步、后重启”
- `wproj engine reload`: 独立的运行时激活动作

这几个能力是分层关系，不应互相替代。

## MVP

首版建议实现：

- `conf/wparse.toml` 中的可选 `[project_remote]`
- `wproj init --remote`
- `wproj conf update`
- `wparse restart` 前置 conf update
- HTTP admin API 的 `update` / `version` 参数
- `init_version` 配置语义
- `--version` 动作参数语义
- `current_version` 状态记录语义
- 基于 version/tag 的版本解析与同步
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
- `wproj init --remote` 可以直接完成首次远端初始化
- `wproj conf update` 可以完成后续更新
- `wparse` 收到 restart 指令时会先执行 conf update
- manual update 和 runtime restart 复用同一套同步核心
- HTTP admin API 可显式指定是否更新以及目标版本
- `init_version` 仅用于首次初始化
- 升级和回退通过动作参数 `version` 完成
- `current_version` 仅写入状态，不写回固定配置
- conf update 失败时 restart 不会继续
- 校验失败时 reload / restart 都不会继续
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
- 看最终解析到了哪个 tag / commit
- 看这次是否真的切换了受管目录
- 看失败发生在同步、检测还是 reload 阶段
- 看回滚是否已经执行，以及回滚是否失败

## 对应英文版

- [../en/project_remote_sync_design.md](../en/project_remote_sync_design.md)
