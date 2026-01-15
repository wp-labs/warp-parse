# Branch Protection Rules for Three-Branch Strategy

## Overview

This document describes the branch protection rules that should be configured on GitHub to enforce the three-branch strategy's dependency maturity requirements.

## Required Settings

### 1. Set Default Branch to Alpha

**Location:** Repository Settings → Branches → Default branch

**Action:** Change from `main` to `alpha`

**Reason:** All new development should start on alpha branch

---

### 2. Branch Protection Rules

#### Alpha Branch

**Location:** Repository Settings → Branches → Branch protection rules → Add rule

**Branch name pattern:** `alpha`

**Settings:**
- ✅ Require a pull request before merging
  - Required approvals: 0 (allow direct push for fast development)
- ✅ Require status checks to pass before merging
  - ✅ Require branches to be up to date before merging
  - Required checks:
    - `Tests` (from build-and-test.yml)
- ⬜ Do not require deployments to succeed before merging
- ⬜ Do not allow force pushes
- ⬜ Do not allow deletions

**Rationale:**
- Alpha is for fast development
- Basic CI checks ensure code compiles and tests pass
- No strict review requirements to maintain velocity

---

#### Beta Branch

**Branch name pattern:** `beta`

**Settings:**
- ✅ Require a pull request before merging
  - Required approvals: 1
  - ✅ Dismiss stale pull request approvals when new commits are pushed
- ✅ Require status checks to pass before merging
  - ✅ Require branches to be up to date before merging
  - Required checks:
    - `Tests` (from build-and-test.yml)
    - `filter` (from dependabot-branch-filter.yml) - **CRITICAL**
- ✅ Require conversation resolution before merging
- ⬜ Do not allow force pushes
- ⬜ Do not allow deletions
- ✅ Restrict who can push to matching branches
  - Add: Release managers only

**Rationale:**
- Beta requires review to ensure quality
- The `filter` check prevents alpha dependencies from being merged
- Only authorized personnel can merge to beta

---

#### Main Branch

**Branch name pattern:** `main`

**Settings:**
- ✅ Require a pull request before merging
  - Required approvals: 2
  - ✅ Dismiss stale pull request approvals when new commits are pushed
  - ✅ Require review from Code Owners (if CODEOWNERS file exists)
- ✅ Require status checks to pass before merging
  - ✅ Require branches to be up to date before merging
  - Required checks:
    - `Tests` (from build-and-test.yml)
    - `filter` (from dependabot-branch-filter.yml) - **CRITICAL**
- ✅ Require conversation resolution before merging
- ✅ Require signed commits
- ✅ Require linear history
- ⬜ Do not allow force pushes
- ⬜ Do not allow deletions
- ✅ Restrict who can push to matching branches
  - Add: Release managers only
- ✅ Allow only specific actors to bypass these settings
  - Add: Repository administrators (for emergency hotfixes)

**Rationale:**
- Main is production, requires maximum scrutiny
- Two approvals ensure thorough review
- The `filter` check prevents alpha and beta dependencies
- Signed commits for security and auditability

---

## Critical: Dependabot Auto-Merge Configuration

### Problem

Dependabot creates PRs before the branch protection rules can evaluate them. This means:
- Dependabot PR to main with `-alpha` dependency is created
- Branch filter workflow runs, but might not block merge
- Manual intervention required to close inappropriate PRs

### Solution 1: CODEOWNERS File (Recommended)

Create `.github/CODEOWNERS`:

```
# Default owners for everything
* @wp-labs/release-managers

# Dependency files require extra scrutiny
Cargo.toml @wp-labs/release-managers @wp-labs/tech-leads
Cargo.lock @wp-labs/release-managers @wp-labs/tech-leads

# Branch-specific dependency rules
# (Note: GitHub doesn't support branch-specific CODEOWNERS,
# so this is for documentation only)
```

### Solution 2: Dependabot Configuration Enhancement

