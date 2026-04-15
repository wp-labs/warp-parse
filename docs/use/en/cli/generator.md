# `wpgen` Generator Usage

## Scope

This document explains how to use `wpgen` to prepare and generate test data.

## Command Role

`wpgen` is used for:

- initializing and checking generator config
- generating test data from rules or samples

## Show Help

```bash
wpgen --help
```

## Initialize Generator Config

```bash
wpgen conf init --work-root .
wpgen conf check --work-root .
```

Recommended order:

1. run `wpgen conf init`
2. adjust `conf/wpgen.toml`
3. run `wpgen conf check`

## Generate From Rules

```bash
wpgen rule --work-root . -c wpgen.toml -n 10000 -s 5000 -p
```

## Generate From Samples

```bash
wpgen sample --work-root . -c wpgen.toml -n 5000 -p
```

## Clean Generated Data

```bash
wpgen data clean --work-root . -c wpgen.toml
```

## Related Docs

- CLI usage guide: [index.md](index.md)
- Runtime usage: [runtime.md](runtime.md)
- Project tool usage: [project.md](project.md)
