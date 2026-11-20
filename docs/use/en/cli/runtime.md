# `wparse` Runtime Usage

## Scope

This document explains the role of the `wparse` runtime binary, the common commands it exposes, and when each mode should be used.

If you need:

- runtime status and reload: see [../operations/admin.md](../operations/admin.md)
- project initialization, checks, and topology inspection: see [project.md](project.md)
- test-data generation: see [generator.md](generator.md)

## Command Role

`wparse` is the actual engine entrypoint. It exposes three subcommands:

- `daemon`: long-running mode for online workloads
- `batch`: one-shot mode for offline replay and validation
- `check`: offline, read-only config check for pre-deploy gating and CI

## Common Commands

Show help:

```bash
wparse --help
```

Start the daemon:

```bash
wparse daemon --work-root .
```

Run batch mode:

```bash
wparse batch --work-root .
```

Check the config offline (no sinks/sources are started):

```bash
wparse check --work-root .
wparse check --work-root . --json
```

## Common Flags

- `--work-root`
- `--max-line`
- `--parse-workers`
- `--stat`
- `--print_stat`
- `--robust`
- `--log-profile`
- `--wpl`

## Choosing `daemon` vs `batch`

Prefer `daemon` when:

- the process should stay online
- runtime admin APIs are needed
- online reload is required

Prefer `batch` when:

- validating rules locally
- replaying sample data once
- running offline checks before rollout

## `check` (offline config check)

`check` loads the project through the same path as `daemon` (route confs +
WPL/OML + sink indexes) but opens **no sockets, takes no locks, writes no
files**, and does not need `data/` or `.run/`. Use it to catch a broken config
before a deployment swaps a working container, and in CI against a published
seed.

Exit codes:

- `0`: no errors
- `1`: semantic config errors
- `2`: the config could not be loaded (TOML syntax, missing file, missing dir)

Checks:

- the same `sink_group.name` declared by more than one route file (the name is the
`SinkID`, so the groups overwrite each other)
- the same `oml` matcher bound by two `sink_group`s (including `*` twice); both
group names and files are reported
- the same `rule` matcher bound by two `sink_group`s
- rule/model index divergence: a rule is routed to a group that has no matching
OML model, so runtime silently falls back to passthrough
- inert bindings (warning): an `oml`/`rule` matcher matches no loaded
model/rule

`oml = ["*"]` in one group plus a **concrete** model name in another is not a
violation: both indexes converge on the same group. `--strict` also fails on
warnings.

If the work root is replaced by an external config sync and a pre-flight
`check` cannot be guaranteed, enable startup fail-closed in the site-local
overlay (`project_remote` does not replace `wparse.local.toml`):

```toml
# wparse.local.toml (sibling of conf/)
[validate]
deny_duplicate_oml = true
```

Default `false`: duplicates only log an `error` and startup continues. With
`true`, a duplicate `oml` binding makes the engine fail to start. The section is
also accepted in `conf/wparse.toml` (useful in a published seed); when both are
present, `wparse.local.toml` wins.

## Constraints

- the admin API is available only in `daemon`
- `batch` is for offline workflows, not online reload
- `check` only validates config; it does not process data and does not replace
`wproj check`
- there is no separate `work` subcommand at the moment

## Related Docs

- CLI usage guide: [index.md](index.md)
- Runtime admin usage: [../operations/admin.md](../operations/admin.md)
- Project tool usage: [project.md](project.md)

## State Directory

Engine state (pid file, single-instance lock, `rule_mapping.dat`,
`authority.sqlite`, and the `project_remote` cache) lives in one directory:
`<work_root>/.run` by default.

Move it when the work root is mounted read-only, or when state should live on a
dedicated volume:

```toml
# wparse.local.toml (site-local, not overwritten by project_remote)
[runtime]
state_dir = "/state"
```

Precedence: `--state-dir` > `wparse.local.toml` `[runtime]` > `conf/wparse.toml`
`[runtime]` > `<work_root>/.run`.

The daemon logs the effective directory at startup (`runtime state_dir: ...`).
Constraints:

- the value must not be the work root itself, an ancestor of it, or inside a
  `project_remote`-managed directory (`conf`, `models`, `topology`, `connectors`)
- for container deployments mount a dedicated volume at that path and keep it on
  the same host/kernel as the engine (`flock` is the single-instance guard)
- changing the directory does not migrate existing state; move the files before
  switching, otherwise a fresh `authority.sqlite` / `project_remote_state.json`
  is built at the new location
