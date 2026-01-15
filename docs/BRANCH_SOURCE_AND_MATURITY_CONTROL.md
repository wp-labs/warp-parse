# Branch Source and Maturity Control Policy

**Version:** 1.0
**Date:** 2026-01-15
**Status:** Official

---

## Overview

This document defines the **source control** and **maturity control** policies for the three-branch strategy. These policies ensure that dependencies and code flow through the proper maturity levels before reaching production.

---

## Core Principles

### 1. Branch Source Control

**Rule:** Each branch can only receive changes from specific sources.

| Branch | Allowed Sources | Prohibited Sources |
|--------|----------------|-------------------|
| **alpha** | • Direct commits<br>• Pull requests from feature branches<br>• Dependabot PRs (any maturity) | • Merges from beta<br>• Merges from main |
| **beta** | • Merges from alpha<br>• Dependabot PRs (beta/stable only)<br>• Cherry-picks from alpha (emergency) | • Direct commits (except hotfixes)<br>• Merges from main<br>• Dependabot PRs with alpha deps |
| **main** | • Merges from beta<br>• Dependabot PRs (stable only)<br>• Hotfix branches | • Direct commits (except hotfixes)<br>• Merges from alpha<br>• Dependabot PRs with alpha/beta deps |

**Enforcement:**
- GitHub branch protection rules
- Required reviews
- Status checks (including `dependabot-branch-filter`)

---

### 2. Dependency Maturity Control

**Rule:** Each branch has strict requirements for dependency versions.

#### Alpha Branch

**Maturity Level:** Development

**Accepts:**
- ✅ `-alpha.N` versions (e.g., `v0.7.5-alpha.1`)
- ✅ `-beta.N` versions (e.g., `v0.7.5-beta.1`)
- ✅ Stable versions (e.g., `v0.7.5`)

**Rationale:** Fast iteration, accept all updates to catch issues early.

**Example:**
```toml
wp-connectors = { git = "...", tag = "v0.7.5-alpha.1" }  # ✅ OK
wp-engine = { git = "...", tag = "v1.8.3-beta.2" }      # ✅ OK
wp-specs = "0.7.6"                                       # ✅ OK
```

---

#### Beta Branch

**Maturity Level:** Pre-production Testing

**Accepts:**
- ❌ `-alpha.N` versions → **REJECT**
- ✅ `-beta.N` versions (e.g., `v0.7.5-beta.1`)
- ✅ Stable versions (e.g., `v0.7.5`)

**Rationale:** Feature-complete, no unstable alpha dependencies.

**Example:**
```toml
wp-connectors = { git = "...", tag = "v0.7.5-alpha.1" }  # ❌ REJECT
wp-connectors = { git = "...", tag = "v0.7.5-beta.2" }   # ✅ OK
wp-engine = { git = "...", tag = "v1.8.3" }              # ✅ OK
```

**Enforcement:**
- `dependabot-branch-filter` workflow auto-closes alpha PRs
- Manual review required for all dependency updates
- 1 approval required before merge

---

#### Main Branch

**Maturity Level:** Production

**Accepts:**
- ❌ `-alpha.N` versions → **REJECT**
- ❌ `-beta.N` versions → **REJECT**
- ✅ Stable versions only (e.g., `v0.7.5`)

**Rationale:** Production stability, only battle-tested dependencies.

**Example:**
```toml
wp-connectors = { git = "...", tag = "v0.7.5-alpha.1" }  # ❌ REJECT
wp-connectors = { git = "...", tag = "v0.7.5-beta.2" }   # ❌ REJECT
wp-connectors = { git = "...", tag = "v0.7.5" }          # ✅ OK
```

**Enforcement:**
- `dependabot-branch-filter` workflow auto-closes alpha/beta PRs
- Manual review required for all dependency updates
- 2 approvals required before merge
- Signed commits required

---

## Merge Flow Control

### Standard Flow (Forward Only)

```
┌─────────┐
│  alpha  │  Development (all dependencies accepted)
└────┬────┘
     │ merge (when ready for testing)
     ▼
┌─────────┐
│  beta   │  Testing (beta/stable only)
└────┬────┘
     │ merge (when ready for production)
     ▼
┌─────────┐
│  main   │  Production (stable only)
└─────────┘
```

**Rules:**
1. **Always merge forward:** alpha → beta → main
2. **Never merge backward:** main ❌→ beta, beta ❌→ alpha
3. **Review dependencies** during merge: remove alpha/beta as needed

---

### Exception: Hotfix Flow

For critical production fixes:

```
┌─────────┐
│  main   │ ← hotfix branch created here
└────┬────┘
     │ merge hotfix back
     ▼
┌─────────┐
│  main   │ ← hotfix merged, tag created
└────┬────┘
     │ sync back to beta and alpha
     ▼
┌─────────┐     ┌─────────┐
│  beta   │  ←  │  alpha  │
└─────────┘     └─────────┘
```

