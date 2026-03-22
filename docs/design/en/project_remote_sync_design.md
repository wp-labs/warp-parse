# Remote Rule Version Update Design

- Status: Draft
- Scope: `wproj conf update`, `wparse restart`, and HTTP admin API runtime instructions

## Requirement Summary

This design follows these requirements:

1. project configuration must contain a remote Git repo address and an enable switch
2. operators should not need to use Git directly
3. `wproj` must provide one explicit entry: `wproj conf update`
4. when `wparse` receives a restart instruction, it must also perform conf update first
5. manual update and runtime restart must not maintain separate sync logic
6. HTTP admin API must provide both an "update or not" flag and a version parameter

## Design Conclusion

Treat remote project synchronization as a shared project capability, not as private behavior of a single binary.

- configuration lives in a project-level config file shared by `wproj` and `wparse`
- `wproj conf update` is the explicit operator entry
- `wparse restart` is the implicit runtime entry
- both entries reuse the same conf update core flow

From the user perspective, the action is "sync rules from a remote version source", not low-level repository manipulation.

## Non-Goals

- exposing low-level repository operations to users
- turning this feature into a general repository-management tool
- rule authoring workflows
- replacing `wproj self update`
- multi-remote or multi-branch orchestration in the first version

## Config Boundary

This capability is best treated as an optional `wparse` configuration section that `wproj` also reads.

So the better location is:

```text
conf/wparse.toml
```

The rule is:

- `wparse.toml` remains the only required config file
- `[project_remote]` is an optional section inside it
- if `[project_remote]` is absent, `wparse` still starts normally
- `wproj conf update` reads the same section from the same file

## Config Model

Add this as an optional section in `conf/wparse.toml`:

```toml
[project_remote]
enabled = true
repo = "ssh://git@github.com/acme/wp-project.git"
init_version = "1.4.2"
```

Field notes:

- `enabled`: master switch; remote sync is disabled when false
- `repo`: remote repository address
- `init_version`: initial version used only for first-time initialization when no local project content exists

Minimum required fields:

- `enabled`
- `repo`

Recommended fields:

- `init_version`

If the repository follows a uniform release-tag rule such as `v1.4.2`, the system should resolve:

- config `init_version = "1.4.2"`
- to tag `v1.4.2` during initialization

If other tag schemes are needed later, add them later. The first version should not expose extra strategy fields yet.

## External Interfaces

### 1. Explicit Entry

```bash
wproj conf update
```

Suggested options:

```bash
wproj conf update [OPTIONS]

Options:
  -w, --work-root <DIR>
      --version <VERSION>
      --dry-run
      --json
      --reason <TEXT>
```

Semantics:

- `wproj conf update` performs remote version synchronization
- operators do not need to know whether this is first deployment or a daily update
- operators do not deal with repository operations directly
- `--version` explicitly defines the target version for this update, which supports both upgrade and rollback
- if `--version` is absent:
- first initialization uses `init_version`
- later updates may follow the "latest released version" rule

### 2. Implicit Entry

When `wparse` receives a restart instruction:

1. read `conf/wparse.toml`
2. if `project_remote.enabled = true`
3. resolve whether the restart instruction carries an explicit version
4. if a version is present, use that version for conf update
5. if no version is present:
6. use `init_version` for first initialization
7. use the latest released version rule for later updates
8. only after successful conf update continue with restart
9. if conf update fails, reject the restart

This is the core rule of the design.

### 3. HTTP Admin API Entry

The runtime admin API needs to express two things:

- whether the runtime action should execute conf update first
- which version should be used if update is requested

Suggested shared request-body fields:

- `update`: `bool`
- `version`: `string | null`

Recommended semantics:

- `update = true`: run conf update before the runtime action
- `update = false`: do not run conf update for this runtime action
- non-empty `version`: use this version as the target version for conf update
- empty `version`: use the default version-selection rule

Suggested request body:

```json
{
  "update": true,
  "version": "1.4.3",
  "wait": true,
  "timeout_ms": 15000,
  "reason": "restart with rule update"
}
```

If reload and restart later become separate endpoints, both should reuse these two fields instead of inventing separate version parameters.

## Shared Core Flow

`wproj conf update` and `wparse restart` must reuse the same core module.

Suggested internal abstraction:

```text
project_sync_core
```

The core module is responsible for:

1. reading project sync config
2. inspecting local directory state
3. resolving the target version for the current action
4. validating remote URL and target release version
5. enforcing a fixed dirty-worktree protection rule
6. running minimum validation
7. returning structured results

`wproj` and `wparse` are responsible only for:

- deciding when to call it
- deciding whether the follow-up runtime action is `reload` or `restart`

At the host layer, the HTTP admin API only maps request parameters into one unified action context:

- `trigger = admin_api`
- `update = true/false`
- `requested_version = <version or null>`
- `runtime_action = reload/restart`

## Release-Version Sync Semantics

Once repository implementation details are hidden from the user, the system should expose stable version-sync semantics:

