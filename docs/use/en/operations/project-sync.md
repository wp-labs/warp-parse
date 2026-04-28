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

## Dual-Repo Mode (Separate models / infra)

### Architecture Overview

Dual-repo mode splits the project into two independently-updated groups:

```
Project Layout                  Source Repository
─────────────                   ────────────────
models/                         models repo (e.g. wp-rule)
├── wpl/                          parsing rules
├── oml/                          model definitions
└── knowledge/                    knowledge base

conf/        ┐
topology/    ├── infra group ─→  infra repo (e.g. editor-monitor-conf)
connectors/  ┘                    config, topology, connectors
```

| Group | Managed Directories | Purpose |
|-------|-------------------|---------|
| `models` | `models/` | Parse rules (wpl), model definitions (oml), knowledge base |
| `infra` | `conf/`, `topology/`, `connectors/` | Main config, source/sink topology, connector configs |

Each repo has independent versioning. Upgrading models does not affect infra config, and vice versa.

### Configuration

```toml
[project_remote]
enabled = true
# repo must be empty in dual-repo mode
repo = ""

[project_remote.models]
repo = "https://github.com/wp-labs/wp-rule.git"
init_version = "0.1.0"       # version used on first initialization

[project_remote.infra]
repo = "https://github.com/wp-labs/editor-monitor-conf.git"
init_version = "0.1.6"       # version used on first initialization
```

**Field Reference:**

| Field | Required | Description |
|-------|----------|-------------|
| `[project_remote].enabled` | Yes | Master switch for remote sync (shared by both groups) |
| `[project_remote].repo` | No | Must be empty `""` in dual-repo mode |
| `[project_remote.models].repo` | Yes | Git URL of the models repository |
| `[project_remote.models].init_version` | No | Version used on first sync; defaults to latest tag thereafter |
| `[project_remote.infra].repo` | Yes | Git URL of the infra repository |
| `[project_remote.infra].init_version` | No | Version used on first sync; defaults to latest tag thereafter |

### Version Resolution Rules

How `wproj conf update --group <group>` resolves the target version:

1. If `--version` is explicitly provided → use that version
2. If the group has **never been initialized** (no entry in state file) → use configured `init_version` if present, otherwise use the latest remote tag
3. If the group **already has a state record** → use the latest remote tag

This ensures sensible defaults for both initial deployment and subsequent updates.

### Sync Flow

`wproj conf update --group <group>` steps:

1. Clone/update the remote repository to a local cache (`.run/project_remote/remote-<group>/`)
2. Fetch remote tags, resolve the target version per the resolution rules above
3. Checkout the target commit
4. Compare cache with work root — if managed directories differ:
   - Back up the current managed directories from the work root
   - Copy managed directories from the cache to the work root
5. Persist state to `.run/project_remote_state.json`

**Cache Paths:**

| Group | Cache Path |
|-------|-----------|
| models | `.run/project_remote/remote-models/` |
| infra | `.run/project_remote/remote-infra/` |
| single-repo | `.run/project_remote/remote/` |

### State File Format

Dual-repo `.run/project_remote_state.json`:

```json
{
  "models": {
    "version": "0.1.0",
    "tag": "0.1.0",
    "revision": "fcfc9e5..."
  },
  "infra": {
    "version": "0.1.6",
    "tag": "v0.1.6",
    "revision": "e2e84e1..."
  }
}
```

### Initialization Order

In dual-repo mode, **infra must be initialized first, then models**. This is because the infra sync writes `conf/wparse.toml` (the project main config), which contains the dual-repo repository URLs. The models sync reads the models repo URL from this config.

> **Best practice:** The infra repository's own `conf/wparse.toml` should contain the complete dual-repo configuration (`[project_remote.models]` + `[project_remote.infra]`). This way, after infra sync, models sync can proceed directly without any manual config patching.

### Commands

Update the models group:

```bash
wproj conf update --work-root /srv/wp/<project> --group models --version 1.4.3
```

Update the infra group:

```bash
wproj conf update --work-root /srv/wp/<project> --group infra --version 1.1.0
```

In dual-repo mode, `--group` is required. In single-repo mode, `--group` must not be used.

### Rollback Behavior

Per-group rollback only affects the directories managed by that group:

- `--group models` rollback → only restores `models/`; `conf/`/`topology/`/`connectors/` unaffected
- `--group infra` rollback → only restores `conf/`/`topology/`/`connectors/`; `models/` unaffected

The current state is backed up before rollback. If the rollback fails, the backup is restored.

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

In dual-repo mode, update by group:

```bash
wproj conf update --work-root /srv/wp/<project> --group models --version 1.4.3
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

In dual-repo mode, add `--group`:

```bash
wproj engine reload \
  --work-root . \
  --update \
  --group models \
  --request-id update-models-$(date +%Y%m%d%H%M%S) \
  --reason "models update and reload"
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

Single-repo rollback:

```bash
wproj conf update --work-root /srv/wp/<project> --version 1.4.2
wproj engine reload \
  --work-root /srv/wp/<project> \
  --request-id rollback-$(date +%Y%m%d%H%M%S) \
  --reason "rollback rule set"
```

Dual-repo rollback by group:

```bash
wproj conf update --work-root /srv/wp/<project> --group models --version 1.4.2
wproj engine reload \
  --work-root /srv/wp/<project> \
  --request-id rollback-$(date +%Y%m%d%H%M%S) \
  --reason "rollback models"
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

Dual-repo:

```bash
wproj engine reload \
  --work-root /srv/wp/<project> \
  --update \
  --group models \
  --version 1.4.2 \
  --request-id rollback-models-$(date +%Y%m%d%H%M%S) \
  --reason "rollback models and reload"
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
