# `wparse` Runtime Usage

## Scope

This document explains the role of the `wparse` runtime binary, the common commands it exposes, and when each mode should be used.

If you need:

- runtime status and reload: see [../operations/admin.md](../operations/admin.md)
- project initialization, checks, and topology inspection: see [project.md](project.md)
- test-data generation: see [generator.md](generator.md)

## Command Role

`wparse` is the actual engine entrypoint. It currently exposes two modes:

- `daemon`: long-running mode for online workloads
- `batch`: one-shot mode for offline replay and validation

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

## Constraints

- the admin API is available only in `daemon`
- `batch` is for offline workflows, not online reload
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
