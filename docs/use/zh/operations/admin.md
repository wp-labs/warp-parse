# Warp Parse 运行时管理面使用说明

## 范围

当前已提供的运行时管理能力仅包含：

- `wparse daemon` 暴露受鉴权保护的 HTTP 管理面
- `wproj engine status` 查询运行时状态
- `wproj engine reload` 触发 `LoadModel` 重载

`batch` 模式不会暴露管理面 HTTP 服务。

当前没有独立的 runtime restart 接口。远端规则更新只与 `reload` 联动。

## 启用管理面

在项目根目录的 `wparse.local.toml` 中配置：

> `wparse.local.toml` 是站点本地覆盖层，位于 `conf/` 之外。`project_remote`
> 只会替换 `conf/`、`models/`、`topology/`、`connectors/`，因此远程发布不会
> 覆盖这里的端口、TLS 和 token 路径。
>
> 兼容旧工程：若 `wparse.local.toml` 不存在，仍会读取 `conf/wparse.toml` 中的
> `[admin_api]`，并输出一条告警。`wproj init` / `wproj conf update` 会自动迁移。
> 若文件存在但缺少 `[admin_api]` 段（或 TOML 非法），会直接报错，不会静默回退。
>
> 建议把 `wparse.local.toml` 加入项目 `.gitignore`，避免误提交到发布仓库。

示例（`wparse.local.toml`）：

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
token_file = "${HOME}/.warp_parse/admin_api.token"
```

启动前创建 token 文件：

```bash
wpadm token generate --out "${HOME}/.warp_parse/admin_api.token"
```

`wpadm token generate` 写入 32 字节目随机数（64 位 hex），原子创建为 `0600`，
只打印路径。目标已存在时拒绝覆盖（需 `--force`），且不会跟随输出路径上的符号链接。

约束：

- token 文件必须是 group/other **不可写**的普通文件
  （`0600`、`0640`、`0440`、`0444` 均可用；`0660`、`0664`、`0777` 被拒绝）
  ——这是为了支持只读的 Docker/Kubernetes secret 挂载
- 允许符号链接，但校验解析后的目标，因此 Kubernetes 的 projected secret
  （`token -> ..data/token`）可以正常使用
- 文件必须恰好包含一个非空行
- 非回环地址绑定必须启用 TLS
- 鉴权固定为 `bearer_token`，没有 auth mode 配置项

引擎只在启动时读取一次 token，因此轮转需要替换文件并重启 daemon。

## Token 工具

```bash
# 生成 token（只打印路径）
wpadm token generate --out /run/secrets/wparse_admin_token

# 体检：类型、符号链接目标、权限、属主、内容、指纹
wpadm token check --file /run/secrets/wparse_admin_token

# 主机文件 / CI 场景可要求更严的 owner-only
wpadm token check --file "${HOME}/.warp_parse/admin_api.token" --require-owner-only

# 查看当前生效的 token（值输出到 stdout，诊断信息输出到 stderr）
wpadm token show --token-file /run/secrets/wparse_admin_token
```

`wpadm` 的解析顺序如下，因此在容器内或 CI 中（没有 `wparse.local.toml`）也能用：

1. `--token-file <path>`
2. `--token-env <NAME>`
3. `$WPARSE_ADMIN_TOKEN`
4. 本地 admin API 配置中的 `token_file`

所有诊断输出只打印 8 位 `token_fp`（`sha256(token)` 前缀），不打印 token 本身。
把引擎启动日志中的
`admin api auth source=file path=... token_fp=...` 与客户端的指纹对比，即可确认双方持有同一个 token。

容器部署请挂载**目录**而不是单个文件（`-v ./secrets:/run/secrets:ro`）：
对单文件 bind mount 做 rename 替换不会传播到容器内。

## 启动方式

```bash
wparse daemon --work-root .
```

启动后可访问：

- `GET /admin/v1/runtime/status`
- `POST /admin/v1/reloads/model`

## 查询运行时状态

文本输出：

```bash
wproj engine status --work-root .
```

JSON 输出：

```bash
wproj engine status --work-root . --json
```

## 触发重载

等待完成：

```bash
wproj engine reload \
  --work-root . \
  --request-id manual-reload-001 \
  --reason "manual model refresh"
```

异步返回：

```bash
wproj engine reload \
  --work-root . \
  --wait false \
  --request-id manual-reload-async-001 \
  --reason "async refresh"
```

先更新远端工程，再执行 reload：

```bash
wproj engine reload \
  --work-root . \
  --update \
  --request-id update-reload-001 \
  --reason "rule update and reload"
```

显式切换到某个版本再 reload：

```bash
wproj engine reload \
  --work-root . \
  --update \
  --version 1.4.3 \
  --request-id update-reload-002 \
  --reason "switch release and reload"
```

JSON 输出：

```bash
wproj engine reload \
  --work-root . \
  --update \
  --request-id manual-reload-json-001 \
  --reason "json output" \
  --json
