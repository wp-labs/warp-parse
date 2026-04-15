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

- working `wproj` and `wparse` binaries
- a fixed work root such as `/srv/wp/<project>`
- a remote repository that already contains a valid WP project layout

Before rollout, confirm:

- the runtime uses `wparse daemon`, not `batch`
- paths in `conf/wparse.toml` are valid relative to the work root
- the remote repository either publishes release tags or at least keeps a usable default branch
- the machine has the SSH key or token required to access the repository

## Enable The Runtime Admin API

Hot reload depends on the runtime admin API. Configure `conf/wparse.toml` as described in [admin.md](admin.md):

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

Prepare the token file before startup:

```bash
mkdir -p runtime
mkdir -p "${HOME}/.warp_parse"
printf 'replace-with-a-secret-token\n' > "${HOME}/.warp_parse/admin_api.token"
chmod 600 "${HOME}/.warp_parse/admin_api.token"
```

Constraints:

- `batch` mode does not expose the admin API
- on Unix, the token file must be owner-only
- non-loopback binds require TLS

## First Deployment

### 1. Initialize From Remote

Initialize to an explicit released version:

```bash
wproj init \
  --work-root /srv/wp/<project> \
  --repo https://github.com/wp-labs/editor-monitor-conf.git \
  --version 1.4.2
```

Initialize to the default target:

```bash
wproj init \
  --work-root /srv/wp/<project> \
  --repo https://github.com/wp-labs/editor-monitor-conf.git
```

Notes:

- `wproj init --repo` creates the local project skeleton first
- `--repo` / `--version` are bootstrap parameters only for the first sync
- then it reuses `wproj conf update` for first sync and validation
- after first sync, configuration from the remote repository becomes authoritative
- explicit `--version` is for tag-based initialization and rollback-friendly bootstrap
- if `--version` is omitted, it first resolves the latest release tag from remote
- if the remote has no release tags, it falls back to the remote default branch `HEAD`

Typical output semantics when the default falls back to remote `HEAD`:

- `Version: main` or `master`
- `Tag: HEAD@main` or `HEAD@master`

### 2. Validate The Project

```bash
wproj check
wproj data stat
```

### 3. Start The Daemon

```bash
wparse daemon --work-root .
```

Then verify runtime status:

```bash
wproj engine status --work-root .
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

Update the project content first:

```bash
wproj conf update --work-root /srv/wp/<project>
```

To upgrade or roll back to a specific released version:

```bash
wproj conf update --work-root /srv/wp/<project> --version 1.4.3
```

Default version-selection rules:

- on first initialization, use `init_version` first when configured
- on later updates, prefer the latest release tag
- if the remote has no release tag, fall back to the remote default branch `HEAD`

Run a minimal gate before reload:

```bash
wproj check --what wpl --fail-fast
```

Inspect current runtime status:

```bash
wproj engine status --work-root .
```

Reload only the already-updated local content:

```bash
wproj engine reload \
  --work-root . \
  --request-id rule-$(date +%Y%m%d%H%M%S) \
  --reason "rule reload"
```

If you want a single runtime action that updates and reloads:

```bash
wproj engine reload \
  --work-root . \
  --update \
  --request-id update-$(date +%Y%m%d%H%M%S) \
  --reason "rule update and reload"
```

To upgrade or roll back and reload in one step:

```bash
wproj engine reload \
  --work-root . \
  --update \
  --version 1.4.3 \
  --request-id update-rollback-$(date +%Y%m%d%H%M%S) \
  --reason "switch release and reload"
```

Check status again after reload:

```bash
wproj engine status --work-root .
```

### Result Interpretation

For `wproj conf update`, focus on:

- `Request`
- `Version`
- `Tag`
- `Changed`

For `wproj engine reload`, focus on:

- `Result`
- `Updated`
- `Request V`
- `Current V`
- `Tag`

Common results:

- `reload_done`: reload completed successfully
- `running`: the request was accepted and is still running
- `reload_in_progress`: another reload is already active
- `update_in_progress`: another project update is already active
- `update_failed`: the update stage failed before reload

Version-field semantics:

- `Request V`: explicit requested version for this action; empty when auto-resolved
- `Current V`: the version actually activated by this update
- `Tag`: the resolved remote target; release tags look like `v1.4.3`
- when the flow falls back to default-branch `HEAD`, `Tag` looks like `HEAD@main`

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

If you trigger "update + reload" as one runtime action from the admin plane, move the validation gate earlier into the release repository workflow.

If the release includes source, sink, or main config changes, run the full project check:

```bash
wproj check
```

## Rollback SOP

If a reload introduces parse failures, field regressions, or downstream alarms, do not restart the daemon first. Roll back the project version and reload again:

```bash
wproj conf update --work-root /srv/wp/<project> --version 1.4.2
wproj engine reload \
  --work-root /srv/wp/<project> \
  --request-id rollback-$(date +%Y%m%d%H%M%S) \
  --reason "rollback rule set"
```

You can also do the rollback in one runtime action:

```bash
wproj engine reload \
  --work-root /srv/wp/<project> \
  --update \
  --version 1.4.2 \
  --request-id rollback-$(date +%Y%m%d%H%M%S) \
  --reason "rollback and reload"
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
wproj engine status \
  --work-root /srv/wp/<project> \
  --admin-url http://127.0.0.1:19090 \
  --token-file "${HOME}/.warp_parse/admin_api.token"
```
