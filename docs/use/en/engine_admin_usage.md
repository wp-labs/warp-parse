# Warp Parse Runtime Admin Usage

## Scope

The currently available runtime admin capability includes only:

- an authenticated HTTP admin API exposed by `wparse daemon`
- `wproj engine status` for runtime status queries
- `wproj engine reload` for model reload requests

Batch mode does not expose the admin HTTP service.

## Enable The Admin API

Configure `conf/wparse.toml`:

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

Create the token file before starting the daemon:

```bash
mkdir -p runtime
printf 'replace-with-a-secret-token\n' > runtime/admin_api.token
chmod 600 runtime/admin_api.token
```

Constraints:

- on Unix, the token file must be owner-only
- non-loopback bind addresses require TLS
- the only supported auth mode is `bearer_token`

## Start The Daemon

```bash
cargo run --bin wparse -- daemon --work-root .
```

The daemon exposes:

- `GET /admin/v1/runtime/status`
- `POST /admin/v1/reloads/model`

## Query Runtime Status

Text output:

```bash
cargo run --bin wproj -- engine status --work-root .
```

JSON output:

```bash
cargo run --bin wproj -- engine status --work-root . --json
```

Important fields:

- `instance_id`: runtime instance identifier
- `version`: current version
- `accepting_commands`: whether admin commands are accepted
- `reloading`: whether a reload is in progress
- `current_request_id`: active reload request ID
- `last_reload_request_id`: most recent reload request ID
- `last_reload_result`: most recent reload result

## Trigger Reload

Wait for completion:

```bash
cargo run --bin wproj -- engine reload \
  --work-root . \
  --request-id manual-reload-001 \
  --reason "manual model refresh"
```

Return immediately:

```bash
cargo run --bin wproj -- engine reload \
  --work-root . \
  --wait false \
  --request-id manual-reload-async-001 \
  --reason "async refresh"
```

JSON output:

```bash
cargo run --bin wproj -- engine reload \
  --work-root . \
  --request-id manual-reload-json-001 \
  --reason "json output" \
  --json
```

Common results:

- `reload_done`: reload completed successfully
- `running`: the request was accepted and is still running
- `reload_in_progress`: another reload is already active

If graceful drain times out, the response can still succeed with:

- `force_replaced = true`
- `warning = "graceful drain timed out, fallback to force replace"`

## Remote Override

If `wproj` is not executed inside the target work directory, override the target explicitly:

```bash
cargo run --bin wproj -- engine status \
  --work-root /path/to/project \
  --admin-url https://127.0.0.1:19090 \
  --token-file /path/to/admin_api.token \
  --insecure
```

Notes:

- `--admin-url` overrides the admin API base URL
- `--token-file` overrides the token file path
- `--insecure` skips TLS certificate validation for debugging only

## Direct HTTP Usage

Query status:

```bash
curl -sS \
  -H 'Authorization: Bearer replace-with-a-secret-token' \
  http://127.0.0.1:19090/admin/v1/runtime/status
```

Trigger reload:

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

## Verified Coverage

Current automated coverage verifies:

- status queries with valid authentication
- rejection for invalid bearer tokens
- synchronous reload
- asynchronous reload
- conflict behavior for concurrent reloads
- force-replace fallback after drain timeout
- `wproj engine status` and `wproj engine reload` against a live daemon
- absence of the admin HTTP service in batch mode

## Chinese Counterpart

- [../zh-CN/engine_admin_usage.md](../zh-CN/engine_admin_usage.md)
