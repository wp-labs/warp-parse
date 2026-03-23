# Remote Rule Version Update Design

- Status: Draft
- Scope: `wproj init --repo`, `wproj conf update`, `wproj engine reload`, and HTTP admin API `POST /admin/v1/reloads/model`

## Requirement Summary

This design follows these requirements:

1. project configuration must contain a remote repo address and an enable switch
2. operators should not need to use Git directly
3. `wproj` must provide one explicit entry: `wproj conf update`
4. runtime "update and reload" must perform conf update first
5. manual update and runtime reload must not maintain separate sync logic
6. the HTTP admin API must provide both an "update or not" flag and a version parameter
7. `wproj init` must support direct project initialization from remote

## Design Conclusion

Treat remote project synchronization as a shared project capability, not as private behavior of a single binary.

- configuration lives in a project-level config file shared by `wproj` and `wparse`
- `wproj init --repo` provides bootstrap remote parameters for first initialization and triggers the first sync
- `wproj conf update` is the explicit operator entry
- `wproj engine reload --update` and the HTTP admin API reload are runtime entries
- all entries reuse the same conf update core flow

From the user perspective, the action is "sync rules from a remote version source", not low-level repository manipulation.

## Non-Goals

- exposing low-level repository operations to users
- turning this feature into a general repository-management tool
- rule authoring workflows
- replacing `wproj self update`
- multi-remote or multi-branch orchestration in the first version
- designing a separate runtime restart API for now

## Config Boundary

This capability is best treated as an optional project-level config section.

Location:

```text
conf/wparse.toml
```

Rules:

- `wparse.toml` remains the only required config file
- `[project_remote]` is an optional section inside it
- if `[project_remote]` is absent, `wparse` still starts normally
- `wproj conf update`, `wproj engine reload --update`, and HTTP admin API reload read the same section

## Config Model

Add this as an optional section in `conf/wparse.toml`:

```toml
[project_remote]
enabled = true
repo = "https://github.com/wp-labs/editor-monitor-conf.git"
init_version = "1.4.2"
```

Field notes:

- `enabled`: master switch; remote sync is disabled when false
- `repo`: remote repository address
- `init_version`: initial version used only before remote state has ever been initialized locally

Minimum required fields:

- `enabled`
- `repo`

Recommended field:

- `init_version`

Tag convention:

- if the remote publishes `v1.4.2`, users still pass `1.4.2`
- the system resolves it internally as a semantic release tag

Repository available for testing and integration:

- `https://github.com/wp-labs/editor-monitor-conf.git`

## External Interfaces

### 1. First-Time Initialization Entry

```bash
wproj init --repo <REPO> [--version <VERSION>]
```

Semantics:

- `wproj init --repo` initializes the local project skeleton first
- `--repo` / `--version` are bootstrap parameters only for the first sync
- then it reuses the same sync, validation, and rollback flow as `wproj conf update`
- after first sync, the project configuration from the remote repository becomes authoritative

Version-selection rules:

- if `--version` is provided, it must resolve to one unique release tag
- if `--version` is omitted, the system first tries the latest release tag
- if the remote has no release tag, the system falls back to the remote default branch `HEAD`

HEAD fallback result semantics:

- `resolved_tag = "HEAD@<branch>"`
- `current_version = "<branch>"`

### 2. Explicit Update Entry

```bash
wproj conf update [--version <VERSION>]
```

Semantics:

- `wproj conf update` performs version synchronization against the configured remote
- operators do not need to know whether this is first deployment or a daily update
- `wproj conf update` does not imply automatic reload
- whether reload happens next is decided explicitly by runtime control

Version-selection rules:

- if the command explicitly passes `--version`, use that version
- if this is first initialization and no version is passed, use `init_version` first
- if this is not first initialization and no version is passed, prefer the latest release tag
- if the remote has no release tag, fall back to the remote default branch `HEAD`

Constraints:

- explicit `--version` still supports only tag-based upgrade or rollback
- automatic fallback to `HEAD` happens only when no explicit version is requested

### 3. Runtime Reload Entry

CLI:

```bash
wproj engine reload [--update] [--version <VERSION>]
```

HTTP:

```http
POST /admin/v1/reloads/model
```

Request fields added:

- `update: bool`
- `version: string | null`

Runtime semantics:

- `update = false`: run `LoadModel` against the current working tree only
- `update = true`: perform conf update first, then reload
- non-empty `version`: use that version for conf update
- empty `version`: resolve the target by the default version-selection rule

Default version-selection rules are the same as `wproj conf update`:

- first initialization prefers `init_version`
- later updates prefer the latest release tag
- if there is no release tag, fall back to the remote default branch `HEAD`

Invalid combinations:

