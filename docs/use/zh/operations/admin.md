# Warp Parse 运行时管理面使用说明

## 范围

当前已提供的运行时管理能力仅包含：

- `wparse daemon` 暴露受鉴权保护的 HTTP 管理面
- `wproj engine status` 查询运行时状态
- `wproj engine reload` 触发 `LoadModel` 重载

`batch` 模式不会暴露管理面 HTTP 服务。

当前没有独立的 runtime restart 接口。远端规则更新只与 `reload` 联动。

## 启用管理面

在 `conf/wparse.toml` 中配置：

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
token_file = "${HOME}/.warp_parse/admin_api.token"
```

启动前创建 token 文件：

```bash
mkdir -p runtime
mkdir -p "${HOME}/.warp_parse"
printf 'replace-with-a-secret-token\n' > "${HOME}/.warp_parse/admin_api.token"
chmod 600 "${HOME}/.warp_parse/admin_api.token"
```

约束：

- Unix 下 token 文件权限必须是 owner-only
- 非回环地址绑定必须启用 TLS
- 当前只支持 `bearer_token` 鉴权模式

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
