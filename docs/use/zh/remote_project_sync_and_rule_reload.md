# 远程工程拉取与规则热更新 SOP

## 适用范围

本文用于整理以下运维任务：

- 在远程机器上初始化一个来自远端版本仓库的 WP 工程
- 后续通过 `wproj conf update` 更新工程内容
- 在不中断 `wparse daemon` 进程的前提下触发规则或模型重载

本文不覆盖具体 `parse.wpl` 的编写和调试。如果任务进入规则编写阶段，应转到独立的 WPL 验证流程。

## 目标

把“更新规则”拆成两个独立动作：

1. 更新工程目录中的受管配置文件
2. 对在线运行的解析引擎发出重载请求

这样可以避免把“规则同步”和“进程生命周期管理”混在一起。

## 前提条件

远程机器应满足：

- 已安装可用的 `wproj`、`wparse`，或可通过 `cargo run --bin ...` 启动
- 已约定固定工作目录，例如 `/srv/wp/<project>`
- 目标远端仓库已经包含完整的 WP 工程配置内容

建议先确认：

- 当前运行模式是 `wparse daemon`，而不是 `batch`
- `conf/wparse.toml` 中的路径都相对 `work-root` 可用
- 远程仓库可通过 release tag 发布版本；如果暂时没有 release tag，也至少要有可用的默认分支
- 远程机器具备拉取仓库所需的 SSH key 或访问令牌

## 启用运行时管理面

规则热更新依赖运行时管理面。按 [engine_admin_usage.md](engine_admin_usage.md) 配置 `conf/wparse.toml`：

```toml
[admin_api]
enabled = true
bind = "127.0.0.1:19090"
request_timeout_ms = 15000
max_body_bytes = 4096

[admin_api.tls]
enabled = false
cert_file = ""
key_file = ""

[admin_api.auth]
mode = "bearer_token"
token_file = "runtime/admin_api.token"
```

启动前准备 token 文件：

```bash
mkdir -p runtime
printf 'replace-with-a-secret-token\n' > runtime/admin_api.token
chmod 600 runtime/admin_api.token
```

约束：

- `batch` 模式不暴露管理面
- Unix 下 token 文件权限必须是 owner-only
- 如果绑定非回环地址，必须启用 TLS

## 首次部署

### 1. 远端初始化工程

初始化到显式版本：

```bash
wproj init \
  --work-root /srv/wp/<project> \
  --repo https://github.com/wp-labs/editor-monitor-conf.git \
  --version 1.4.2
```

初始化到默认目标版本：

```bash
wproj init \
  --work-root /srv/wp/<project> \
  --repo https://github.com/wp-labs/editor-monitor-conf.git
```

说明：

- `wproj init --repo` 会先生成本地工程骨架
- `--repo` / `--version` 只作为首次同步引导参数
- 然后自动复用 `wproj conf update` 完成首次同步和校验
- 首次同步完成后，以远端仓库中的工程配置为准
- 显式 `--version` 只用于按版本 tag 初始化，也可用于指定首个回退点
- 如果不指定 `--version`，会先尝试解析远端最新 release tag
- 如果远端没有任何 release tag，会自动回退到远端默认分支 `HEAD`

当默认回退到远端 `HEAD` 时，典型输出语义为：

- `Version: main` 或 `master`
- `Tag: HEAD@main` 或 `HEAD@master`

### 2. 校验工程完整性

```bash
wproj check
wproj data stat
```

如果二进制未安装到 `PATH`，可改用：

```bash
cargo run --bin wproj -- check
cargo run --bin wproj -- data stat
```

### 3. 启动 daemon

```bash
cargo run --bin wparse -- daemon --work-root .
```

启动后建议立刻检查运行时状态：

```bash
cargo run --bin wproj -- engine status --work-root .
```

应重点确认：

- `accepting_commands = true`
- `reloading = false`

## 日常规则更新 SOP

### 标准流程

进入目标工程目录：

```bash
cd /srv/wp/<project>
```

先更新工程内容：

```bash
wproj conf update --work-root /srv/wp/<project>
```

如需显式升级或回退到某个版本：

```bash
wproj conf update --work-root /srv/wp/<project> --version 1.4.3
```

默认版本选择规则：