**Process:**
1. Create hotfix branch from main: `git checkout -b hotfix/v0.14.1 v0.14.0`
2. Fix the issue, test thoroughly
3. Merge to main: `git checkout main && git merge hotfix/v0.14.1`
4. Tag: `git tag v0.14.1`
5. Merge back: `git checkout beta && git merge main`
6. Merge back: `git checkout alpha && git merge beta`

---

## Dependabot Integration

### Configuration Philosophy

Dependabot is configured to create PRs for all branches, but **branch-specific filtering** determines which PRs are accepted.

### Branch-Specific Behavior

#### Alpha Branch Configuration

```yaml
- package-ecosystem: cargo
  target-branch: alpha
  schedule:
    interval: weekly
    day: monday
    time: "09:00"
  # No ignore rules - accept all updates
```

**Workflow Action:**
- All Dependabot PRs → ✅ Auto-approved and auto-merged

---

#### Beta Branch Configuration

```yaml
- package-ecosystem: cargo
  target-branch: beta
  schedule:
    interval: weekly
    day: monday
    time: "10:00"
  # Note: Dependabot cannot filter by Git tag suffix
  # So filtering is done by dependabot-branch-filter workflow
```

**Workflow Action:**
- Alpha dependency PR → ❌ Auto-closed with comment
- Beta dependency PR → ✅ Auto-approved and auto-merged
- Stable dependency PR → ✅ Auto-approved and auto-merged

---

#### Main Branch Configuration

```yaml
- package-ecosystem: cargo
  target-branch: main
  schedule:
    interval: weekly
    day: monday
    time: "11:00"
  # Note: Dependabot cannot filter by Git tag suffix
  # So filtering is done by dependabot-branch-filter workflow
```

**Workflow Action:**
- Alpha dependency PR → ❌ Auto-closed with comment
- Beta dependency PR → ❌ Auto-closed with comment
- Stable dependency PR → ✅ Auto-approved and auto-merged

---

## Enforcement Mechanisms

### 1. Automated Workflow

**File:** `.github/workflows/dependabot-branch-filter.yml`

**Function:**
- Triggered on all Dependabot PRs
- Extracts dependency version from PR diff
- Checks if version matches branch maturity requirements
- Auto-closes PRs that violate rules with explanatory comment
- Auto-approves and enables auto-merge for compliant PRs

**Example Auto-Close Comment:**

```markdown
### ❌ PR Closed Automatically

**Branch:** `main`
**New Version:** `v0.7.5-alpha.1`
**Reason:** Main branch only accepts stable versions

This dependency version is not appropriate for the `main` branch.

---
**Branch maturity levels:**
- `alpha`: accepts all versions (including -alpha, -beta)
- `beta`: accepts -beta and stable only
- `main`: accepts stable versions only

This version will be automatically accepted when it reaches the appropriate
maturity level on this branch, or you can manually merge it from a less
restrictive branch.
```

---

### 2. Branch Protection Rules

**GitHub Settings → Branches → Branch protection rules**

#### Required for Beta

- ✅ Require pull request reviews: 1 approval
- ✅ Require status checks: `Tests`, `filter`
- ✅ Require conversation resolution
- ✅ Do not allow bypassing settings

#### Required for Main

- ✅ Require pull request reviews: 2 approvals
- ✅ Require status checks: `Tests`, `filter`
- ✅ Require signed commits
- ✅ Require linear history
- ✅ Restrict who can push
- ✅ Do not allow bypassing settings (except for admins in emergencies)

**Critical:** The `filter` status check (from `dependabot-branch-filter.yml`) must pass before merge. This prevents manual override of maturity rules.

---

### 3. Manual Review Process

Even with automation, all dependency updates to beta and main require human review:

**Review Checklist:**

- [ ] Dependency version matches branch maturity requirements
- [ ] CHANGELOG of the dependency has been reviewed
- [ ] No breaking changes in the update
- [ ] CI tests pass
- [ ] Integration tests pass (for beta/main)
- [ ] Security vulnerabilities addressed (if applicable)

---

## Decision Matrix

### When Dependabot Creates a PR

| PR Target | Dep Version | Auto Action | Manual Action Required |
|-----------|-------------|-------------|------------------------|
| alpha     | -alpha.1    | ✅ Auto-merge | None |
| alpha     | -beta.1     | ✅ Auto-merge | None |
| alpha     | stable      | ✅ Auto-merge | None |
| beta      | -alpha.1    | ❌ Auto-close | None (already handled) |
| beta      | -beta.1     | ✅ Auto-approve | Review & merge (1 approval) |
| beta      | stable      | ✅ Auto-approve | Review & merge (1 approval) |
| main      | -alpha.1    | ❌ Auto-close | None (already handled) |
| main      | -beta.1     | ❌ Auto-close | None (already handled) |
| main      | stable      | ✅ Auto-approve | Review & merge (2 approvals) |