- first initialization syncs to `init_version` or an explicitly requested version
- later updates sync to the version selected for the current action
- when no explicit version is provided, the default rule may resolve the latest released version
- when local state violates safety rules, the action fails directly

## Sync Strategy

The first version should support only the safest predictable path:

- the sync target is a concrete release version, not a moving branch head
- the target version for the current action must resolve to one unique release tag
- fail immediately on dirty local worktrees

That makes "update rules" point to a stable and auditable released version instead of a time-varying branch head.

## Why `init_version`, Action Version, And `current_version` Must Be Separate

If releases are published through version tags, operators care about:

- which version should be used for first-time initialization
- which version should be activated for this specific action
- which version is currently active now

Those are different concerns and should not be collapsed into one field.

Recommended separation:

- `init_version`: fixed config for first initialization only
- `version`: action parameter for the current `conf update` or `restart`
- `current_version`: state field for the current result

This supports:

- first deployment
- normal upgrade
- explicit rollback

without forcing the config file to carry a permanently changing target version.

## Restart Semantics

When `wparse` receives a restart instruction, its meaning changes from "restart immediately from the local tree" to:

1. synchronize the project to the configured remote state
2. restart from the synchronized tree

If `project_remote.enabled = false`:

- `wparse` may still perform a local restart
- but the result must explicitly state that no remote sync was attempted

## Manual Update Semantics

The meaning of `wproj conf update` is:

1. synchronize the project tree to the release version selected for the current action
2. run minimum validation
3. return the update result

`wproj conf update` does not imply automatic reload or restart.

Whether the next runtime action is:

- `wproj engine reload`
- `wparse restart`

should be decided explicitly by runtime control, not by extra config switches.

Version selection rules:

- if the command explicitly passes `--version`, use that version
- if this is first initialization and no version is passed, use `init_version`
- if this is not first initialization and no version is passed, update to the latest released version

HTTP admin API follows the same version-selection rules:

- if `update = false`, ignore `version`
- if `update = true` and `version` is non-empty, use that version
- if `update = true` and `version` is empty:
- use `init_version` for first initialization
- use the latest released version for later updates

To avoid ambiguity, invalid combinations should be rejected directly:

- `update = false` with a non-empty `version` -> invalid request

## Minimum Validation Gate

No matter which entry triggers the sync, a minimum validation gate is required after synchronization.

First version recommendation:

- always perform WPL-related validation

Equivalent target behavior:

```bash
wproj check --what wpl --fail-fast
```

If validation fails:

- `wproj conf update` returns failure
- `wparse restart` must not continue into restart

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
  "runtime_action": "restart",
  "runtime_result": "success"
}
```

Suggested `sync_result` values:

- `disabled`
- `cloned`
- `up_to_date`
- `updated`
- `init_version_missing`
- `version_not_found`
- `dirty_worktree`
- `remote_mismatch`
- `invalid_worktree`
- `validation_failed`

Suggested `trigger` values:

- `manual`
- `wparse_restart`
- `admin_api`

Suggested `runtime_action` values:

- `none`
- `reload`
- `restart`

## Failure Semantics

Three failure classes must stay distinct:

1. sync failure
2. sync success but validation failure
3. sync and validation success but runtime action failure

Required behavior:

- if conf update fails, `wparse restart` must not continue
- if runtime action fails, the whole operation must not be reported as success
- if sync already succeeded, the state must still retain the new revision and current version information

## Lock And State Files

Suggested project-local files:

```text
.run/conf_update.lock
.run/conf_update_state.json
```

Purpose:

- prevent concurrent updates
- prevent update/restart overlap
- store the latest successful revision, latest current version, latest trigger, and latest failure reason

## Security Constraints

- `repo` must come from local config, not interactive input
- low-level repository commands must not be exposed to users
- external callers must not depend on repository implementation details
- dirty local worktrees must be rejected by rule, not bypassed through config
- remote authentication should reuse standard SSH key / token mechanisms rather than inventing a new CLI auth layer

## Relationship To Existing Capabilities

- `wproj self update`: upgrades Warp Parse binaries
- `wproj conf update`: manually triggers project config synchronization
- `wparse restart`: runtime-triggered "sync first, restart second"
- `wproj engine reload`: an independent runtime activation action

These capabilities are layered and should not replace each other.

## MVP

First version should include:

- optional `[project_remote]` in `conf/wparse.toml`
- `wproj conf update`
- conf update before `wparse restart`
- HTTP admin API `update` / `version` parameters
- `init_version` config semantics
- `--version` action parameter semantics
- `current_version` state semantics
- version/tag resolution and version-based synchronization
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
- `wproj conf update` handles both initial sync and later updates
- `wparse` performs conf update before restart
- manual update and runtime restart share one sync core
- HTTP admin API can explicitly specify whether to update and which version to use
- `init_version` is used only for first initialization
- upgrades and rollbacks are driven by the action parameter `version`
- `current_version` is state only and is not written back into fixed config
- restart does not continue after conf update failure
- reload/restart does not continue after validation failure
- results are traceable in structured form

## Chinese Counterpart

- [../zh/project_remote_sync_design.md](../zh/project_remote_sync_design.md)
