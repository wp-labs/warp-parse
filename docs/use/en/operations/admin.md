# Warp Parse Runtime Admin Usage

## Scope

The currently available runtime admin capability includes only:

- an authenticated HTTP admin API exposed by `wparse daemon`
- `wproj engine status` for runtime status queries
- `wproj engine reload` for `LoadModel` reload requests

Batch mode does not expose the admin HTTP service.

There is no separate runtime restart API at the moment. Remote rule updates are wired only into reload.

## Enable The Admin API

Configure `wparse.local.toml` in the project root:

> `wparse.local.toml` is the site-local overlay and lives outside `conf/`.
> `project_remote` only replaces `conf/`, `models/`, `topology/` and
> `connectors/`, so a remote publish cannot overwrite the bind address, TLS or
> token path configured here.
>
> Backward compatibility: when `wparse.local.toml` is absent, `[admin_api]` in
> `conf/wparse.toml` is still honored with a warning. `wproj init` and
> `wproj conf update` migrate it automatically. If the file exists but has no
> `[admin_api]` section (or is invalid TOML), startup fails instead of falling
> back silently.
>
> Add `wparse.local.toml` to the project `.gitignore` so it is not committed to
> the publish repository by accident.

Example (`wparse.local.toml`):

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

Create the token file before starting the daemon:

```bash
wpadm token generate --out "${HOME}/.warp_parse/admin_api.token"
```

`wpadm token generate` writes 32 random bytes (64 hex characters), creates the
file atomically with mode `0600`, and prints only the path. It refuses to
overwrite an existing file unless `--force` is passed, and it never follows a
symlink at the output path.

Constraints:

- the token file must be a regular file that group/other cannot write
  (`0600`, `0640`, `0440` and `0444` are accepted; `0660`, `0664`, `0777` are
  rejected) — this allows read-only Docker/Kubernetes secret mounts
- symlinks are followed and the resolved target is validated, so Kubernetes
  projected secrets (`token -> ..data/token`) work
- the file must contain exactly one non-empty line
- non-loopback bind addresses require TLS
- Bearer token is the only auth scheme; there is no auth-mode setting

The engine reads the token once at startup. Rotating a token therefore requires
replacing the file and restarting the daemon.

## Token Tools

```bash
# Generate a token (prints only the path)
wpadm token generate --out /run/secrets/wparse_admin_token

# Inspect a token file: type, symlink target, mode, owner, content, fingerprint
wpadm token check --file /run/secrets/wparse_admin_token

# Stricter check for host files / CI
wpadm token check --file "${HOME}/.warp_parse/admin_api.token" --require-owner-only

# Print the effective token (value on stdout, diagnostics on stderr)
wpadm token show --token-file /run/secrets/wparse_admin_token
```

`wpadm` resolves the token in this order, which lets it run inside a container or
in CI where the engine's `wparse.local.toml` is not available:

1. `--token-file <path>`
2. `--token-env <NAME>`
3. `$WPARSE_ADMIN_TOKEN`
4. `token_file` from the local admin API profile

Every diagnostic prints an 8-character `token_fp` (`sha256(token)` prefix), never
the token itself. Compare the engine's startup log line
(`admin api auth source=file path=... token_fp=...`) with the client-side
fingerprint to confirm both sides hold the same token.

For container deployments mount the **directory** rather than a single file
(`-v ./secrets:/run/secrets:ro`); renaming a file over a single-file bind mount
does not propagate into the container.

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
- `project_version`: project configuration version currently active in the work tree
  - Single-repo mode: a string (e.g. `"1.0"`); empty when no remote state exists
  - Dual-repo mode: an object with per-group versions, e.g. `{"models": {"version": "1.4.2", "tag": "v1.4.2"}, "infra": {"version": "0.1.7", "tag": "v0.1.7"}}`; a group only appears after at least one successful update
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

### Dual-Repo Mode

In dual-repo mode (`[project_remote.models]` + `[project_remote.infra]`), every update must specify `--group`:

Update the models group:

```bash
wproj engine reload \
  --work-root . \
  --update \
  --group models \
  --request-id update-models-001 \
  --reason "update models group"
```

Update the infra group to a specific version:

```bash
wproj engine reload \
  --work-root . \
  --update \
  --group infra \
  --version 0.1.7 \
  --request-id update-infra-002 \
  --reason "switch infra to 0.1.7"
```

Constraints:

- In dual-repo mode, `--update` requires `--group`, otherwise an error is returned
- `--group` is only meaningful with `--update`; it is ignored for reload-only requests

### HTTP Request Body

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

Dual-repo — update models group:

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

Dual-repo — update infra group:

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

API rules:

- when `update = false`, `version` must be absent
- when `update = true` and `version` is empty, the server resolves the target by the default version-selection rule
- in dual-repo mode, `update = true` requires `group` (`"models"` or `"infra"`), otherwise an error is returned
- `group` is only meaningful with `update = true`

Default version-selection rules:

- on first initialization, use `init_version` first when configured
- on later updates, prefer the latest release tag
- if the remote has no release tag, fall back to the remote default branch `HEAD`

### Request Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `wait` | bool | no | Wait for reload completion before returning; default `true` |
| `update` | bool | no | Update project content first; default `false` |
| `version` | string | no | Target version; only valid when `update = true` |
| `group` | string | no | Target group: `"models"` or `"infra"`; required in dual-repo mode when `update = true` |
| `timeout_ms` | number | no | Wait timeout when `wait = true`; falls back to server `admin_api.request_timeout_ms` |
| `reason` | string | no | Extra reason string for logs |

## Response Fields

For accepted or completed `POST /admin/v1/reloads/model` calls, common fields include:

- `request_id`: request identifier
- `accepted`: whether the request was accepted
- `result`: current result code
- `update`: whether this request included project update
- `requested_version`: explicit requested version; empty in auto mode
- `current_version`: the version actually activated by this update
- `resolved_tag`: the resolved remote target
- `group`: the target group in dual-repo mode (`"models"` / `"infra"`); omitted in single-repo mode
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