---

## Example Scenarios

### Scenario 1: wp-connectors Releases v0.7.6-alpha.1

**Monday 9:00 AM:**
- Dependabot detects update on alpha → Creates PR
- Workflow checks: alpha branch, -alpha version → ✅ Auto-approves
- CI passes → ✅ Auto-merges
- **Result:** Alpha branch updated automatically

**Monday 10:00 AM:**
- Dependabot detects update on beta → Creates PR
- Workflow checks: beta branch, -alpha version → ❌ Auto-closes
- **Result:** Beta branch unchanged (waiting for beta or stable release)

**Monday 11:00 AM:**
- Dependabot detects update on main → Creates PR
- Workflow checks: main branch, -alpha version → ❌ Auto-closes
- **Result:** Main branch unchanged (waiting for stable release)

---

### Scenario 2: wp-connectors Releases v0.7.6 (Stable)

**Next Monday:**
- Alpha: Already has v0.7.6-alpha.1, Dependabot detects v0.7.6 → Creates PR → ✅ Auto-merges
- Beta: Detects v0.7.6 → Creates PR → ✅ Auto-approves → **Manual review (1)** → Merges
- Main: Detects v0.7.6 → Creates PR → ✅ Auto-approves → **Manual review (2)** → Merges

**Result:** All branches updated to stable version through appropriate review process.

---

### Scenario 3: Emergency - Need Alpha Dependency on Beta

**Situation:** Beta testing requires a fix only available in alpha dependency.

**Option A - Wait for Beta Release (Recommended):**
1. Request upstream to release beta version
2. Wait for Dependabot to detect and merge

**Option B - Manual Override (Not Recommended):**
1. Temporarily relax branch protection on beta
2. Manually update `Cargo.toml`
3. Document reason in commit message
4. Create tracking issue to upgrade to beta/stable
5. Re-enable branch protection

**Option C - Cherry-pick from Alpha:**
1. Identify commit in alpha with the update
2. `git checkout beta`
3. `git cherry-pick <commit-hash>`
4. Update any alpha dependencies to beta/stable
5. Manual review and merge

---

## Troubleshooting

### Problem: Dependabot PR Not Auto-Closing on Main

**Symptoms:** PR with `-alpha` dependency stays open on main branch

**Diagnosis:**
1. Check if `dependabot-branch-filter` workflow exists on main
2. Check workflow run logs in Actions tab
3. Verify GitHub Actions permissions

**Solutions:**
1. Ensure workflow file exists on main branch
2. Check branch protection requires `filter` status check
3. Manually close PR if workflow fails

---

### Problem: Need to Override Maturity Rule

**Symptoms:** Legitimately need alpha dependency on beta for testing

**Solutions:**
1. **Best:** Request upstream beta release
2. **Acceptable:** Cherry-pick from alpha, document reason
3. **Last Resort:** Manually update with team approval

**Documentation Required:**
- Create GitHub issue explaining need
- Document in commit message
- Add TODO comment in Cargo.toml
- Plan to upgrade to stable version

---

## Maintenance

### Monthly Review

- Review all dependency versions across branches
- Check for branches with outdated alpha/beta dependencies
- Plan upgrades to stable versions
- Update this policy document if rules change

### Quarterly Audit

- Review branch protection rules still enforced
- Test dependabot-branch-filter workflow
- Verify no unauthorized dependency downgrades
- Check for security vulnerabilities in dependencies

---

## Summary Table

| Control Type | Alpha | Beta | Main | Enforcement |
|--------------|-------|------|------|-------------|
| **Accept -alpha** | ✅ | ❌ | ❌ | Workflow + Protection |
| **Accept -beta** | ✅ | ✅ | ❌ | Workflow + Protection |
| **Accept stable** | ✅ | ✅ | ✅ | Always allowed |
| **Source: Direct commits** | ✅ | ⚠️ Hotfix only | ⚠️ Hotfix only | Protection rules |
| **Source: From alpha** | N/A | ✅ Required | ❌ | Protection rules |
| **Source: From beta** | ❌ | N/A | ✅ Required | Protection rules |
| **Approvals required** | 0 | 1 | 2 | Protection rules |
| **CI checks required** | ✅ | ✅ | ✅ | Protection rules |
| **Filter check required** | N/A | ✅ | ✅ | Protection rules |

---

**Document Owner:** WarpParse Release Team
**Next Review:** 2026-04-15
**Related Documents:**
- `docs/RELEASE_MANAGEMENT.md` - Overall release strategy
- `docs/BRANCH_PROTECTION_RULES.md` - GitHub configuration guide
- `.github/workflows/dependabot-branch-filter.yml` - Enforcement workflow