Update `.github/dependabot.yml` to be more restrictive:

```yaml
# Main branch - add ignore rules
- package-ecosystem: cargo
  directory: "/"
  target-branch: main
  # ... existing config ...
  ignore:
    # Ignore all pre-release versions
    - dependency-name: "*"
      update-types: ["version-update:semver-prerelease"]
```

**Note:** This doesn't fully work because Dependabot can't distinguish Git tag suffixes like `-alpha` vs `-beta`.

### Solution 3: Manual PR Review Required

**Best practice for now:**

1. **Enable required reviews** on beta and main branches
2. **Required status check:** `filter` (from dependabot-branch-filter.yml)
3. **Manual review:** Any Dependabot PR to beta/main must be manually reviewed

When you see a Dependabot PR like:
```
dependabot wants to merge 1 commit into main
from dependabot/cargo/main/wp-connectors-v0.7.5-alpha.1
```

**Action:**
1. Check the version: `v0.7.5-alpha.1` contains `-alpha`
2. Close the PR with comment:
   ```
   This PR is closed because main branch does not accept alpha dependencies.

   This dependency will be available when:
   - It's released as stable (v0.7.5), OR
   - It's merged from alpha → beta → main branches

   See docs/RELEASE_MANAGEMENT.md for details.
   ```
3. The dependabot-branch-filter workflow should do this automatically once it's active on main

---

## Immediate Action Required

### For the Current Dependabot PR

Since the workflow isn't active on main yet, you need to **manually close** the PR:

```bash
# Get the PR number (e.g., #42)
# Then close it with a comment

gh pr close <PR-number> --comment "Closing: main branch does not accept alpha dependencies (v0.7.5-alpha.1).

This dependency is inappropriate for the main (production) branch.

According to our three-branch strategy:
- main: accepts only stable versions
- beta: accepts beta and stable versions
- alpha: accepts all versions

This dependency will be automatically merged to the alpha branch. It will reach main when:
1. It's released as a stable version (v0.7.5), OR
2. It's properly promoted through alpha → beta → main

See docs/RELEASE_MANAGEMENT.md for our release process."
```

### Verify Workflow on All Branches

Check that dependabot-branch-filter.yml exists on all branches:

```bash
git checkout alpha && git log --oneline -- .github/workflows/dependabot-branch-filter.yml | head -1
git checkout beta && git log --oneline -- .github/workflows/dependabot-branch-filter.yml | head -1
git checkout main && git log --oneline -- .github/workflows/dependabot-branch-filter.yml | head -1
```

All three should show the file exists.

---

## Testing the Protection

After configuring branch protection:

1. **Create a test PR** to main with an alpha dependency change
2. **Verify** that:
   - The `filter` status check runs
   - The PR is automatically closed with a comment
   - Or if auto-close fails, it's blocked from merging by required reviews

---

## Long-term Solution

Consider developing a **GitHub App** or using **Probot** to:
- Automatically close Dependabot PRs that violate branch maturity rules
- Add labels based on dependency versions
- Post helpful comments explaining why a PR was closed

Example tools:
- [Probot](https://probot.github.io/)
- [GitHub Actions Toolkit](https://github.com/actions/toolkit)

---

## Summary

| Branch | Accept Alpha | Accept Beta | Accept Stable | Required Approvals | Auto-Close Bad PRs |
|--------|--------------|-------------|---------------|--------------------|--------------------|
| alpha  | ✅           | ✅          | ✅            | 0                  | N/A                |
| beta   | ❌           | ✅          | ✅            | 1                  | ✅ (via workflow)  |
| main   | ❌           | ❌          | ✅            | 2                  | ✅ (via workflow)  |

**Critical Next Steps:**
1. ✅ Set alpha as default branch
2. ⚠️ Configure branch protection rules (do this now!)
3. ⚠️ Manually close the current bad Dependabot PR
4. ✅ Verify workflows are active on all branches
