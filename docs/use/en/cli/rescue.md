# `wprescue` And Rescue Data Usage

## Scope

This document covers:

- how to use `wprescue`
- how to inspect rescue data with `wproj rescue stat`

## `wprescue`

`wprescue` is the dedicated entrypoint for rescue-data processing.

It currently supports `batch` only.

```bash
wprescue --help
wprescue batch --work-root .
```

Its common flags are similar to `wparse batch`.

## Rescue Statistics

If you only need to inspect rescue data volume or distribution, use:

```bash
wproj rescue stat --work-root .
wproj rescue stat --work-root . --detail
wproj rescue stat --work-root . --json
```

## Related Docs

- CLI usage guide: [index.md](index.md)
- Project tool usage: [project.md](project.md)
- Runtime usage: [runtime.md](runtime.md)
