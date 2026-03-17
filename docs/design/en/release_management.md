# Warp Parse Release Management

- Version: 1.0
- Status: Official

## Overview

Warp Parse uses a three-branch release model:

- `alpha`: active development
- `beta`: pre-production testing
- `main`: stable production releases

This model is paired with dependency maturity control so that each branch accepts only the dependency stability level it is meant to carry.

## Branch Roles

### Alpha

- purpose: development and experimentation
- stability: unstable
- dependencies: accepts alpha, beta, and stable
- release tags: `vX.Y.Z-alpha.N`

### Beta

- purpose: testing and stabilization
- stability: mostly stable, bugs expected
- dependencies: accepts beta and stable only
- release tags: `vX.Y.Z-beta.N`

### Main

- purpose: production
- stability: highest
- dependencies: accepts stable only
- release tags: `vX.Y.Z`

## Merge Direction

Always merge forward:

```text
alpha -> beta -> main
```

Do not merge backward except for controlled hotfix synchronization.

## Versioning

Warp Parse follows semantic versioning with maturity suffixes:

- `v0.14.0-alpha.1`
- `v0.14.0-beta.2`
- `v0.14.0`
- `v0.14.1`

## Dependency Management

Dependabot can open PRs for all branches, but branch policies decide what is acceptable:

- alpha: auto-accept all maturity levels
- beta: reject alpha dependencies
- main: reject alpha and beta dependencies

Required guardrails:

- branch protection rules
- required status checks
- branch-specific review policy
- `dependabot-branch-filter` workflow

## Release Process

### Alpha Release

1. work on `alpha`
2. validate CI and dependency updates
3. create `vX.Y.Z-alpha.N`

### Beta Release

1. merge `alpha` into `beta`
2. remove or upgrade dependencies that are too immature for beta
3. run validation and stabilization tests
4. create `vX.Y.Z-beta.N`

### Stable Release

1. merge `beta` into `main`
2. ensure all dependencies are stable
3. run full release validation
4. create `vX.Y.Z`

### Hotfix

1. branch from `main`
2. implement and validate the fix
3. merge back to `main`
4. tag the patch release
5. sync the hotfix back to `beta` and `alpha`

## Daily Workflow

For developers:

1. start feature work from `alpha`
2. keep dependency changes aligned with branch maturity
3. promote changes forward when stable enough

For release managers:

1. review maturity before promotions
2. confirm status checks and approvals
3. tag releases from the correct branch only

## Best Practices

- do not bypass the forward promotion path
- avoid direct commits to `beta` and `main`
- treat dependency maturity as part of release quality
- keep release notes aligned with branch promotions

## Troubleshooting

- Unexpected Dependabot PR on `main`: close it if it carries pre-release dependencies
- Beta promotion blocked: check for lingering alpha dependencies
- Stable release blocked: confirm signed commits, reviews, and stable-only dependencies

## Chinese Counterpart

- [../zh-CN/release_management.md](../zh-CN/release_management.md)
