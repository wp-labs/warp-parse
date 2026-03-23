# Remote Project Sync And Rule Reload SOP

## Scope

This document covers the operator workflow for:

- initializing a WP project on a remote machine from a remote version repository
- updating the project later with `wproj conf update`
- reloading rules or models without stopping `wparse daemon`

This document does not cover authoring or debugging `parse.wpl`.

## Goal

Treat a rule update as two separate actions:

1. update managed project configuration files
2. send a reload request to the live runtime

That separation keeps repository sync independent from process lifecycle control.

## Prerequisites

The remote machine should have:

- working `wproj` and `wparse` binaries, or the ability to run them with `cargo run --bin ...`
- a fixed work root such as `/srv/wp/<project>`
- a remote repository that already contains a valid WP project layout

Before rollout, confirm:

- the runtime uses `wparse daemon`, not `batch`
- paths in `conf/wparse.toml` are valid relative to the work root
- the machine has the SSH key or token required to access the repository

## Enable The Runtime Admin API

Hot reload depends on the runtime admin API. Configure `conf/wparse.toml` as described in [engine_admin_usage.md](engine_admin_usage.md):

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

Prepare the token file before startup:

```bash
mkdir -p runtime
printf 'replace-with-a-secret-token\n' > runtime/admin_api.token
chmod 600 runtime/admin_api.token
```

Constraints:

- `batch` mode does not expose the admin API
- on Unix, the token file must be owner-only
- non-loopback binds require TLS

## First Deployment

### 1. Initialize From Remote

```bash
wproj init \
  --work-root /srv/wp/<project> \
  --remote https://github.com/wp-labs/editor-monitor-conf.git \
  --version 1.4.2
```

Notes:

- `wproj init --remote` creates the local project skeleton first
- `--remote` / `--version` are used only as bootstrap parameters for first sync
- then it reuses `wproj conf update` for first sync and validation
- after first sync, configuration from the remote repository becomes authoritative
- if `--version` is omitted, it resolves the latest released version from remote

### 2. Validate The Project

```bash
wproj check
wproj data stat
```

If binaries are not on `PATH`, use:

```bash
cargo run --bin wproj -- check
cargo run --bin wproj -- data stat
```

### 3. Start The Daemon

```bash
cargo run --bin wparse -- daemon --work-root .
```

Then verify runtime status:

```bash
cargo run --bin wproj -- engine status --work-root .
```

Important fields:

- `accepting_commands = true`
- `reloading = false`

## Daily Rule Update SOP

### Standard Flow

Move into the target project:

```bash
cd /srv/wp/<project>
```

Update to the default or resolved target version:

```bash
wproj conf update --work-root /srv/wp/<project>
```

To upgrade or roll back to a specific version:

```bash
wproj conf update --work-root /srv/wp/<project> --version 1.4.3
```

Run a minimal gate before reload:

```bash
wproj check --what wpl --fail-fast
```

Inspect current runtime status:

```bash
cargo run --bin wproj -- engine status --work-root .
```

Trigger a synchronous reload:

```bash
cargo run --bin wproj -- engine reload \
  --work-root . \
  --request-id rule-$(date +%Y%m%d%H%M%S) \
  --reason "rule update"
```

Check status again after reload:

```bash
cargo run --bin wproj -- engine status --work-root .
```

### Result Interpretation

Focus on:

- `last_reload_request_id`
- `last_reload_result`
- `reloading`

Common results:

- `reload_done`: reload completed successfully
- `running`: request accepted and still executing
- `reload_in_progress`: another reload is already active

If the response includes the following fields, graceful drain timed out and the runtime fell back to forced replacement:

- `force_replaced = true`
- `warning = "graceful drain timed out, fallback to force replace"`

This is not an immediate failure, but it should trigger extra observation.

## Recommended Release Gate

Use this fixed sequence:

1. `wproj conf update`
2. `wproj check --what wpl --fail-fast`
3. `wproj engine status`
4. `wproj engine reload`
5. `wproj engine status` again
6. inspect `data/logs/`, parse statistics, and sink outputs

If the release includes source, sink, or main config changes, run the full project check:

```bash
wproj check
```

## Rollback SOP

If a reload introduces parse failures, field regressions, or downstream alarms, do not restart the daemon first. Roll back the project version and reload again:

```bash
wproj conf update --work-root /srv/wp/<project> --version 1.4.2
cargo run --bin wproj -- engine reload \
  --work-root /srv/wp/<project> \
  --request-id rollback-$(date +%Y%m%d%H%M%S) \
  --reason "rollback rule set"
```

After rollback, verify:

- `wproj engine status`
- `data/logs/`
- sink output recovery

If you need to return to the latest release later:

```bash
wproj conf update --work-root /srv/wp/<project>
```

## Remote Override

If `wproj` is executed outside the target project, override the target explicitly:

```bash
cargo run --bin wproj -- engine status \
  --work-root /srv/wp/<project> \
  --admin-url http://127.0.0.1:19090 \
  --token-file /srv/wp/<project>/runtime/admin_api.token
```

The same override pattern applies to `engine reload`.

## Acceptance Checklist

A remote rule update is complete only when all of the following are true:

- the repository was cloned or pulled successfully
- `wproj check --what wpl --fail-fast` passed
- `wparse daemon` stayed online without restart
- `wproj engine reload` returned an acceptable result
- post-reload runtime status is queryable
- the new rule behavior was verified through stats, logs, or sink output

## Minimal Command Set

First deployment:

```bash
git clone <repo> /srv/wp/<project>
cd /srv/wp/<project>
wproj check
cargo run --bin wparse -- daemon --work-root .
```

Daily update:

```bash
cd /srv/wp/<project>
git pull
wproj check --what wpl --fail-fast
cargo run --bin wproj -- engine reload \
  --work-root . \
  --request-id rule-$(date +%Y%m%d%H%M%S) \
  --reason "rule update"
cargo run --bin wproj -- engine status --work-root .
```
