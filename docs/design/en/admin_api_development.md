# Warp Parse Admin API Development Guide

- Status: Draft
- Audience: admin-plane callers, CLI developers, SDK/control-plane integrators
- Current API scope: `GET /admin/v1/runtime/status`, `POST /admin/v1/reloads/model`

## Goal

This is a developer-facing standalone API document focused on:

- HTTP routes and authentication protocol
- headers, request bodies, response bodies, and status codes
- the API semantics of `reload + update/version`
- conflict, timeout, and error behavior

Operator-facing usage is documented in:

- [../../use/en/operations/admin.md](../../use/en/operations/admin.md)

Overall remote-sync design is documented in:

- [project_remote_sync_design.md](project_remote_sync_design.md)

## Overview

The runtime admin API currently exposes only two endpoints:

- `GET /admin/v1/runtime/status`
- `POST /admin/v1/reloads/model`

There is no separate runtime restart API at the moment.

`POST /admin/v1/reloads/model` supports two modes:

- reload the current working tree only
- update project content first, then reload

## Base Conventions

### Base URL

The base URL is determined by `[admin_api]` in `conf/wparse.toml`, for example:

```toml
[admin_api]
enabled = true
bind = "127.0.0.1:19090"
```

The default base URL is then:

```text
http://127.0.0.1:19090
```

If TLS is enabled, use `https://`.

### Content-Type

- request body: `application/json`
- response body: `application/json`

### Authentication

All endpoints require a Bearer token:

```http
Authorization: Bearer <token>
```

If the token is missing, malformed, or mismatched, the server returns:

- HTTP `401 Unauthorized`
- `result = "unauthorized"`

### Request ID

The server reads request id from:

```http
X-Request-Id: <custom-id>
```

Rules:

- if present and non-empty, the value is used as-is
- if missing or empty, the server generates a UUID
- `request_id` is always present in the response

### Body Size Limit

The request-body limit is controlled by `admin_api.max_body_bytes`, default `4096`.

If exceeded, the server returns:

- HTTP `413 Payload Too Large`
- `result = "payload_too_large"`

## Common Error Response

Error response shape:

```json
{
  "request_id": "manual-http-reload-001",
  "accepted": false,
  "result": "invalid_request",
  "error": "version requires update=true"
}
```

Field notes:

- `request_id`: request identifier
- `accepted`: whether the request was accepted for execution
- `result`: error or state code
- `error`: human-readable detail

## Endpoint 1: Query Runtime Status

### Request

```http
GET /admin/v1/runtime/status
Authorization: Bearer <token>
```

Request body: none

### Success Response

- HTTP `200 OK`

Response body:

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

Field notes:

- `instance_id`: runtime instance identifier, typically `<hostname>:<pid>`
- `version`: current binary version
- `project_version`: project configuration version currently active in the work tree; `null` when remote project state is not available
- `accepting_commands`: whether admin commands are currently accepted
- `reloading`: whether a reload is in progress
- `current_request_id`: request id of the active reload
- `last_reload_request_id`: request id of the most recent reload
- `last_reload_result`: result code of the most recent reload
- `last_reload_started_at`: RFC 3339 start timestamp of the most recent reload
- `last_reload_finished_at`: RFC 3339 finish timestamp of the most recent reload

### Possible Errors

- `401 unauthorized`
- `404 not_found`

## Endpoint 2: Trigger Model Reload

### Request

```http
POST /admin/v1/reloads/model
Authorization: Bearer <token>
Content-Type: application/json
X-Request-Id: manual-http-reload-001
```

Request shape:

```json
{
  "wait": true,
  "update": false,
  "version": null,
  "timeout_ms": 15000,
  "reason": "manual http reload"
}
```

Field notes:

- `wait`: whether the server should wait for reload completion; default `true`
- `update`: whether project content should be updated first; default `false`
- `version`: target version for update; only meaningful when `update = true`
- `timeout_ms`: wait timeout when `wait = true`; if omitted, the server uses local `admin_api.request_timeout_ms`
- `reason`: extra log annotation

### Request Constraints

- when `update = false`, `version` must not be provided
- when `update = true`, `version` may be empty
- when `version` is empty, the server resolves the target by the default version-selection rule

Default version-selection rules:

- on first initialization, use `init_version` first when configured
- on later updates, prefer the latest release tag
- if the remote has no release tag, fall back to the remote default branch `HEAD`

### Reload-Only Example

```json
{
  "wait": true,
  "reason": "manual http reload"
}
```

### Update-And-Reload Example

```json
{
  "wait": true,
  "update": true,
  "version": "1.4.3",
  "timeout_ms": 15000,
  "reason": "switch release and reload"
}
```

## Reload Response Model

Successful, accepted, conflict, and partial-failure reload flows share one response shape:

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

Field notes:

