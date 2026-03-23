# 远程工程拉取与规则热更新 SOP

## 适用范围

本文用于整理以下运维任务：

- 在远程机器上拉取一个 WP 工程
- 后续通过 `git pull` 更新工程内容
- 在不中断 `wparse daemon` 进程的前提下触发规则或模型重载

本文不覆盖具体 `parse.wpl` 的编写和调试。如果任务进入规则编写阶段，应转到独立的 WPL 验证流程。

## 目标

把“更新规则”拆成两个独立动作：

1. 更新工程目录中的文件
2. 对在线运行的解析引擎发出重载请求

这样可以避免把“代码/规则同步”和“进程生命周期管理”混在一起。

## 前提条件

远程机器应满足：

- 已安装 `git`
- 已安装可用的 `wproj`、`wparse`，或可通过 `cargo run --bin ...` 启动
- 已约定固定工作目录，例如 `/srv/wp/<project>`
- 目标工程仓库已经包含完整的 WP 目录结构与配置

建议先确认：

- 当前运行模式是 `wparse daemon`，而不是 `batch`
- `conf/wparse.toml` 中的 `wpl`、source、sink 路径都相对 `work-root` 可用
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

### 1. 拉取工程

```bash
mkdir -p /srv/wp
git clone <repo> /srv/wp/<project>
cd /srv/wp/<project>
```

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

拉取最新内容：

```bash
git pull
```

在发起重载前先做最小校验：

```bash
wproj check --what wpl --fail-fast
```

查看当前运行状态：

```bash
cargo run --bin wproj -- engine status --work-root .
```

触发同步重载：

```bash
cargo run --bin wproj -- engine reload \
  --work-root . \
  --request-id rule-$(date +%Y%m%d%H%M%S) \
  --reason "rule update"
```

重载完成后再次确认状态：

```bash
cargo run --bin wproj -- engine status --work-root .
```

### 结果判断

重点关注以下字段：

- `last_reload_request_id`
- `last_reload_result`
- `reloading`

常见结果：

- `reload_done`：重载成功完成
- `running`：请求已接收但仍在执行
- `reload_in_progress`：已有其他重载在进行中

如果返回中带有以下内容，表示优雅 drain 超时后使用了兜底替换：

- `force_replaced = true`
- `warning = "graceful drain timed out, fallback to force replace"`

这不是立即失败，但需要额外关注线上流量与错误日志。

## 推荐发布门禁

为避免把错误规则直接打进在线实例，建议将更新流程固定为：

1. `git pull`
2. `wproj check --what wpl --fail-fast`
3. `wproj engine status`
4. `wproj engine reload`
5. 再次 `wproj engine status`
6. 检查 `data/logs/`、解析统计和目标 sink 输出

如果工程里除了 WPL 还有 source/sink 或主配置变更，发布前建议跑完整检查：

```bash
wproj check
```

## 回滚 SOP

当新规则重载后出现解析错误、字段异常或下游告警时，不要先重启进程，先回滚工程版本并再次重载：

```bash
cd /srv/wp/<project>
git log --oneline -n 5
git checkout <last-known-good-revision>
wproj check --what wpl --fail-fast
cargo run --bin wproj -- engine reload \
  --work-root . \
  --request-id rollback-$(date +%Y%m%d%H%M%S) \
  --reason "rollback rule set"
```

回滚后继续检查：

- `wproj engine status`
- `data/logs/`
- 目标 sink 输出是否恢复

如果后续仍需回到分支最新版本，再执行：

```bash
git checkout <branch>
git pull
```

## 远端覆盖方式

如果 `wproj` 不是在目标工作目录内执行，可以显式覆盖目标：

```bash
cargo run --bin wproj -- engine status \
  --work-root /srv/wp/<project> \
  --admin-url http://127.0.0.1:19090 \
  --token-file /srv/wp/<project>/runtime/admin_api.token
```

同理，`engine reload` 也可以使用同样的覆盖参数。

## 验收清单

一次合格的远程规则更新至少应满足：

- 仓库已成功 `clone` 或 `pull`
- `wproj check --what wpl --fail-fast` 通过
- `wparse daemon` 持续运行，无需重启
- `wproj engine reload` 返回可接受结果
- 重载后的运行状态可查询
- 新规则效果已通过统计、日志或下游输出验证

## 最小命令集

首次部署：

```bash
git clone <repo> /srv/wp/<project>
cd /srv/wp/<project>
wproj check
cargo run --bin wparse -- daemon --work-root .
```

日常更新：

```bash
cd /srv/wp/<project>
git pull
wproj check --what wpl --fail-fast
cargo run --bin wproj -- engine reload \
  --work-root . \
  --request-id rule-$(date +%Y%m%d%H%M%S) \
  --reason "rule update"
cargo run --bin wproj -- engine status --work-root .
```
