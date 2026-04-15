# Warp Parse Admin API 接口开发文档

- 状态: Draft
- 适用对象: 管理面调用方、CLI 开发者、SDK/控制面集成方
- 当前版本范围: `GET /admin/v1/runtime/status`、`POST /admin/v1/reloads/model`

## 文档目标

本文是面向开发者的独立接口文档，重点说明：

- HTTP 路由与鉴权协议
- 请求头、请求体、响应体和状态码
- `reload + update/version` 的接口语义
- 并发冲突、超时和错误返回约定

使用者/运维操作说明见：

- [../../use/zh/operations/admin.md](../../use/zh/operations/admin.md)

远端规则同步的整体设计见：

- [project_remote_sync_design.md](project_remote_sync_design.md)

## 总览

当前运行时管理面只暴露两个接口：

- `GET /admin/v1/runtime/status`
- `POST /admin/v1/reloads/model`

当前没有独立的 runtime restart 接口。

`POST /admin/v1/reloads/model` 支持两种模式：

- 仅 reload 当前工作目录
- 先更新工程内容，再 reload

## 基础约定

### Base URL

由 `conf/wparse.toml` 中的 `[admin_api]` 决定，例如：

```toml
[admin_api]
enabled = true
bind = "127.0.0.1:19090"
```

则默认 base URL 为：

```text
http://127.0.0.1:19090
```

如果启用了 TLS，则改为 `https://`。

### Content-Type

- 请求体：`application/json`
- 响应体：`application/json`

### 鉴权

所有接口都要求 Bearer Token：

```http
Authorization: Bearer <token>
```

如果 token 缺失、格式错误或值不匹配，返回：

- HTTP `401 Unauthorized`
- `result = "unauthorized"`

### Request ID

服务端会从以下请求头读取 request id：

```http
X-Request-Id: <custom-id>
```

规则：

- 如果请求头存在且非空，则原样使用
- 如果缺失或为空，服务端自动生成 UUID
- 返回体中的 `request_id` 永远存在

### Body 大小限制

请求体上限由 `admin_api.max_body_bytes` 控制，默认 `4096` 字节。

超限时返回：

- HTTP `413 Payload Too Large`
- `result = "payload_too_large"`

## 通用错误响应

错误响应结构：

```json
{
  "request_id": "manual-http-reload-001",
  "accepted": false,
  "result": "invalid_request",
  "error": "version requires update=true"
}
```

字段说明：

- `request_id`: 请求 ID
- `accepted`: 是否被接受执行
- `result`: 错误或状态代码
- `error`: 人类可读错误信息

## 接口 1: 查询运行时状态

### 请求

```http
GET /admin/v1/runtime/status
Authorization: Bearer <token>
```

请求体：无

### 成功响应

- HTTP `200 OK`

响应体：

```json
{
  "instance_id": "host-a:12345",
  "version": "0.21.0",
  "project_version": "1.4.3",
  "accepting_commands": true,
  "reloading": false,
  "current_request_id": null,
  "last_reload_request_id": "reload-20260324-001",
  "last_reload_result": "reload_done",
  "last_reload_started_at": "2026-03-24T01:23:45Z",
  "last_reload_finished_at": "2026-03-24T01:23:47Z"
}
```

字段说明：

- `instance_id`: 实例标识，格式通常为 `<hostname>:<pid>`
- `version`: 当前运行二进制版本
- `project_version`: 当前工作目录使用的工程配置版本；未启用远端版本同步或尚未产生状态文件时为 `null`
- `accepting_commands`: 当前是否接受管理命令
- `reloading`: 当前是否正在执行 reload
- `current_request_id`: 当前正在执行的 reload 请求 ID
- `last_reload_request_id`: 最近一次 reload 请求 ID
- `last_reload_result`: 最近一次 reload 结果
- `last_reload_started_at`: 最近一次 reload 开始时间，RFC 3339
- `last_reload_finished_at`: 最近一次 reload 结束时间，RFC 3339

### 可能错误

- `401 unauthorized`
- `404 not_found`

## 接口 2: 触发模型重载

### 请求

```http
POST /admin/v1/reloads/model
Authorization: Bearer <token>
Content-Type: application/json
X-Request-Id: manual-http-reload-001
```

请求体结构：

```json
{
  "wait": true,
  "update": false,
  "version": null,
  "timeout_ms": 15000,
  "reason": "manual http reload"
}
```

字段说明：

- `wait`: 是否等待 reload 完成；默认 `true`
- `update`: 是否先执行工程更新；默认 `false`
- `version`: 更新目标版本；仅在 `update = true` 时生效
- `timeout_ms`: `wait = true` 时的等待超时时间；为空时使用本地 `admin_api.request_timeout_ms`
- `reason`: 日志中的附加说明

### 请求体约束

- `update = false` 时，`version` 不允许传值
- `update = true` 时，`version` 可为空
- `version` 为空时，服务端按默认版本选择规则解析目标版本

默认版本选择规则：

- 首次初始化且配置了 `init_version` 时，优先使用 `init_version`
- 非首次更新时，优先解析远端最新 release tag
- 如果远端没有 release tag，则回退到远端默认分支 `HEAD`

### 仅 reload 示例

```json
{
  "wait": true,
  "reason": "manual http reload"
}
```

### 更新并 reload 示例

```json
{
  "wait": true,
  "update": true,
  "version": "1.4.3",
  "timeout_ms": 15000,
  "reason": "switch release and reload"
}
```

## Reload 响应模型

成功、已接受、冲突和部分失败场景共用同一个响应结构：

