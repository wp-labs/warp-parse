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
