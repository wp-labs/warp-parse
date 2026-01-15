# Three-Branch Release Strategy

[English](#english) | [中文](#中文)

---

## English

### Overview

WarpParse uses a **three-branch strategy** with automated dependency management:

```
alpha (development)  →  beta (testing)  →  main (stable)
   ↓                       ↓                   ↓
 -alpha tags            -beta tags         stable tags
```

### Branch Strategy

```
alpha (default branch for development)
  ├── Accepts all dependency versions
  ├── Fast iteration, frequent commits
  ├── Tags: v0.14.0-alpha.1, v0.14.0-alpha.2, ...
  └── Auto-merges: ALL Dependabot PRs
      │
      ├─ (merge when ready for testing)
      ▼
beta (testing and stabilization)
  ├── Accepts beta and stable versions only
  ├── Feature freeze, bug fixes only
  ├── Tags: v0.14.0-beta.1, v0.14.0-beta.2, ...
  └── Auto-merges: beta and stable Dependabot PRs
      │
      ├─ (merge when ready for release)
      ▼
main (production-ready)
  ├── Accepts stable versions only
  ├── Release branch, always stable
  ├── Tags: v0.14.0, v0.14.1, ...
  └── Auto-merges: stable Dependabot PRs only
```

### Key Principles

1. **Default branch: `alpha`** - All development happens here
2. **Merge direction: alpha → beta → main** - Always forward
3. **Dependabot per branch** - Different rules per branch
4. **Tags on all branches** - For release tracking

### Workflow

#### Daily Development (on alpha)

```bash
# Clone and work on alpha (default branch)
git clone https://github.com/wp-labs/warp-parse.git
cd warp-parse
git checkout alpha  # (already on alpha by default)

# Make changes
git add .
git commit -m "feat: add new feature"
git push origin alpha

# Dependabot automatically updates wp-connectors on alpha branch
# → All versions accepted and auto-merged
```

#### Alpha Release

```bash
# On alpha branch
git checkout alpha
git tag v0.14.0-alpha.1
git push origin v0.14.0-alpha.1

# CI builds and publishes alpha release
```

#### Promote to Beta

```bash
# Merge alpha to beta when ready for testing
git checkout beta
git merge alpha --no-ff -m "chore: merge alpha to beta for v0.14.0-beta.1"

# Review and update dependencies if needed
# (Remove any -alpha dependencies, use -beta or stable)
vim Cargo.toml

# Tag beta release
git tag v0.14.0-beta.1
git push origin beta --tags

# Dependabot now manages beta branch separately
# → Only beta and stable versions auto-merged
```

#### Promote to Stable (main)

```bash
# Merge beta to main when ready for production
git checkout main
git merge beta --no-ff -m "chore: merge beta to main for v0.14.0"

# Ensure all dependencies are stable
# (No -alpha or -beta versions)
vim Cargo.toml

# Tag stable release
git tag v0.14.0
git push origin main --tags

# Dependabot now manages main branch separately
# → Only stable versions auto-merged

# Start next cycle: back to alpha
git checkout alpha
```

### Dependency Management

#### Dependabot Configuration Per Branch

Dependabot runs separately on each branch with different rules:

**On `alpha` branch:**
- Accepts: ALL versions (including -alpha, -beta)
- Auto-merge: YES (all updates)
- Purpose: Fast iteration

**On `beta` branch:**
- Accepts: -beta and stable only
- Rejects: -alpha versions
- Auto-merge: YES (for accepted versions)
- Purpose: Stabilization

**On `main` branch:**
- Accepts: Stable only
- Rejects: -alpha and -beta versions
- Auto-merge: YES (for stable versions)
- Purpose: Production stability

#### wp-connectors Frequent Updates

```
Time: Monday 9:00 AM (Dependabot runs)

wp-connectors releases v0.7.6-alpha
    │
    ├─► alpha branch
    │   └─ Dependabot creates PR
    │      └─ ✅ Auto-merged (accepts -alpha)
    │
    ├─► beta branch
    │   └─ Dependabot creates PR
    │      └─ ⚠️ Review needed (rejects -alpha)
    │
    └─► main branch
        └─ Dependabot creates PR
           └─ ⚠️ Review needed (rejects -alpha)

wp-connectors releases v0.7.6-beta
    │
    ├─► alpha branch
    │   └─ ✅ Auto-merged
    │
    ├─► beta branch
    │   └─ ✅ Auto-merged
    │
    └─► main branch
        └─ ⚠️ Review needed (rejects -beta)

wp-connectors releases v0.7.6 (stable)
    │
    ├─► alpha branch → ✅ Auto-merged
    ├─► beta branch  → ✅ Auto-merged
    └─► main branch  → ✅ Auto-merged
```

### Handling Merge Conflicts

#### Strategy 1: Frequent Merges (Recommended)

Merge alpha → beta → main frequently to avoid conflicts:

```bash
# Weekly or bi-weekly merge
git checkout beta
git merge alpha
git push origin beta

git checkout main
git merge beta
git push origin main
```

#### Strategy 2: Dependency-Only Merges

When dependencies diverge, cherry-pick dependency updates:

```bash
# If beta needs an alpha dependency update
git checkout beta
git cherry-pick <commit-hash-from-alpha>
git push origin beta
```

#### Strategy 3: Conflict Resolution

If conflicts occur during merge:

```bash
git checkout beta
git merge alpha
# CONFLICT in Cargo.toml

# Resolve manually: keep beta-appropriate versions
vim Cargo.toml
git add Cargo.toml
git commit -m "chore: merge alpha to beta, resolve dependency conflicts"
git push origin beta
```

### Release Checklist

#### Alpha Release
- [ ] All tests pass on alpha branch
- [ ] Create tag: `v0.14.0-alpha.N`
- [ ] Push tag to trigger CI/CD

#### Beta Release
- [ ] Merge alpha to beta
- [ ] Update any -alpha dependencies to -beta or stable
- [ ] All integration tests pass
- [ ] Create tag: `v0.14.0-beta.N`
- [ ] Update CHANGELOG

#### Stable Release
- [ ] Merge beta to main
- [ ] Ensure all dependencies are stable (no -alpha or -beta)
- [ ] Full CI/CD pipeline passes
- [ ] Create tag: `v0.14.0`
- [ ] Finalize CHANGELOG
- [ ] Publish release notes

---

## 中文

### 概述

WarpParse 使用**三分支策略**配合自动化依赖管理：

```
alpha (开发)  →  beta (测试)  →  main (稳定)
   ↓               ↓                ↓
 -alpha 标签     -beta 标签      stable 标签
```

### 分支策略

```
alpha (开发默认分支)
  ├── 接受所有依赖版本
  ├── 快速迭代，频繁提交
  ├── 标签: v0.14.0-alpha.1, v0.14.0-alpha.2, ...
  └── 自动合并: 所有 Dependabot PR
      │
      ├─ (准备测试时合并)
      ▼
beta (测试和稳定化)
  ├── 仅接受 beta 和稳定版本
  ├── 功能冻结，仅修复 bug
  ├── 标签: v0.14.0-beta.1, v0.14.0-beta.2, ...
  └── 自动合并: beta 和稳定版本的 Dependabot PR
      │
      ├─ (准备发布时合并)
      ▼
main (生产就绪)
  ├── 仅接受稳定版本
  ├── 发布分支，始终稳定
  ├── 标签: v0.14.0, v0.14.1, ...
  └── 自动合并: 仅稳定版本的 Dependabot PR
```

### 核心原则

1. **默认分支: `alpha`** - 所有开发在此进行
2. **合并方向: alpha → beta → main** - 始终向前
3. **分支独立的 Dependabot** - 每个分支不同规则
4. **所有分支打标签** - 用于发布跟踪

### 工作流程

#### 日常开发（在 alpha）

```bash
# 克隆并在 alpha 分支工作（默认分支）
git clone https://github.com/wp-labs/warp-parse.git
cd warp-parse
git checkout alpha  # (默认已在 alpha)

# 进行修改
git add .
git commit -m "feat: add new feature"
git push origin alpha

# Dependabot 自动在 alpha 分支更新 wp-connectors
# → 所有版本都被接受并自动合并
```

### wp-connectors 频繁更新处理

```
时间: 周一上午 9:00 (Dependabot 运行)

wp-connectors 发布 v0.7.6-alpha
    │
    ├─► alpha 分支
    │   └─ Dependabot 创建 PR
    │      └─ ✅ 自动合并 (接受 -alpha)
    │
    ├─► beta 分支
    │   └─ Dependabot 创建 PR
    │      └─ ⚠️ 需要审查 (拒绝 -alpha)
    │
    └─► main 分支
        └─ Dependabot 创建 PR
           └─ ⚠️ 需要审查 (拒绝 -alpha)

wp-connectors 发布 v0.7.6 (稳定版)
    │
    ├─► alpha 分支 → ✅ 自动合并
    ├─► beta 分支  → ✅ 自动合并
    └─► main 分支  → ✅ 自动合并
```
