# `wproj` Project Tool Usage

## Scope

This document explains the main project-level workflows handled by `wproj`.

## Show Help

```bash
wproj --help
```

## Initialize A Project

```bash
wproj init --work-root .
```

Bootstrap from a remote repo:

```bash
wproj init \
  --work-root /srv/wp/demo \
  --repo https://github.com/example/project-conf.git
```

## Check A Project

```bash
wproj check --work-root .
wproj check --work-root . --what wpl --fail-fast
```

## Data Operations

```bash
wproj data check --work-root .
wproj data clean --work-root .
wproj data stat --work-root .
wproj data validate --work-root .
```

## Topology Inspection

```bash
wproj model sources --work-root .
wproj model sinks --work-root .
wproj model route --work-root .
```

## Runtime Operations

```bash
wproj engine status --work-root .
wproj engine reload --work-root . --reason "manual reload"
```

For the detailed runtime workflow, see [../operations/admin.md](../operations/admin.md).

## Remote Project Sync

```bash
wproj conf update --work-root .
wproj conf update --work-root . --version 1.4.3
```

For the full sync and reload workflow, see [../operations/project-sync.md](../operations/project-sync.md).

## Rescue Statistics

```bash
wproj rescue stat --work-root .
wproj rescue stat --work-root . --detail
```

## Self Update

```bash
wproj self check
wproj self update --yes
```

## Related Docs

- CLI usage guide: [index.md](index.md)
- Runtime admin usage: [../operations/admin.md](../operations/admin.md)
- Remote project sync and reload SOP: [../operations/project-sync.md](../operations/project-sync.md)