- `update = false` with non-empty `version` -> reject directly
- `project_remote.enabled = false` with `update = true` -> update fails and reload does not continue

Example:

```json
{
  "update": true,
  "version": "1.4.3",
  "wait": true,
  "timeout_ms": 15000,
  "reason": "rule update and reload"
}
```

## Shared Core Flow

`wproj init --repo`, `wproj conf update`, `wproj engine reload --update`, and the HTTP admin API reload must reuse the same core module.

The core flow is responsible for:

1. reading project sync config
2. resolving the target version for the current action
3. updating the selected version inside a dedicated remote directory
4. creating a backup for the managed-directory whitelist inside the current working directory
5. copying the managed-directory whitelist from remote into the current working directory
6. running post-update validation
7. returning structured results on success
8. restoring the managed-directory whitelist from backup on failure

Runtime reload continues only after conf update succeeds.

## Directory Model

This design does not switch Git state directly inside the live working directory. Instead, it uses a three-directory model:

- `remote`: cached project directory used for clone, fetch, and version checkout
- `current`: the live working directory actually used by `wparse`
- `backup`: backup copy of `current`, used for rollback

Rules:

- Git operations happen only in `remote`
- `current` must always represent a runnable project snapshot
- if validation or reload fails, the system must restore `current` from `backup`
- directory switching uses a managed-directory whitelist instead of overwriting the entire work tree

Managed-directory whitelist:

- `conf/`
- `models/`
- `topology/`
- `connectors/`

Runtime-local directories excluded from switching:

- `data/`
- `logs/`
- `.run/`
- `runtime/`

Rules:

- backup only includes whitelist-managed directories
- remote -> current copy only includes whitelist-managed directories
- restore only applies to whitelist-managed directories
- if a file or directory disappears from the target release inside the whitelist, it must also be deleted from `current`

## Unified Update Flow

No matter whether the entry comes from `wproj init --repo`, `wproj conf update`, or `reload + update`, the system should follow this flow:

1. resolve the remote directory from configuration
2. update the remote directory and switch it to the target version
3. back up the managed-directory whitelist from current into backup
4. copy the managed-directory whitelist from remote into current
5. run post-update validation
6. if the caller requested reload, continue with `LoadModel`
7. if validation fails or reload fails, restore the managed-directory whitelist from backup

For `wproj init --repo`, two extra steps happen before entering the flow above:

1. generate the local project skeleton
2. pass `--repo` / `--version` into the core flow as bootstrap parameters

## Release-Version Sync Semantics

Once repository implementation details are hidden from the user, the system should expose stable version-sync semantics:

- first initialization syncs to `init_version`, an explicitly requested version, or an auto-resolved default target
- later updates sync to the version selected for the current action or the auto-resolved default target
- auto mode prefers release tags; if no release tag exists, it falls back to default-branch `HEAD`
- when local state violates safety rules, the action fails directly

## Why `init_version`, Action Version, And `current_version` Must Be Separate

If releases are published through version tags, operators care about:

- which version should be used for first-time initialization
- which version should be activated for this specific action
- which version is currently active now

Recommended separation:

- `init_version`: fixed config for first initialization only
- `version`: action parameter for the current `conf update` or `reload --update`
- `current_version`: state field for the current result

This supports first deployment, normal upgrade, explicit rollback, and the non-tag state where the current target is default-branch `HEAD`.

## Manual Update Semantics

The meaning of `wproj conf update` is:

1. synchronize the remote directory to the target selected for the current action
2. back up the managed-directory whitelist in the current working directory
3. overwrite the current working directory with the managed-directory whitelist from remote
4. run post-update validation
5. return success on completion
6. restore backup on failure

`wproj conf update` does not imply automatic reload.

## Reload Semantics

The meaning of `wproj engine reload --update` or HTTP admin API `update = true` is:

1. perform conf update
2. if conf update fails, fail the whole request and do not continue reload
3. if conf update succeeds, execute `LoadModel` against the updated `current` tree
4. if reload fails, roll back the managed-directory whitelist

If `update = false`:

- only `LoadModel` is executed against the current tree
- no remote synchronization is attempted

## Minimum Validation Gate

No matter which entry triggers the sync, a minimum validation gate is required after synchronization.

First version recommendation:

- always perform WPL-related validation
- runtime update entry must pass this gate before reload

Equivalent target behavior:

```bash
wproj check --what wpl --fail-fast
```

## Result Model

Suggested structured output:

```json
{
  "action": "conf_update",
  "trigger": "admin_api",
  "work_root": "/srv/wp/project-a",
  "repo": "ssh://git@github.com/acme/wp-project.git",
  "update": true,
  "requested_version": "1.4.2",
  "init_version": "1.4.2",
  "current_version": "1.4.2",
  "resolved_tag": "v1.4.2",
  "sync_result": "updated",
  "from_revision": "abc1234",
  "to_revision": "def5678",
  "validation_result": "passed",
  "runtime_action": "reload",
  "runtime_result": "success"
}
```