- `request_id`: request identifier
- `accepted`: whether the runtime accepted the request
- `result`: current result code
- `update`: whether project update was enabled for this request
- `requested_version`: explicitly requested version; empty in auto mode
- `current_version`: version actually selected by this update
- `resolved_tag`: resolved remote target
- `force_replaced`: whether graceful drain timed out and force replace was used
- `warning`: warning or rollback warning
- `error`: error detail

Version-field semantics:

- normal release-tag case: `resolved_tag` looks like `v1.4.3`
- no-tag fallback case: `resolved_tag` looks like `HEAD@main`
- in no-tag fallback, `current_version` records the branch name such as `main` or `master`

## Result Codes And Status Codes

### 1. Waited And Completed Successfully

When `wait = true` and reload completes within the wait window:

- HTTP `200 OK`
- `result = "reload_done"`

Example:

```json
{
  "request_id": "reload-001",
  "accepted": true,
  "result": "reload_done",
  "update": false,
  "force_replaced": false
}
```

### 2. Accepted Asynchronously Or Still Running After Wait Timeout

The following two cases share the same semantics:

- `wait = false`
- `wait = true`, but the wait timeout is reached while reload is still running

Return:

- HTTP `202 Accepted`
- `result = "running"`

Example:

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

### 3. Graceful Drain Timed Out But Force Replace Succeeded

Return:

- HTTP `200 OK`
- `result = "reload_done"`
- `force_replaced = true`
- `warning = "graceful drain timed out, fallback to force replace"`

### 4. Invalid Request

Typical cases:

- `version` combined with `update = false`
- invalid JSON
- request body read failure

Return:

- HTTP `400 Bad Request`
- `result = "invalid_request"`

Example:

```json
{
  "request_id": "reload-003",
  "accepted": false,
  "result": "invalid_request",
  "error": "version requires update=true"
}
```

### 5. Body Too Large

Return:

- HTTP `413 Payload Too Large`
- `result = "payload_too_large"`

### 6. Runtime Not Ready

Typical case:

- runtime command receiver is not ready yet

Return:

- HTTP `503 Service Unavailable`
- `result = "runtime_not_ready"`

### 7. Runtime Channel Unavailable

Typical case:

- runtime command channel has been closed

Return:

- HTTP `503 Service Unavailable`
- `result = "runtime_unavailable"`

### 8. Reload Conflict

Typical case:

- another reload is already running

Return:

- HTTP `409 Conflict`
- `result = "reload_in_progress"`

Example:

```json
{
  "request_id": "reload-004",
  "accepted": false,
  "result": "reload_in_progress"
}
```

### 9. Update Conflict

Typical case:

- another project update currently holds the project-remote lock

Return:

- HTTP `409 Conflict`
- `result = "update_in_progress"`

Example:

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

### 10. Update Stage Failed

Typical cases:

- remote sync failed
- pre-update project snapshot capture failed
- runtime artifact snapshot capture failed

Return:

- HTTP `500 Internal Server Error`
- `result = "update_failed"`

### 11. Reload Stage Failed

Typical cases:

- `LoadModel` execution failed
- runtime response channel closed

Return:

- HTTP `500 Internal Server Error`
- `result = "reload_failed"`

Notes:

- if `update = true`, the response may still include `current_version` and `resolved_tag`
- those fields indicate which version the failed attempt had switched to
- if rollback also fails, `warning` carries the rollback-failure detail

## Concurrency And Locking

The server maintains two layers of mutual exclusion:

- reload gate: prevents concurrent reload execution
- project remote lock: prevents overlap between project update and reload/update flows

Implications:

- two reload requests cannot run concurrently
- `update + reload` cannot overlap with `wproj conf update`
- even a plain reload attempts to acquire the project-remote lock first, so it does not interleave with updates

## Rollback Semantics

When `update = true`, the server captures two kinds of snapshots first:

- managed project-content snapshot
- runtime artifact snapshot

If reload later fails, the server attempts rollback in this order:

1. restore project content
2. restore runtime artifacts

If rollback succeeds:

- `warning` is typically empty

If rollback fails:

- `warning` contains `project rollback failed: ...`

## Recommended Integration Order

Recommended integration order:

1. `GET /admin/v1/runtime/status`
2. `POST /admin/v1/reloads/model` with `update = false`
3. `POST /admin/v1/reloads/model` with `update = true`
4. cover invalid version combinations, conflict paths, and wait-timeout behavior

## Key Automated Coverage

Current automated coverage includes:

- Bearer token validation
- token-file permission checks
- rejection of non-loopback bind without TLS
- invalid request path where `version` is provided without `update`
- synchronous reload
- asynchronous reload
- reload conflict
- update-lock conflict
- force-replace fallback after graceful-drain timeout

## Chinese Counterpart

- [../zh/admin_api_development.md](../zh/admin_api_development.md)
