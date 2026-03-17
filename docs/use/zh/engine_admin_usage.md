# Warp Parse 运行时管理面使用说明

## 范围

当前已提供的运行时管理能力仅包含：

- `wparse daemon` 暴露受鉴权保护的 HTTP 管理面
- `wproj engine status` 查询运行时状态
- `wproj engine reload` 触发模型重载

`batch` 模式不会暴露管理面 HTTP 服务。

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
token_file = "runtime/admin_api.token"
```

启动前创建 token 文件：

```bash
mkdir -p runtime
printf 'replace-with-a-secret-token\n' > runtime/admin_api.token
chmod 600 runtime/admin_api.token
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
- `version`: 当前版本
- `accepting_commands`: 是否接受管理命令
- `reloading`: 当前是否处于重载中
- `current_request_id`: 当前执行中的重载请求 ID
- `last_reload_request_id`: 最近一次重载请求 ID
- `last_reload_result`: 最近一次重载结果

## 触发重载

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

JSON 输出：

```bash
cargo run --bin wproj -- engine reload \
  --work-root . \
  --request-id manual-reload-json-001 \
  --reason "json output" \
  --json
```

常见结果：

- `reload_done`: 重载成功完成
- `running`: 请求已接收但仍在执行
- `reload_in_progress`: 已有其他重载在进行中

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

## 直接 HTTP 调用

查询状态：

```bash
curl -sS \
  -H 'Authorization: Bearer replace-with-a-secret-token' \
  http://127.0.0.1:19090/admin/v1/runtime/status
```

触发重载：

```bash
curl -sS \
  -X POST \
  -H 'Authorization: Bearer replace-with-a-secret-token' \
  -H 'Content-Type: application/json' \
  -H 'X-Request-Id: manual-http-reload-001' \
  http://127.0.0.1:19090/admin/v1/reloads/model \
  -d '{
    "wait": true,
    "timeout_ms": 15000,
    "reason": "manual http reload"
  }'
```

## 已验证覆盖

当前自动化测试已覆盖：

- 正确鉴权下的状态查询
- 错误 token 的拒绝路径
- 同步重载
- 异步重载
- 并发重载冲突
- drain 超时后的 force replace 回退
- `wproj engine status` 与 `wproj engine reload` 联调
- batch 模式不暴露 HTTP 服务

## 对应英文版

- [../en/engine_admin_usage.md](../en/engine_admin_usage.md)
