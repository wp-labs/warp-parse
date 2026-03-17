# Branch Protection Rules

## Goal

These rules enforce the three-branch strategy on GitHub and prevent dependencies from bypassing their intended maturity path.

## Default Branch

Set the default branch to `alpha` so new development starts from the development lane.

## Alpha Rules

- require pull requests before merge
- approvals required: `0`
- require CI checks to pass
- keep force-push and deletion disabled

Rationale:

- maintain development speed
- still require build and test coverage

## Beta Rules

- require pull requests before merge
- approvals required: `1`
- dismiss stale approvals on new commits
- require CI checks and `dependabot-branch-filter`
- require conversations to be resolved
- restrict push rights to release managers

Rationale:

- beta is the stabilization lane
- alpha-grade dependencies must be blocked here

## Main Rules

- require pull requests before merge
- approvals required: `2`
- require code-owner review when applicable
- require CI checks and `dependabot-branch-filter`
- require signed commits
- require linear history
- restrict push rights to release managers
- allow limited bypass only for emergency administrators

Rationale:

- main is production
- dependency maturity and auditability matter most here

## Dependabot Handling

Dependabot can still create PRs before branch policy rejects them. Use:

- required reviews
- required `filter` status
- manual review for unusual cases

If a PR targets `beta` or `main` with a lower maturity dependency, close it with a short policy note.

## Immediate Setup Checklist

1. set default branch to `alpha`
2. configure protection for `alpha`, `beta`, and `main`
3. require the `filter` workflow on `beta` and `main`
4. restrict push rights for release branches
5. verify the workflow exists on all branches

## Chinese Counterpart

- [../zh-CN/branch_protection_rules.md](../zh-CN/branch_protection_rules.md)
