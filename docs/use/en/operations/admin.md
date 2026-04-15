# Warp Parse Runtime Admin Usage

## Scope

The currently available runtime admin capability includes only:

- an authenticated HTTP admin API exposed by `wparse daemon`
- `wproj engine status` for runtime status queries
- `wproj engine reload` for `LoadModel` reload requests

Batch mode does not expose the admin HTTP service.

There is no separate runtime restart API at the moment. Remote rule updates are wired only into reload.

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
token_file = "${HOME}/.warp_parse/admin_api.token"
```

Create the token file before starting the daemon:

```bash
mkdir -p runtime
mkdir -p "${HOME}/.warp_parse"
printf 'replace-with-a-secret-token\n' > "${HOME}/.warp_parse/admin_api.token"
chmod 600 "${HOME}/.warp_parse/admin_api.token"
```

Constraints:

- on Unix, the token file must be owner-only
- non-loopback bind addresses require TLS
- the only supported auth mode is `bearer_token`

## Start The Daemon

```bash
wparse daemon --work-root .
```

The daemon exposes:

- `GET /admin/v1/runtime/status`
- `POST /admin/v1/reloads/model`

## Query Runtime Status

Text output:

```bash
wproj engine status --work-root .
```

JSON output:

```bash
wproj engine status --work-root . --json
```

Important fields:

- `instance_id`: runtime instance identifier
- `version`: current binary version
- `project_version`: project configuration version currently active in the work tree; empty when no remote state exists
- `accepting_commands`: whether admin commands are accepted
- `reloading`: whether a reload is in progress
- `current_request_id`: active reload request ID
- `last_reload_request_id`: most recent reload request ID
- `last_reload_result`: most recent reload result

## Trigger Reload

### CLI

Wait for completion:

```bash
wproj engine reload \
  --work-root . \
  --request-id manual-reload-001 \
  --reason "manual model refresh"
```

Return immediately:

```bash
wproj engine reload \
  --work-root . \
  --wait false \
  --request-id manual-reload-async-001 \
  --reason "async refresh"
```

Update the remote project first, then reload:

```bash
wproj engine reload \
  --work-root . \
  --update \
  --request-id update-reload-001 \
  --reason "rule update and reload"
```

Switch to a specific release version and reload:

```bash
wproj engine reload \
  --work-root . \
  --update \
  --version 1.4.3 \
  --request-id update-reload-002 \
  --reason "switch release and reload"
```

JSON output:

```bash
wproj engine reload \
  --work-root . \
  --update \
  --request-id manual-reload-json-001 \
  --reason "json output" \
  --json
```

CLI rules:

- `--version` must be used together with `--update`
- without `--update`, reload only applies to the current local work tree
- with `--update`, the runtime performs the equivalent of `wproj conf update` before reload, including validation and rollback-on-failure

### HTTP Request Body

Request fields:

- `wait`: whether to wait for reload completion before returning; default `true`
- `update`: whether to update the project content first; default `false`
- `version`: target version; only valid when `update = true`
- `timeout_ms`: wait timeout when `wait = true`; if omitted, the server uses local `admin_api.request_timeout_ms`
- `reason`: extra reason string for logs

Reload only:

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

Update first, then reload:

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

API rules:

- when `update = false`, `version` must be absent
- when `update = true` and `version` is empty, the server resolves the target by the default version-selection rule
- the default version-selection rule is the same as `wproj conf update`

Default version-selection rules:

- on first initialization, use `init_version` first when configured
- on later updates, prefer the latest release tag
- if the remote has no release tag, fall back to the remote default branch `HEAD`

## Response Fields

For accepted or completed `POST /admin/v1/reloads/model` calls, common fields include:

- `request_id`: request identifier
- `accepted`: whether the request was accepted
- `result`: current result code
- `update`: whether this request included project update
- `requested_version`: explicit requested version; empty in auto mode
- `current_version`: the version actually activated by this update
- `resolved_tag`: the resolved remote target
- `force_replaced`: whether graceful drain timed out and forced replacement was used
- `warning`: warning detail
- `error`: error detail

Field semantics:

- if the remote publishes release tags, `resolved_tag` looks like `v1.4.3`
- if the remote has no release tags and falls back to default-branch sync, `resolved_tag` looks like `HEAD@main`
- in that case, `current_version` records the branch name such as `main` or `master`

Common results:

- `reload_done`: reload completed successfully
- `running`: request accepted and still executing
- `reload_in_progress`: another reload is already active
- `update_in_progress`: another project update is already active
- `update_failed`: update stage failed
- `reload_failed`: reload stage failed

If graceful drain times out, the response can still succeed with:

- `force_replaced = true`
- `warning = "graceful drain timed out, fallback to force replace"`

## Remote Override

If `wproj` is not executed inside the target work directory, override the target explicitly:

```bash
wproj engine status \
  --work-root /path/to/project \
  --admin-url https://127.0.0.1:19090 \
  --token-file /path/to/admin_api.token \
  --insecure
```

Notes:

- `--admin-url` overrides the admin API base URL
- `--token-file` overrides the token file path
- `--insecure` skips TLS certificate validation for debugging only

## Verified Coverage

Current automated coverage verifies:

- status queries with valid authentication
- rejection for invalid bearer tokens
- synchronous reload
- asynchronous reload
- reload with `update/version`
- rejection when `version` is provided without `update`
- conflict behavior for concurrent reloads
- conflict behavior between update and reload
- force-replace fallback after drain timeout
- `wproj engine status` and `wproj engine reload` against a live daemon
- absence of the admin HTTP service in batch mode

## Chinese Counterpart

- [../../zh/operations/admin.md](../../zh/operations/admin.md)
