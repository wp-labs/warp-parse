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
cargo run --bin wparse -- daemon --work-root .
```

启动后可访问：

- `GET /admin/v1/runtime/status`
- `POST /admin/v1/reloads/model`

## 查询运行时状态

文本输出：

```bash
cargo run --bin wproj -- engine status --work-root .
```

JSON 输出：

```bash
cargo run --bin wproj -- engine status --work-root . --json
```

关键字段：

- `instance_id`: 实例标识
- `version`: 当前二进制版本
- `project_version`: 当前工作目录使用的工程配置版本；没有远端状态时为空
- `accepting_commands`: 是否接受管理命令
- `reloading`: 当前是否处于重载中
- `current_request_id`: 当前执行中的重载请求 ID
- `last_reload_request_id`: 最近一次重载请求 ID
- `last_reload_result`: 最近一次重载结果

## 触发重载

### CLI

等待完成：

```bash
cargo run --bin wproj -- engine reload \
  --work-root . \
  --request-id manual-reload-001 \
  --reason "manual model refresh"
```

异步返回：

```bash
cargo run --bin wproj -- engine reload \
  --work-root . \
  --wait false \
  --request-id manual-reload-async-001 \
  --reason "async refresh"
```

先更新远端工程，再执行 reload：

```bash
cargo run --bin wproj -- engine reload \
  --work-root . \
  --update \
  --request-id update-reload-001 \
  --reason "rule update and reload"
```

显式切换到某个版本再 reload：

```bash
cargo run --bin wproj -- engine reload \
  --work-root . \
  --update \
  --version 1.4.3 \
  --request-id update-reload-002 \
  --reason "switch release and reload"
```

JSON 输出：

```bash
cargo run --bin wproj -- engine reload \
  --work-root . \
  --update \
  --request-id manual-reload-json-001 \
  --reason "json output" \
  --json
```

CLI 约束：

- `--version` 必须与 `--update` 一起使用
- 不带 `--update` 时，只对当前工作目录做 reload
- 带 `--update` 时，会在 reload 前执行一次与 `wproj conf update` 等价的同步、校验和失败回滚

### HTTP 请求体

请求体字段：

- `wait`: 是否等待 reload 完成后再返回；默认 `true`
- `update`: 是否先更新工程内容；默认 `false`
- `version`: 目标版本；仅在 `update = true` 时生效
- `timeout_ms`: `wait = true` 时的等待超时时间；为空时使用本地配置的 `admin_api.request_timeout_ms`
- `reason`: 日志中的附加原因说明

只 reload：

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

先更新再 reload：

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

接口约束：

- `update = false` 时，`version` 不能传值
- `update = true` 且 `version` 为空时，按默认版本选择规则解析目标版本
- 默认版本选择规则与 `wproj conf update` 一致

默认版本选择规则：

- 首次初始化且配置了 `init_version` 时，优先使用 `init_version`
- 非首次更新时，优先解析远端最新 release tag
- 如果远端没有 release tag，则回退到远端默认分支 `HEAD`

## 响应字段

`POST /admin/v1/reloads/model` 成功或已接受时，常见返回字段包括：

- `request_id`: 请求 ID
- `accepted`: 是否被接受
- `result`: 当前结果
- `update`: 本次请求是否带更新
- `requested_version`: 本次显式指定的版本；自动模式下为空
- `current_version`: 本次更新后实际落地的版本
- `resolved_tag`: 本次更新最终解析到的远端目标
- `force_replaced`: 优雅 drain 超时后是否走兜底替换
- `warning`: 告警信息
- `error`: 错误详情

字段语义：

- 如果远端按 release tag 发布，`resolved_tag` 形如 `v1.4.3`
- 如果远端没有 release tag 并回退到默认分支，`resolved_tag` 形如 `HEAD@main`
- 此时 `current_version` 会记录分支名，如 `main` 或 `master`

常见结果：

- `reload_done`: 重载成功完成
- `running`: 请求已接收但仍在执行
- `reload_in_progress`: 已有其他重载在进行中
- `update_in_progress`: 已有其他工程更新在进行中
- `update_failed`: 更新阶段失败
- `reload_failed`: reload 阶段失败

如果优雅 drain 超时，返回中可能包含：

- `force_replaced = true`
- `warning = "graceful drain timed out, fallback to force replace"`

## 远端覆盖参数

当 `wproj` 不在目标工作目录执行时，可以覆盖目标：

```bash
cargo run --bin wproj -- engine status \
  --work-root /path/to/project \
  --admin-url https://127.0.0.1:19090 \
  --token-file /path/to/admin_api.token \
  --insecure
```

说明：

- `--admin-url` 覆盖管理面地址
- `--token-file` 覆盖 token 文件位置
- `--insecure` 仅用于调试时跳过 TLS 证书校验

## 已验证覆盖

当前自动化测试已覆盖：

- 正确鉴权下的状态查询
- 错误 token 的拒绝路径
- 同步 reload
- 异步 reload
- 带 `update/version` 的 reload
- `version` 缺少 `update` 时的拒绝路径
- 并发 reload 冲突
- 更新与 reload 互斥冲突
- drain 超时后的 force replace 回退
- `wproj engine status` 与 `wproj engine reload` 联调
- batch 模式不暴露 HTTP 服务

## 对应英文版

- [../en/engine_admin_usage.md](../en/engine_admin_usage.md)