HEAD fallback example:

```json
{
  "requested_version": null,
  "current_version": "main",
  "resolved_tag": "HEAD@main"
}
```

Suggested `trigger` values:

- `manual`
- `engine_reload`
- `admin_api`

Suggested `runtime_action` values:

- `none`
- `reload`

## Failure Semantics

Three failure classes must stay distinct:

1. sync failure
2. sync success but validation failure
3. sync and validation success but reload failure

Required behavior:

- if conf update fails, reload must not continue
- if reload fails, the whole operation must not be reported as success
- if sync already succeeded, the state must still retain the new revision, `current_version`, and `resolved_tag`

## Lock And State Files

Project-local files:

```text
.run/project_remote.lock
.run/project_remote_state.json
```

Purpose:

- prevent concurrent updates
- prevent update/reload overlap
- store the latest successful revision, latest `current_version`, and latest `resolved_tag`

Definition of "first initialization":

- it is determined by whether `.run/project_remote_state.json` exists
- it is not determined by whether the project directory already has content

## Security Constraints

- `repo` must come from local config or `wproj init --repo` bootstrap parameters, not runtime request bodies
- low-level repository commands must not be exposed to users
- external callers must not depend on repository implementation details
- dirty local worktrees must be rejected by rule
- remote authentication should reuse standard SSH key / token mechanisms

## Relationship To Existing Capabilities

- `wproj self update`: upgrades Warp Parse binaries
- `wproj init --repo`: creates the project skeleton and triggers the first remote sync
- `wproj conf update`: manually triggers project config synchronization
- `wproj engine reload`: runtime activation action; with `--update`, it syncs first and reloads second
- HTTP admin API reload: remote control surface for `wproj engine reload`

These capabilities are layered and should not replace each other.

## MVP

First version should include:

- optional `[project_remote]` in `conf/wparse.toml`
- `wproj init --repo`
- `wproj conf update`
- `wproj engine reload --update`
- HTTP admin API `update` / `version` parameters
- `init_version` config semantics
- `--version` action parameter semantics
- `current_version` and `resolved_tag` state semantics
- version/tag resolution and version-based synchronization
- automatic fallback to remote default-branch `HEAD` when no release tag exists
- fixed dirty-worktree protection
- minimum WPL validation
- lock file and state file
- JSON output

Future work:

- polling-based update
- multi-branch / multi-remote support
- finer validation policy
- approval-based rollout

## Acceptance Criteria

- operators only need repo config and a switch; they do not use Git directly
- `wproj init --repo` can complete first-time remote initialization directly
- `wproj conf update` handles later updates
- `wproj engine reload --update` and the HTTP admin API reload perform conf update first
- manual update and runtime reload share one sync core
- the HTTP admin API can explicitly specify whether to update and which version to use
- `init_version` is used only for first initialization
- upgrades and rollbacks are driven by the action parameter `version`
- `current_version` / `resolved_tag` are state only and are not written back into fixed config
- when the remote has no release tag, the system falls back to default-branch `HEAD`
- reload does not continue after conf update failure
- reload does not continue after validation failure
- results are traceable in structured form

## Logging And Troubleshooting

Treat the remote update logs as the primary troubleshooting entry point.

Key log messages:

- `project remote sync start`
- `project remote sync target resolved`
- `project remote sync tag resolved`
- `project remote sync diff`
- `project remote sync apply managed dirs`
- `project remote sync done`
- `project remote sync apply failed`
- `project remote sync rollback done`
- `wproj conf update start`
- `wproj conf update validate failed`
- `wproj conf update rollback done`
- `admin api project update start`
- `admin api project update done`
- `admin api project update failed`
- `admin api project rollback done`

Recommended fields to grep:

- `request_id`
- `work_root`
- `requested_version`
- `current_version`
- `resolved_tag`
- `from_revision`
- `to_revision`
- `changed`
- `error`

Common troubleshooting examples:

```bash
grep -E "project remote sync|wproj conf update|admin api project update" data/logs/wparse.log
```

```bash
grep -E "project remote sync apply failed|validate failed|rollback" data/logs/wparse.log
```

Use these logs to answer:

- which version was requested
- which tag / branch / commit was finally resolved
- whether managed directories were actually switched
- whether the failure happened in sync, validation, or reload
- whether rollback ran and whether rollback itself failed

## Chinese Counterpart

- [../zh/project_remote_sync_design.md](../zh/project_remote_sync_design.md)
