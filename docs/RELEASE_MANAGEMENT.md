# WarpParse Release and Dependency Management Strategy

**Version:** 1.0
**Date:** 2026-01-15
**Status:** Official

---

## Table of Contents

- [1. Overview](#1-overview)
- [2. Branch Strategy](#2-branch-strategy)
- [3. Version Numbering](#3-version-numbering)
- [4. Automated Dependency Management](#4-automated-dependency-management)
- [5. Release Process](#5-release-process)
- [6. Daily Workflow](#6-daily-workflow)
- [7. Best Practices](#7-best-practices)
- [8. Troubleshooting](#8-troubleshooting)

---

## 1. Overview

### 1.1 Strategy Summary

WarpParse uses a **three-branch strategy** with automated dependency management to support three maturity levels: **alpha** (development), **beta** (testing), and **stable** (production).

```
┌─────────────────────────────────────────────────────────────┐
│                     Branch Structure                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  alpha (default)    →    beta    →    main (stable)       │
│      ↓                    ↓              ↓                  │
│  v0.14.0-alpha.1     v0.14.0-beta.1   v0.14.0             │
│  v0.14.0-alpha.2     v0.14.0-beta.2   v0.14.1             │
│      ...                 ...             ...                │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 Key Benefits

- **🤖 Automated dependency updates** - Dependabot handles updates based on branch maturity
- **🎯 Clear separation** - Each branch has explicit stability guarantees
- **⚡ Fast iteration** - Alpha branch accepts all updates automatically
- **🔒 Production safety** - Main branch only accepts stable dependencies
- **📊 Simple configuration** - No custom state files, branches define maturity

### 1.3 Design Rationale

This strategy was chosen because:

1. **Frequent dependency updates** - WarpParse depends on `wp-connectors` which updates frequently
2. **Native Dependabot support** - Uses Dependabot's built-in `target-branch` feature
3. **Explicit maturity** - Branch names clearly indicate stability level
4. **Proven workflow** - Standard Git branching model familiar to developers

---

## 2. Branch Strategy

### 2.1 Branch Roles

#### Alpha Branch (Development)

- **Purpose:** Active development and experimentation
- **Stability:** Unstable, may break
- **Dependencies:** Accepts all versions (including -alpha, -beta, stable)
- **Commits:** Frequent, daily
- **Tags:** `v0.14.0-alpha.1`, `v0.14.0-alpha.2`, ...
- **Default branch:** Yes

**Dependency Update Policy:**
```
wp-connectors v0.7.6-alpha  → ✅ Auto-merge
wp-connectors v0.7.6-beta   → ✅ Auto-merge
wp-connectors v0.7.6        → ✅ Auto-merge
```

#### Beta Branch (Testing)

- **Purpose:** Feature-complete testing and stabilization
- **Stability:** Generally stable, bugs expected
- **Dependencies:** Accepts beta and stable only (no alpha)
- **Commits:** Bug fixes only, no new features
- **Tags:** `v0.14.0-beta.1`, `v0.14.0-beta.2`, ...
- **Source:** Merged from alpha when ready for testing

**Dependency Update Policy:**
```
wp-connectors v0.7.6-alpha  → ❌ Auto-reject (close PR)
wp-connectors v0.7.6-beta   → ✅ Auto-merge
wp-connectors v0.7.6        → ✅ Auto-merge
```

#### Main Branch (Stable/Production)

- **Purpose:** Production-ready releases
- **Stability:** Highly stable, production quality
- **Dependencies:** Accepts stable versions only (no alpha or beta)
- **Commits:** Rare, only from beta merges or critical hotfixes
- **Tags:** `v0.14.0`, `v0.14.1`, ...
- **Source:** Merged from beta when ready for production

**Dependency Update Policy:**
```
wp-connectors v0.7.6-alpha  → ❌ Auto-reject (close PR)
wp-connectors v0.7.6-beta   → ❌ Auto-reject (close PR)
wp-connectors v0.7.6        → ✅ Auto-merge
```

### 2.2 Merge Direction

**Always merge forward:**
```
alpha → beta → main
```

**Never merge backward:**
```
main ❌→ beta
beta ❌→ alpha
```

**Exception:** Hotfixes can be cherry-picked from main to alpha/beta if needed.

---

## 3. Version Numbering

### 3.1 Semantic Versioning

WarpParse follows [Semantic Versioning 2.0.0](https://semver.org/):

```
MAJOR.MINOR.PATCH[-MATURITY.INCREMENT]

Examples:
  v0.14.0-alpha.1   - Alpha release
  v0.14.0-beta.2    - Beta release
  v0.14.0           - Stable release
  v0.14.1           - Patch release
```

**Version components:**
- **MAJOR:** Breaking changes (currently 0 for pre-1.0)
- **MINOR:** New features, backward compatible
- **PATCH:** Bug fixes
- **MATURITY:** alpha | beta | (none for stable)
- **INCREMENT:** Sequential number within maturity level

### 3.2 Version Progression

Typical progression for a release:

```
v0.14.0-alpha.1  → Initial alpha
v0.14.0-alpha.2  → Alpha with updates
v0.14.0-alpha.3  → More alpha iterations
v0.14.0-beta.1   → First beta (merged from alpha)
v0.14.0-beta.2   → Beta with fixes
v0.14.0          → Stable release (merged from beta)
v0.14.1          → Patch release (hotfix)
v0.15.0-alpha.1  → Next version cycle begins
```

### 3.3 Tag Naming Convention

- **Alpha:** `v{MAJOR}.{MINOR}.{PATCH}-alpha.{N}`
- **Beta:** `v{MAJOR}.{MINOR}.{PATCH}-beta.{N}`
- **Stable:** `v{MAJOR}.{MINOR}.{PATCH}`
- **Patch:** `v{MAJOR}.{MINOR}.{PATCH+1}`

---

## 4. Automated Dependency Management

### 4.1 System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Dependabot (Weekly)                      │
│  Checks for dependency updates on all branches separately  │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ├─► Alpha Branch
                 │   └─ Creates PR for wp-connectors v0.7.6-alpha
                 │
                 ├─► Beta Branch
                 │   └─ Creates PR for wp-connectors v0.7.6-alpha
                 │
                 └─► Main Branch
                     └─ Creates PR for wp-connectors v0.7.6-alpha

┌─────────────────────────────────────────────────────────────┐
│          GitHub Action: dependabot-branch-filter            │
│  Checks if dependency version matches branch requirements  │
└────────────────┬────────────────────────────────────────────┘
                 │
     ┌───────────┼───────────┐
     │           │           │
     ▼           ▼           ▼
   Alpha       Beta        Main
     │           │           │
     ├─ Check: Accept all?
     │  ✅ Yes → Auto-approve & merge
     │
     ├─ Check: Reject -alpha?
        │  ❌ Yes → Close PR with comment
        │  ✅ No  → Auto-approve & merge
                   │
                   ├─ Check: Reject -alpha and -beta?
                      │  ❌ Yes → Close PR with comment
                      │  ✅ No  → Auto-approve & merge
```

### 4.2 Configuration Files

#### `.github/dependabot.yml`

Defines Dependabot behavior for each branch:

- **Schedule:** Weekly on Monday at 9:00 AM (alpha), 10:00 AM (beta), 11:00 AM (main)
- **Target branches:** alpha, beta, main (separate configurations)
- **Grouping:** Groups wp-* dependencies together, external dependencies separately
- **Update types:** Only minor and patch updates (major requires manual review)

#### `.github/workflows/dependabot-branch-filter.yml`

Automatically filters Dependabot PRs:

- **Triggers:** On pull requests to alpha, beta, main branches
- **Checks:** Extracts dependency version from PR diff
- **Decisions:**
  - Alpha: Approves all versions
  - Beta: Approves beta and stable, closes alpha
  - Main: Approves stable only, closes alpha and beta
- **Actions:** Auto-approve and enable auto-merge, or close with explanatory comment

### 4.3 Dependency Update Flow

#### Example: wp-connectors releases v0.7.6-alpha

```
Monday 9:00 AM - Dependabot runs on alpha
  ├─ Detects wp-connectors v0.7.6-alpha
  ├─ Creates PR to alpha branch
  │
  └─ GitHub Action triggers
     ├─ Reads branch: alpha
     ├─ Checks version: v0.7.6-alpha
     ├─ Decision: ✅ Accept (alpha accepts all)
     ├─ Approves PR
     ├─ Enables auto-merge
     └─ PR merges after CI passes

Monday 10:00 AM - Dependabot runs on beta
  ├─ Detects wp-connectors v0.7.6-alpha
  ├─ Creates PR to beta branch
  │
  └─ GitHub Action triggers
     ├─ Reads branch: beta
     ├─ Checks version: v0.7.6-alpha
     ├─ Decision: ❌ Reject (beta rejects alpha)
     ├─ Comments on PR with explanation
     └─ Closes PR

Monday 11:00 AM - Dependabot runs on main
  ├─ Detects wp-connectors v0.7.6-alpha
  ├─ Creates PR to main branch
  │
  └─ GitHub Action triggers
     ├─ Reads branch: main
     ├─ Checks version: v0.7.6-alpha
     ├─ Decision: ❌ Reject (main rejects alpha)
     ├─ Comments on PR with explanation
     └─ Closes PR
```

**Result:** Only alpha branch gets the alpha dependency automatically. Beta and main wait for beta or stable versions.

---

## 5. Release Process

### 5.1 Alpha Release

**When:** Regularly during development (daily/weekly)

**Steps:**

1. **Ensure code quality:**
   ```bash
   git checkout alpha
   cargo test
   cargo check
   ```

2. **Create tag:**
   ```bash
   git tag v0.14.0-alpha.1
   git push origin alpha --tags
   ```

3. **CI/CD automatically:**
   - Builds binaries for all platforms
   - Runs integration tests
   - Creates GitHub Release (marked as pre-release)
   - Publishes Docker images

**Checklist:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Tag follows naming convention

### 5.2 Beta Release

**When:** Feature-complete, ready for broader testing

**Steps:**

1. **Merge alpha to beta:**
   ```bash
   git checkout beta
   git pull origin beta
   git merge alpha --no-ff
   ```

2. **Resolve conflicts (if any):**
   - Focus on dependency versions in `Cargo.toml` and `Cargo.lock`
   - Ensure no `-alpha` dependencies remain
   - Update to `-beta` or stable versions as needed

3. **Update dependencies:**
   ```bash
   # Review dependencies
   grep "tag =" Cargo.toml

   # Update any -alpha dependencies to -beta or stable
   # Then update lock file
   cargo update
   ```

4. **Update CHANGELOG:**
   ```bash
   vim CHANGELOG.md      # Chinese
   vim CHANGELOG.en.md   # English
   ```

5. **Test thoroughly:**
   ```bash
   cargo test
   cargo build --release

   # Integration tests
   git clone https://github.com/wp-labs/wp-examples /tmp/wp-examples
   cd /tmp/wp-examples/core
   bash run_all.sh
   ```

6. **Create tag and push:**
   ```bash
   git add -A
   git commit -m "chore: prepare beta release v0.14.0-beta.1"
   git tag v0.14.0-beta.1
   git push origin beta --tags
   ```

**Checklist:**
- [ ] Merged from alpha successfully
- [ ] No `-alpha` dependencies
- [ ] CHANGELOG updated
- [ ] All tests pass (including integration tests)
- [ ] Tag created and pushed

### 5.3 Stable Release

**When:** Thoroughly tested, production-ready

**Steps:**

1. **Merge beta to main:**
   ```bash
   git checkout main
   git pull origin main
   git merge beta --no-ff
   ```

2. **Ensure all dependencies are stable:**
   ```bash
   # Check for any pre-release dependencies
   grep -E "tag.*-(alpha|beta)" Cargo.toml

   # Should return nothing - if it does, update to stable versions
   ```

3. **Update version in Cargo.toml:**
   ```bash
   vim Cargo.toml  # Update version = "0.14.0"
   ```

4. **Finalize CHANGELOG:**
   ```bash
   vim CHANGELOG.md      # Add release date, verify entries
   vim CHANGELOG.en.md   # Same for English
   ```

5. **Full test suite:**
   ```bash
   cargo test
   cargo build --release --all-targets

   # Integration tests
   cd /tmp/wp-examples/core
   bash run_all.sh
   ```

6. **Create release:**
   ```bash
   git add -A
   git commit -m "chore: release v0.14.0"
   git tag v0.14.0
   git push origin main --tags
   ```

7. **Verify CI/CD:**
   - Wait for CI to complete
   - Check GitHub Release page
   - Verify Docker images published
   - Test installation script

8. **Start next cycle:**
   ```bash
   git checkout alpha
   git merge main  # Merge stable release back to alpha
   # Update version to next planned version
   vim Cargo.toml  # version = "0.15.0"
   git commit -am "chore: begin v0.15.0 development"
   git push origin alpha
   ```

**Checklist:**
- [ ] Merged from beta successfully
- [ ] No `-alpha` or `-beta` dependencies
- [ ] Version updated in `Cargo.toml`
- [ ] CHANGELOG finalized with release date
- [ ] All tests pass
- [ ] CI/CD pipeline completes successfully
- [ ] GitHub Release created and verified
- [ ] Docker images published
- [ ] Installation tested
- [ ] Next development cycle started

### 5.4 Hotfix Release

**When:** Critical bug in production (main branch)

**Steps:**

1. **Create hotfix branch from tag:**
   ```bash
   git checkout -b hotfix/v0.14.1 v0.14.0
   ```

2. **Fix the issue:**
   ```bash
   # Make fix
   git commit -m "fix: critical issue description"
   ```

3. **Test:**
   ```bash
   cargo test
   cargo build --release
   ```

4. **Merge to main:**
   ```bash
   git checkout main
   git merge hotfix/v0.14.1 --no-ff
   git tag v0.14.1
   git push origin main --tags
   ```

5. **Merge to beta and alpha:**
   ```bash
   git checkout beta
   git merge main
   git push origin beta

   git checkout alpha
   git merge beta
   git push origin alpha
   ```

6. **Clean up:**
   ```bash
   git branch -d hotfix/v0.14.1
   ```

---

## 6. Daily Workflow

### 6.1 For Developers

#### Daily Development

```bash
# Clone repository (first time)
git clone https://github.com/wp-labs/warp-parse.git
cd warp-parse
# Default branch is alpha

# Daily work
git checkout alpha
git pull origin alpha

# Make changes
vim src/...
cargo test

# Commit and push
git add .
git commit -m "feat: add new feature"
git push origin alpha

# Dependabot automatically handles dependency updates
# No manual intervention needed!
```

#### Working on Features

```bash
# Create feature branch from alpha
git checkout alpha
git checkout -b feature/my-feature

# Develop
git commit -m "wip: feature progress"

# Merge back to alpha when ready
git checkout alpha
git merge feature/my-feature --no-ff
git push origin alpha

# Delete feature branch
git branch -d feature/my-feature
```

### 6.2 For Release Managers

#### Weekly Maintenance

```bash
# Monday: Review Dependabot activity
gh pr list --author "dependabot[bot]"

# Check for any rejected PRs that need attention
gh pr list --label "⚠️ review-needed" --state closed

# Periodic merges to keep branches in sync (every 1-2 weeks)
git checkout beta
git merge alpha
git push origin beta

git checkout main
git merge beta
git push origin main
```

#### Release Preparation

```bash
# When ready for beta
git checkout beta
git merge alpha
# Resolve conflicts if any
git push origin beta
git tag v0.14.0-beta.1
git push --tags

# When ready for stable
git checkout main
git merge beta
# Update version, finalize CHANGELOG
git commit -am "chore: release v0.14.0"
git tag v0.14.0
git push origin main --tags
```

### 6.3 Monitoring Dependabot

#### Check Dependabot Status

```bash
# View all Dependabot PRs
gh pr list --author "dependabot[bot]"

# View auto-approved PRs
gh pr list --label "✅ auto-approved"

# View PRs that were closed (rejected)
gh pr list --label "⚠️ review-needed" --state closed --author "dependabot[bot]"

# Check Dependabot logs
# Go to: GitHub → Insights → Dependency graph → Dependabot
```

#### Manual Override

If a rejected dependency is needed:

```bash
# Option 1: Temporarily update Cargo.toml manually
git checkout beta
vim Cargo.toml  # Change dependency version
git commit -am "chore: manually update dependency X"
git push origin beta

# Option 2: Cherry-pick from alpha
git checkout beta
git cherry-pick <commit-hash-from-alpha>
git push origin beta
```

---

## 7. Best Practices

### 7.1 Branch Management

1. **Keep alpha as default branch** - All new work starts here
2. **Merge forward frequently** - Prevent large conflicts by merging alpha→beta→main weekly or bi-weekly
3. **Never skip levels** - Always merge alpha→beta→main in order
4. **Tag every release** - Even alpha releases, for traceability

### 7.2 Dependency Management

1. **Trust the automation** - Let Dependabot handle routine updates
2. **Review major versions** - Major updates require manual review even on alpha
3. **Pin critical dependencies** - Use exact versions for security-critical deps if needed
4. **Document overrides** - If manually overriding automation, document why

### 7.3 Testing

1. **Alpha:** Basic tests (unit tests, cargo check)
2. **Beta:** Full tests (unit + integration tests from wp-examples)
3. **Stable:** Comprehensive tests (all of the above + manual QA)

### 7.4 Communication

1. **CHANGELOG:** Update for every beta and stable release
2. **Release Notes:** Write detailed notes for stable releases
3. **Breaking Changes:** Clearly document in both CHANGELOG and release notes
4. **Migration Guides:** Provide for major version upgrades

### 7.5 Conflict Resolution

#### Handling Merge Conflicts

Most conflicts will be in `Cargo.lock`:

```bash
# Strategy 1: Accept version from target branch
git checkout --theirs Cargo.lock
cargo update
git add Cargo.lock

# Strategy 2: Regenerate lock file
rm Cargo.lock
cargo update
git add Cargo.lock

# Always test after resolving
cargo test
```

For `Cargo.toml` conflicts:

```bash
# Manually resolve, preferring stability
# - Beta: Use beta or stable versions
# - Main: Use only stable versions
vim Cargo.toml
cargo update
git add Cargo.toml Cargo.lock
```

---

## 8. Troubleshooting

### 8.1 Common Issues

#### Issue: Dependabot PR not auto-merging on alpha

**Symptoms:** PR created but not approved/merged

**Diagnosis:**
```bash
# Check workflow run
# Go to: Actions → Find the dependabot-branch-filter run

# Check PR labels
gh pr view <PR-number>
```

**Solutions:**
- Ensure CI checks are passing
- Check workflow logs for errors
- Verify GitHub Actions permissions
- Check if auto-merge is enabled on repository

#### Issue: Beta merge has conflicts

**Symptoms:** `git merge alpha` shows conflicts

**Diagnosis:**
```bash
git status  # Shows conflicting files
```

**Solutions:**
```bash
# For Cargo.lock conflicts
git checkout --theirs Cargo.lock
cargo update
git add Cargo.lock

# For Cargo.toml conflicts
vim Cargo.toml  # Manually resolve
cargo update
git add Cargo.toml Cargo.lock

# Complete merge
git commit
```

#### Issue: Alpha dependency on beta/main

**Symptoms:** Dependabot PRs closed immediately on beta or main

**Expected behavior:** This is correct! Beta and main should reject alpha dependencies.

**Action:**
- No action needed if on beta/main
- These dependencies will auto-merge to alpha branch
- They will be available to beta/main when released as beta or stable

#### Issue: Want to manually accept rejected dependency

**Scenario:** Beta needs an alpha dependency urgently

**Solution 1 - Cherry-pick from alpha:**
```bash
git checkout beta
# Find commit in alpha that updated the dependency
git log alpha --oneline -- Cargo.toml
git cherry-pick <commit-hash>
git push origin beta
```

**Solution 2 - Manual update:**
```bash
git checkout beta
vim Cargo.toml  # Update dependency version
cargo update
git commit -am "chore: manually update dep X to alpha version (urgent)"
git push origin beta
```

### 8.2 Emergency Procedures

#### Emergency: Alpha branch broken

```bash
# Option 1: Revert last commit
git checkout alpha
git revert HEAD
git push origin alpha

# Option 2: Reset to last known good tag
git checkout alpha
git reset --hard v0.14.0-alpha.2
git push origin alpha --force

# Option 3: Create from beta
git checkout beta
git checkout -b alpha-new
git push origin alpha-new
# Then make alpha-new the new alpha via GitHub settings
```

#### Emergency: Need to skip beta and release directly to main

**Not recommended, but if absolutely necessary:**

```bash
# Ensure alpha is thoroughly tested
git checkout alpha
cargo test
# ... extensive testing ...

# Create beta tag without merging
git tag v0.14.0-beta.1
git push --tags

# Merge to main
git checkout main
git merge alpha --no-ff
# Resolve conflicts, ensure all deps are stable
git commit -m "emergency: direct merge from alpha to main"
git tag v0.14.0
git push origin main --tags

# Sync beta
git checkout beta
git merge main
git push origin beta
```

---

## Appendix

### A. File Structure

```
warp-parse/
├── .github/
│   ├── workflows/
│   │   ├── build-and-test.yml          # CI for all branches
│   │   ├── release.yml                 # Release when tags pushed
│   │   └── dependabot-branch-filter.yml # Dependency filtering
│   └── dependabot.yml                   # Dependabot configuration
├── docs/
│   ├── RELEASE_MANAGEMENT.md           # This document
│   ├── THREE_BRANCH_STRATEGY.md        # Detailed branch strategy
│   ├── STRATEGY_COMPARISON.md          # Comparison with alternatives
│   └── ...
├── scripts/
│   ├── prepare-release.sh              # Release preparation helper
│   └── migrate-to-three-branches.sh    # Migration script
├── Cargo.toml                           # Dependency definitions
├── CHANGELOG.md                         # Chinese changelog
└── CHANGELOG.en.md                      # English changelog
```

### B. Quick Reference

#### Commands

```bash
# Check current branch
git branch --show-current

# List all tags
git tag -l

# View Dependabot PRs
gh pr list --author "dependabot[bot]"

# Merge alpha to beta
git checkout beta && git merge alpha --no-ff

# Create alpha release
git tag v0.14.0-alpha.1 && git push --tags

# Check dependency versions
grep "tag =" Cargo.toml
```

#### Branch Policies

| Branch | Accept -alpha | Accept -beta | Accept stable |
|--------|--------------|--------------|---------------|
| alpha  | ✅           | ✅           | ✅            |
| beta   | ❌           | ✅           | ✅            |
| main   | ❌           | ❌           | ✅            |

### C. References

- [Semantic Versioning](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/)
- [Dependabot Documentation](https://docs.github.com/en/code-security/dependabot)
- [GitHub Actions](https://docs.github.com/en/actions)

---

**Document Version:** 1.0
**Last Updated:** 2026-01-15
**Maintainer:** WarpParse Team
**Review Cycle:** Quarterly or as needed