```

### 双仓库模式

在双仓库模式下（`[project_remote.models]` + `[project_remote.infra]`），每次 update 必须通过 `--group` 指定更新目标：

更新 models 组：

```bash
wproj engine reload \
  --work-root . \
  --update \
  --group models \
  --request-id update-models-001 \
  --reason "update models group"
```

更新 infra 组并指定版本：

```bash
wproj engine reload \
  --work-root . \
  --update \
  --group infra \
  --version 0.1.7 \
  --request-id update-infra-002 \
  --reason "switch infra to 0.1.7"
```

约束：

- 双仓库模式下 `--update` 必须搭配 `--group`，否则报错
- `--group` 仅在与 `--update` 联用时有效；仅 reload 不 update 时忽略

## HTTP API 示例

仅 reload：

```bash
curl -sS \
  -X POST \
  -H 'Authorization: Bearer replace-with-a-secret-token' \
  -H 'Content-Type: application/json' \
  -H 'X-Request-Id: manual-http-reload-001' \
  http://127.0.0.1:19090/admin/v1/reloads/model \
  -d '{
    "wait": true,
    "reason": "manual http reload"
  }'
```

更新后 reload：

```bash
curl -sS \
  -X POST \
  -H 'Authorization: Bearer replace-with-a-secret-token' \
  -H 'Content-Type: application/json' \
  -H 'X-Request-Id: manual-http-update-reload-001' \
  http://127.0.0.1:19090/admin/v1/reloads/model \
  -d '{
    "wait": true,
    "update": true,
    "version": "1.4.3",
    "timeout_ms": 15000,
    "reason": "switch release and reload"
  }'
```

双仓库模式 —— 更新 models 组：

```bash
curl -sS \
  -X POST \
  -H 'Authorization: Bearer replace-with-a-secret-token' \
  -H 'Content-Type: application/json' \
  -H 'X-Request-Id: dual-models-update-001' \
  http://127.0.0.1:19090/admin/v1/reloads/model \
  -d '{
    "wait": true,
    "update": true,
    "group": "models",
    "timeout_ms": 15000,
    "reason": "update models from dual repo"
  }'
```

双仓库模式 —— 更新 infra 组：

```bash
curl -sS \
  -X POST \
  -H 'Authorization: Bearer replace-with-a-secret-token' \
  -H 'Content-Type: application/json' \
  -H 'X-Request-Id: dual-infra-update-001' \
  http://127.0.0.1:19090/admin/v1/reloads/model \
  -d '{
    "wait": true,
    "update": true,
    "group": "infra",
    "version": "0.1.7",
    "timeout_ms": 15000,
    "reason": "update infra to 0.1.7"
  }'
```

API 约束：

- `update = false` 时不能传 `version`
- `update = true` 且 `version` 为空时，服务端按默认版本选择规则解析目标
- 双仓库模式下 `update = true` 时必须传 `group`（`"models"` 或 `"infra"`），否则返回错误
- `group` 仅在 `update = true` 时有效

### 请求字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `wait` | bool | 否 | 是否等待 reload 完成后返回，默认 `true` |
| `update` | bool | 否 | 是否先更新工程内容，默认 `false` |
| `version` | string | 否 | 目标版本，仅 `update = true` 时有效 |
| `group` | string | 否 | 更新目标组：`"models"` 或 `"infra"`，双仓库模式下 `update = true` 时必填 |
| `timeout_ms` | number | 否 | `wait = true` 时的等待超时，未指定时使用服务端 `admin_api.request_timeout_ms` |
| `reason` | string | 否 | 附加原因说明，用于日志 |

### 响应字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `request_id` | string | 请求标识 |
| `accepted` | bool | 是否已被接受 |
| `result` | string | 当前结果码 |
| `update` | bool | 是否包含工程更新 |
| `requested_version` | string | 显式请求的版本，自动模式下为空 |
| `current_version` | string | 本次更新实际激活的版本 |
| `resolved_tag` | string | 解析到的远端目标 |
| `group` | string | 双仓库模式下本次更新的目标组（`"models"` / `"infra"`），单仓库模式下省略 |
| `force_replaced` | bool | 是否因优雅 drain 超时强制替换 |
| `warning` | string | 警告信息 |
| `error` | string | 错误信息 |

### 状态响应中的 project_version

单仓库模式下，`project_version` 为字符串：

```json
{
  "project_version": "1.0"
}
```

双仓库模式下，`project_version` 为对象，按组返回版本：

```json
{
  "project_version": {
    "models": {"version": "1.4.2", "tag": "v1.4.2"},
    "infra": {"version": "0.1.7", "tag": "v0.1.7"}
  }
}
```

未初始化的组在状态中不存在（直到执行过至少一次 `update`）。

## 远端覆盖参数

```bash
wproj engine status \
  --work-root /path/to/project \
  --admin-url https://127.0.0.1:19090 \
  --token-file /path/to/admin_api.token \
  --insecure
```

## 相关文档

- 远程工程拉取与规则热更新 SOP: [project-sync.md](project-sync.md)
- 对应英文版: [../../en/operations/admin.md](../../en/operations/admin.md)
