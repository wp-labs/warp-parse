# Release Strategy Comparison

[English](#english) | [中文](#中文)

---

## English

### Overview

WarpParse has two viable release strategies. This document helps you choose the best one for your needs.

## Strategy Comparison

### Option 1: Single Main Branch + Tags ⚙️

**Structure:**
```
main (single branch)
  ├── v0.14.0-alpha.1  (tag)
  ├── v0.14.0-alpha.2  (tag)
  ├── v0.14.0-beta.1   (tag)
  └── v0.14.0          (tag)
```

**Files needed:**
- `.dev-stage.yml` (stage configuration)
- `.github/workflows/dependabot-auto-merge.yml` (custom automation)
- `.github/dependabot.yml` (standard)

**Pros:**
- ✅ Simple branch management (only one branch)
- ✅ No merge conflicts between branches
- ✅ Follows Rust ecosystem convention (semantic versioning)
- ✅ Linear git history
- ✅ Easy to understand: one branch, many tags

**Cons:**
- ❌ Requires custom GitHub Actions workflow
- ❌ Need to manage `.dev-stage.yml` file
- ❌ More complex automation logic
- ❌ Main branch may be unstable at times

**Best for:**
- Projects with simple release cycles
- Teams that prefer minimal branch management
- When you want linear history

---

### Option 2: Three Branches (alpha/beta/main) 🌳

**Structure:**
```
alpha (development) → beta (testing) → main (stable)
  ↓                     ↓                 ↓
-alpha tags          -beta tags       stable tags
```

**Files needed:**
- `.github/dependabot-three-branch.yml` (rename to dependabot.yml)
- `.github/workflows/dependabot-branch-filter.yml` (simple filter)

**Pros:**
- ✅ Simpler automation (native Dependabot features)
- ✅ No extra configuration files needed
- ✅ Branch name = maturity level (clear and explicit)
- ✅ Can work on different maturity levels in parallel
- ✅ Easier to reason about: "I'm on beta, so I need beta deps"
- ✅ Main branch is always stable

**Cons:**
- ❌ Need to merge between branches (alpha → beta → main)
- ❌ Potential merge conflicts
- ❌ More branches to maintain
- ❌ Slightly more complex git workflow

**Best for:**
- **Projects with frequent dependency updates** ⭐ (like yours!)
- Teams with parallel work on different maturity levels
- When you want guaranteed stability on main branch
- When you prefer explicit branch-based workflows

---

## Detailed Comparison

### 1. Handling wp-connectors Frequent Updates

#### Single Branch + Tags
```bash
# wp-connectors releases v0.7.6-alpha

# Current stage: alpha (from .dev-stage.yml)
Dependabot creates PR → GitHub Action checks .dev-stage.yml
→ stage=alpha → ✅ Auto-merge

# Change to beta
./scripts/prepare-release.sh 0.14.0 beta  # Updates .dev-stage.yml

# wp-connectors releases v0.7.7-alpha
Dependabot creates PR → GitHub Action checks .dev-stage.yml
→ stage=beta → ❌ Review needed
```

**Complexity:** Medium (custom workflow needed)

#### Three Branches
```bash
# wp-connectors releases v0.7.6-alpha

# On alpha branch
Dependabot creates PR → ✅ Auto-merge (alpha accepts all)

# On beta branch
Dependabot creates PR → ❌ Auto-close (beta rejects alpha)

# On main branch
Dependabot creates PR → ❌ Auto-close (main rejects alpha)

# No configuration file changes needed!
```

**Complexity:** Low (native branch filtering)

---

### 2. Release Process

#### Single Branch + Tags
```bash
# Alpha release
./scripts/prepare-release.sh 0.14.0 alpha
git add .dev-stage.yml
git commit -m "chore: start alpha"
git tag v0.14.0-alpha.1
git push --tags

# Beta release
./scripts/prepare-release.sh 0.14.0 beta
git add .dev-stage.yml Cargo.toml
git commit -m "chore: transition to beta"
git tag v0.14.0-beta.1
git push --tags

# Stable release
./scripts/prepare-release.sh 0.14.0 stable
git add .dev-stage.yml Cargo.toml
git commit -m "chore: prepare stable"
git tag v0.14.0
git push --tags
```

**Steps:** 3 releases, 3 config file updates

#### Three Branches
```bash
# Alpha release (on alpha branch)
git checkout alpha
git tag v0.14.0-alpha.1
git push origin alpha --tags

# Beta release
git checkout beta
git merge alpha  # May need to resolve dependency versions
git tag v0.14.0-beta.1
git push origin beta --tags

# Stable release
git checkout main
git merge beta  # May need to resolve dependency versions
git tag v0.14.0
git push origin main --tags
```

**Steps:** 3 releases, 0 config file updates (just git operations)

---

### 3. Developer Experience

#### Single Branch + Tags

**For new contributors:**
```bash
git clone repo
# Always on main branch
# Need to check .dev-stage.yml to know current maturity
cat .dev-stage.yml  # stage: alpha
```

**Daily work:**
```bash
# All work on main
git checkout main
git pull
# Make changes
git push
```

**Simplicity:** ⭐⭐⭐⭐⭐ (very simple - one branch)

#### Three Branches

**For new contributors:**
```bash
git clone repo
# On alpha branch by default
# Branch name tells you the maturity level
git branch  # * alpha
```

**Daily work:**
```bash
# Work on alpha (development)
git checkout alpha
git pull
# Make changes
git push

# Test on beta
git checkout beta
git merge alpha
git push

# Release on main
git checkout main
git merge beta
git push
```

**Simplicity:** ⭐⭐⭐⭐ (clear, but more branches)

---

### 4. Merge Conflicts

#### Single Branch + Tags
- **Frequency:** Never (no branch merges)
- **Risk:** ⭐ (very low)

#### Three Branches
- **Frequency:** Potentially on every alpha→beta→main merge
- **Risk:** ⭐⭐⭐ (medium, especially with Cargo.lock)
- **Mitigation:**
  - Merge frequently (weekly)
  - Use automated merge scripts
  - Accept that some manual resolution is needed

---

### 5. Automation Complexity

#### Single Branch + Tags

**Automation files:**
1. `.dev-stage.yml` - 30 lines (configuration)
2. `.github/workflows/dependabot-auto-merge.yml` - 180 lines (complex logic)
3. `scripts/prepare-release.sh` - 200 lines (helper script)

**Total complexity:** ~410 lines, custom logic

#### Three Branches

**Automation files:**
1. `.github/dependabot-three-branch.yml` - 120 lines (standard Dependabot)
2. `.github/workflows/dependabot-branch-filter.yml` - 140 lines (simple filter)

**Total complexity:** ~260 lines, simpler logic

---

## Recommendation

### For WarpParse (with frequent wp-connectors updates): 🌳 Three Branches

**Why:**

1. **Simpler automation** - No custom stage management, branches speak for themselves
2. **Native Dependabot** - Leverages built-in `target-branch` feature
3. **Clear separation** - Alpha, beta, and main are explicitly different
4. **Parallel work** - Can have alpha development while beta is in testing
5. **Main is stable** - Production branch never breaks

**Trade-off:** You'll need to merge branches periodically, but this is a well-understood Git workflow.

### Migration Path from Current Single Branch

If you decide to switch to three branches:

```bash
# 1. Create branches from current main
git checkout main
git checkout -b alpha
git checkout -b beta

# 2. Set alpha as default branch on GitHub
# Settings → Branches → Default branch → alpha

# 3. Replace dependabot.yml
mv .github/dependabot-three-branch.yml .github/dependabot.yml

# 4. Add branch filter workflow
# (already created: dependabot-branch-filter.yml)

# 5. Remove single-branch files
rm .dev-stage.yml
rm .github/workflows/dependabot-auto-merge.yml
rm docs/AUTOMATED_DEPENDENCY_MANAGEMENT.md

# 6. Push all branches
git push origin alpha beta main
```

---

## 中文

### 概述

WarpParse 有两种可行的发布策略。本文档帮助你选择最适合的方案。

## 策略对比

### 方案 1: 单主分支 + 标签 ⚙️

**结构:**
```
main (单一分支)
  ├── v0.14.0-alpha.1  (标签)
  ├── v0.14.0-alpha.2  (标签)
  ├── v0.14.0-beta.1   (标签)
  └── v0.14.0          (标签)
```

**优点:**
- ✅ 分支管理简单（只有一个分支）
- ✅ 分支间无合并冲突
- ✅ 符合 Rust 生态约定（语义化版本）
- ✅ 线性 git 历史
- ✅ 易于理解：一个分支，多个标签

**缺点:**
- ❌ 需要自定义 GitHub Actions workflow
- ❌ 需要管理 `.dev-stage.yml` 文件
- ❌ 自动化逻辑较复杂
- ❌ Main 分支有时可能不稳定

**最适合:**
- 发布周期简单的项目
- 偏好最小化分支管理的团队
- 想要线性历史记录时

---

### 方案 2: 三分支 (alpha/beta/main) 🌳

**结构:**
```
alpha (开发) → beta (测试) → main (稳定)
  ↓              ↓              ↓
-alpha 标签   -beta 标签   stable 标签
```

**优点:**
- ✅ 自动化更简单（原生 Dependabot 特性）
- ✅ 无需额外配置文件
- ✅ 分支名 = 成熟度级别（清晰明确）
- ✅ 可并行处理不同成熟度级别
- ✅ 更容易推理："我在 beta，所以需要 beta 依赖"
- ✅ Main 分支始终稳定

**缺点:**
- ❌ 需要在分支间合并（alpha → beta → main）
- ❌ 潜在的合并冲突
- ❌ 需要维护更多分支
- ❌ Git 工作流略复杂

**最适合:**
- **依赖更新频繁的项目** ⭐（比如你的项目！）
- 需要并行处理不同成熟度级别的团队
- 想要 main 分支保证稳定时
- 偏好显式的基于分支的工作流

---

## 推荐

### 对于 WarpParse（wp-connectors 频繁更新）：🌳 三分支方案

**原因：**

1. **自动化更简单** - 无需自定义阶段管理，分支本身就说明一切
2. **原生 Dependabot** - 利用内置的 `target-branch` 特性
3. **清晰分离** - Alpha、beta 和 main 明确不同
4. **并行工作** - Beta 测试时可以继续 alpha 开发
5. **Main 稳定** - 生产分支永不损坏

**权衡：** 需要定期合并分支，但这是成熟的 Git 工作流。