```json
{
  "request_id": "manual-http-reload-001",
  "accepted": true,
  "result": "reload_done",
  "update": true,
  "requested_version": "1.4.3",
  "current_version": "1.4.3",
  "resolved_tag": "v1.4.3",
  "force_replaced": false,
  "warning": null,
  "error": null
}
```

字段说明：

- `request_id`: 请求 ID
- `accepted`: 是否被运行时接受
- `result`: 当前结果代码
- `update`: 本次请求是否启用了更新
- `requested_version`: 显式请求的版本；自动模式下为空
- `current_version`: 本次更新后实际选中的版本
- `resolved_tag`: 最终解析到的远端目标
- `force_replaced`: 是否在优雅 drain 超时后走了兜底替换
- `warning`: 告警或回滚警告
- `error`: 错误详情

版本字段语义：

- 正常 release tag 场景：`resolved_tag` 形如 `v1.4.3`
- 无 tag 回退默认分支场景：`resolved_tag` 形如 `HEAD@main`
- 无 tag 回退时：`current_version` 记录分支名，如 `main` 或 `master`

## 结果代码与状态码

### 1. 同步等待完成且成功

当 `wait = true` 且 reload 在等待窗口内完成：

- HTTP `200 OK`
- `result = "reload_done"`

示例：

```json
{
  "request_id": "reload-001",
  "accepted": true,
  "result": "reload_done",
  "update": false,
  "force_replaced": false
}
```

### 2. 异步接受或等待超时但仍在执行

以下两种情况会返回相同语义：

- `wait = false`
- `wait = true`，但等待超时，reload 仍未结束

返回：

- HTTP `202 Accepted`
- `result = "running"`

示例：

```json
{
  "request_id": "reload-002",
  "accepted": true,
  "result": "running",
  "update": true,
  "requested_version": "1.4.3",
  "current_version": "1.4.3",
  "resolved_tag": "v1.4.3"
}
```

### 3. 优雅 drain 超时但兜底替换成功

返回：

- HTTP `200 OK`
- `result = "reload_done"`
- `force_replaced = true`
- `warning = "graceful drain timed out, fallback to force replace"`

### 4. 请求非法

典型场景：

- `version` 与 `update = false` 组合
- JSON 非法
- body 读取失败

返回：

- HTTP `400 Bad Request`
- `result = "invalid_request"`

示例：

```json
{
  "request_id": "reload-003",
  "accepted": false,
  "result": "invalid_request",
  "error": "version requires update=true"
}
```

### 5. body 超限

返回：

- HTTP `413 Payload Too Large`
- `result = "payload_too_large"`

### 6. 运行时尚未准备好

典型场景：

- runtime command receiver 尚未 ready

返回：

- HTTP `503 Service Unavailable`
- `result = "runtime_not_ready"`

### 7. runtime 通道不可用

典型场景：

- runtime command channel 已关闭

返回：

- HTTP `503 Service Unavailable`
- `result = "runtime_unavailable"`

### 8. reload 冲突

典型场景：

- 当前已有其他 reload 在执行

返回：

- HTTP `409 Conflict`
- `result = "reload_in_progress"`

示例：

```json
{
  "request_id": "reload-004",
  "accepted": false,
  "result": "reload_in_progress"
}
```

### 9. 更新冲突

典型场景：

- 当前已有其他工程更新占用了 project remote 锁

返回：

- HTTP `409 Conflict`
- `result = "update_in_progress"`

示例：

```json
{
  "request_id": "reload-005",
  "accepted": false,
  "result": "update_in_progress",
  "update": true,
  "requested_version": "1.4.3",
  "error": "project remote update already in progress"
}
```

### 10. 更新阶段失败

典型场景：

- 远端同步失败
- 更新前快照捕获失败
- runtime artifact 快照捕获失败

返回：

- HTTP `500 Internal Server Error`
- `result = "update_failed"`

### 11. reload 阶段失败

典型场景：

- `LoadModel` 执行失败
- runtime response channel 被关闭

返回：

- HTTP `500 Internal Server Error`
- `result = "reload_failed"`

说明：

- 如果本次请求带 `update = true`，响应仍可能带回 `current_version` / `resolved_tag`
- 因为它们表示“本次尝试切换到了哪个版本”
- 若回滚也失败，`warning` 会带出回滚失败信息

## 并发与锁语义

服务端同时维护两层互斥：

- reload gate：防止并发 reload
- project remote lock：防止远端更新与 reload/update 交叉执行

含义：

- 两个 reload 请求不能并发执行
- `update + reload` 与 `wproj conf update` 不能并发执行
- 普通 `reload` 也会先尝试获取 project remote lock，以避免与更新过程交叉

## 回滚语义

当 `update = true` 时，服务端会先捕获两类快照：

- 工程目录受管内容快照
- runtime 运行态产物快照

如果后续 reload 失败，会尝试回滚：

1. 还原工程目录
2. 还原运行态产物

如果回滚成功：

- 响应中的 `warning` 通常为空

如果回滚失败：

- `warning` 会包含 `project rollback failed: ...`

## 推荐联调顺序

建议按以下顺序联调：

1. `GET /admin/v1/runtime/status`
2. `POST /admin/v1/reloads/model`，`update = false`
3. `POST /admin/v1/reloads/model`，`update = true`
4. 覆盖 `version` 非法组合、并发冲突、等待超时场景

## 当前已覆盖的关键测试

当前自动化测试已覆盖：

- Bearer token 校验
- token 文件权限检查
- 非回环地址绑定未启用 TLS 的拒绝路径
- `version` 缺少 `update` 的非法请求
- 同步 reload
- 异步 reload
- reload 并发冲突
- update 锁冲突
- drain 超时后的 force replace

## 对应英文版

- [../en/admin_api_development.md](../en/admin_api_development.md)
