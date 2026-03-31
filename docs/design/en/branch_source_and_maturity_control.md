# Branch Source And Maturity Control

## Purpose

This policy defines:

- which branches are allowed to feed which other branches
- which dependency maturity levels each branch may accept

## Source Control Rules

### Alpha

Allowed:

- direct commits
- feature branch pull requests
- Dependabot PRs of any maturity

Blocked:

- merges from `beta`
- merges from `main`

### Beta

Allowed:

- merges from `alpha`
- beta or stable Dependabot PRs
- emergency cherry-picks when justified

Blocked:

- normal direct commits
- merges from `main`
- alpha dependency PRs

### Main

Allowed:

- merges from `beta`
- stable-only Dependabot PRs
- controlled hotfix branches

Blocked:

- normal direct commits
- merges from `alpha`
- alpha or beta dependency PRs

## Dependency Maturity Rules

- `alpha` accepts alpha, beta, stable
- `beta` accepts beta, stable
- `main` accepts stable only

This rule applies to both direct code changes and automated dependency updates.

## Merge Flow

Standard flow:

```text
alpha -> beta -> main
```

Exception flow:

- critical hotfix starts from `main`
- after release, the fix is synchronized back to `beta` and `alpha`

## Enforcement

Use the following together:

- branch protection
- reviews
- status checks
- `dependabot-branch-filter`

If any one layer is missing, policy drift becomes likely.

## Operating Guidance

- review dependency tags during promotion
- do not merge unstable dependencies upward by accident
- treat branch source and dependency maturity as one policy, not two unrelated rules

## Chinese Counterpart

- [../zh-CN/branch_source_and_maturity_control.md](../zh-CN/branch_source_and_maturity_control.md)
