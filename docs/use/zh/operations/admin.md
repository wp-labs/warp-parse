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
