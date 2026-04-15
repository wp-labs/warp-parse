# Warp Parse CLI Usage Guide

## Scope

This document explains how the current Warp Parse binaries are intended to be used in day-to-day work: local rule debugging, project checks, data generation, runtime operations, and rescue handling.

For adjacent topics:

- Product positioning: [../overview/product.md](../overview/product.md)
- Runtime admin API and reload flow: [../operations/admin.md](../operations/admin.md)
- Remote project sync and hot reload SOP: [../operations/project-sync.md](../operations/project-sync.md)

## Tool Roles

Warp Parse currently exposes four main binaries:

- `wparse`: main runtime entry
- `wpgen`: test-data generator tool
- `wproj`: project and operations tool
- `wprescue`: rescue processing entry

Quick help:

```bash
wparse --help
wpgen --help
wproj --help
wprescue --help
```

## CLI Topic Pages

- `wparse` runtime usage: [runtime.md](runtime.md)
- `wpgen` generator usage: [generator.md](generator.md)
- `wproj` project tool usage: [project.md](project.md)
- `wprescue` and rescue-data usage: [rescue.md](rescue.md)

## Recommended Flows

Local development:

1. `wproj init`
2. `wpgen conf init` and `wpgen conf check`
3. `wpgen rule` or `wpgen sample`
4. `wproj check --what wpl`
5. `wparse batch`
6. `wproj model route` and `wproj data stat`

Operations:

1. `wproj conf update`
2. `wproj check --what wpl --fail-fast`
3. `wproj engine status`
4. `wproj engine reload`
5. `wproj engine status`
6. `wproj rescue stat` if needed

## Current Boundaries

- `wparse` currently exposes `daemon` and `batch` only
- `wprescue` currently supports `batch` only
- `wproj rule` is not yet a full rule workbench
- Detailed config-field behavior should still be documented in dedicated config docs