- 首次初始化且配置了 `init_version` 时，优先使用 `init_version`
- 非首次更新时，优先解析远端最新 release tag
- 如果远端没有 release tag，则回退到远端默认分支 `HEAD`

在发起重载前先做最小校验：

```bash
wproj check --what wpl --fail-fast
```

查看当前运行状态：

```bash
cargo run --bin wproj -- engine status --work-root .
```

触发仅重载本地已更新内容：

```bash
cargo run --bin wproj -- engine reload \
  --work-root . \
  --request-id rule-$(date +%Y%m%d%H%M%S) \
  --reason "rule reload"
```

如果希望“更新并重载”合并成一个运行时动作：

```bash
cargo run --bin wproj -- engine reload \
  --work-root . \
  --update \
  --request-id update-$(date +%Y%m%d%H%M%S) \
  --reason "rule update and reload"
```

如需显式升级或回退并重载：

```bash
cargo run --bin wproj -- engine reload \
  --work-root . \
  --update \
  --version 1.4.3 \
  --request-id update-rollback-$(date +%Y%m%d%H%M%S) \
  --reason "switch release and reload"
```

重载完成后再次确认状态：

```bash
cargo run --bin wproj -- engine status --work-root .
```

### 结果判断

`wproj conf update` 重点关注：

- `Request`
- `Version`
- `Tag`
- `Changed`

`wproj engine reload` 重点关注：

- `Result`
- `Updated`
- `Request V`
- `Current V`
- `Tag`

常见结果：

- `reload_done`：重载成功完成
- `running`：请求已接收但仍在执行
- `reload_in_progress`：已有其他重载在进行中
- `update_in_progress`：已有其他工程更新在进行中
- `update_failed`：更新阶段失败，未进入 reload

版本字段语义：

- `Request V`：本次请求显式指定的版本；未指定时为空
- `Current V`：本次更新后实际生效的版本
- `Tag`：本次更新解析到的远端目标；tag 发布时形如 `v1.4.3`
- 如果回退到默认分支 `HEAD`，`Tag` 形如 `HEAD@main`

如果返回中带有以下内容，表示优雅 drain 超时后使用了兜底替换：

- `force_replaced = true`
- `warning = "graceful drain timed out, fallback to force replace"`

这不是立即失败，但需要额外关注线上流量与错误日志。

## 推荐发布门禁

为避免把错误规则直接打进在线实例，建议将更新流程固定为：

1. `wproj conf update`
2. `wproj check --what wpl --fail-fast`
3. `wproj engine status`
4. `wproj engine reload`
5. 再次 `wproj engine status`
6. 检查 `data/logs/`、解析统计和目标 sink 输出

如果要把“更新 + 重载”作为一个原子动作从管理面触发，则门禁点应前移到发布仓库侧，确保远端版本在进入线上前已完成检查。

如果工程里除了 WPL 还有 source/sink 或主配置变更，发布前建议跑完整检查：

```bash
wproj check
```

## 回滚 SOP

当新规则重载后出现解析错误、字段异常或下游告警时，不要先重启进程，先回滚工程版本并再次重载：

```bash
wproj conf update --work-root /srv/wp/<project> --version 1.4.2
cargo run --bin wproj -- engine reload \
  --work-root /srv/wp/<project> \
  --request-id rollback-$(date +%Y%m%d%H%M%S) \
  --reason "rollback rule set"
```

也可以直接通过一个运行时动作完成：

```bash
cargo run --bin wproj -- engine reload \
  --work-root /srv/wp/<project> \
  --update \
  --version 1.4.2 \
  --request-id rollback-$(date +%Y%m%d%H%M%S) \
  --reason "rollback and reload"
```

回滚后继续检查：

- `wproj engine status`
- `data/logs/`
- 目标 sink 输出是否恢复

如果后续仍需回到最新发布版本，再执行：

```bash
wproj conf update --work-root /srv/wp/<project>
```

## 远端覆盖方式

如果 `wproj` 不是在目标工作目录内执行，可以显式覆盖目标：

```bash
cargo run --bin wproj -- engine status \
  --work-root /srv/wp/<project> \
  --admin-url http://127.0.0.1:19090 \
  --token-file /srv/wp/<project>/runtime/admin_api.token
```
