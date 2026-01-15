# Automated Dependency Management

[English](#english) | [中文](#中文)

---

## English

### Overview

WarpParse uses an automated dependency management system that combines **Dependabot** with **GitHub Actions** to automatically review and merge dependency updates based on the current development stage.

### How It Works

```
┌─────────────────────────────────────────────────────────────┐
│  1. Developer updates .dev-stage.yml                        │
│     stage: alpha → beta → stable                            │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│  2. Dependabot detects new dependency versions              │
│     (runs weekly on Monday at 9:00 AM)                      │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│  3. Dependabot creates PR with dependency update            │
│     e.g., Update wp-connectors from v0.7.5 to v0.7.6       │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│  4. GitHub Action: dependabot-auto-merge.yml triggers       │
│     - Reads .dev-stage.yml                                  │
│     - Checks new dependency version                         │
│     - Applies stage-specific rules                          │
└─────────────────────────┬───────────────────────────────────┘
                          │
                ┌─────────┴─────────┐
                │                   │
    ┌───────────▼──────────┐  ┌────▼──────────────┐
    │  5a. Auto-approve    │  │  5b. Review       │
    │      + Auto-merge    │  │      required     │
    │                      │  │                   │
    │  • Adds ✅ label    │  │  • Adds ⚠️ label │
    │  • Approves PR       │  │  • Adds comment   │
    │  • Enables auto-merge│  │  • Waits for      │
    │  • Merges after CI   │  │    manual review  │
    └──────────────────────┘  └───────────────────┘
```

### Stage-Based Rules

#### Alpha Stage
**Goal:** Fast iteration, accept all updates

```yaml
stage: alpha
```

**Rules:**
- ✅ Accept all dependency versions
- ✅ Accept `-alpha` versions
- ✅ Accept `-beta` versions
- ✅ Accept stable versions

**Example:**
```toml
wp-connectors = { git = "...", tag = "v0.7.6-alpha" }  # ✅ Accepted
wp-engine = { git = "...", tag = "v1.8.3-alpha" }      # ✅ Accepted
```

#### Beta Stage
**Goal:** Stabilization, no alpha dependencies

```yaml
stage: beta
```

**Rules:**
- ❌ Reject `-alpha` versions
- ✅ Accept `-beta` versions
- ✅ Accept stable versions

**Example:**
```toml
wp-connectors = { git = "...", tag = "v0.7.6-alpha" }  # ❌ Rejected
wp-connectors = { git = "...", tag = "v0.7.6-beta" }   # ✅ Accepted
wp-engine = { git = "...", tag = "v1.8.3" }            # ✅ Accepted
```

#### Stable Stage
**Goal:** Production-ready, stable dependencies only

```yaml
stage: stable
```

**Rules:**
- ❌ Reject `-alpha` versions
- ❌ Reject `-beta` versions
- ✅ Accept stable versions only

**Example:**
```toml
wp-connectors = { git = "...", tag = "v0.7.6-beta" }   # ❌ Rejected
wp-connectors = { git = "...", tag = "v0.7.6" }        # ✅ Accepted
```

### Usage Examples

#### Scenario 1: Starting Alpha Development

```bash
# Update stage to alpha
./scripts/prepare-release.sh 0.14.0 alpha

# Commit the change
git add .dev-stage.yml
git commit -m "chore: start alpha development for v0.14.0"
git push origin main

# From now on, Dependabot will auto-merge all dependency updates
```

**What happens next:**
- Dependabot creates PR: "Update wp-connectors to v0.7.6-alpha"
- GitHub Action checks: stage=alpha, version=v0.7.6-alpha
- Decision: ✅ Auto-approve (alpha accepts all versions)
- PR is automatically approved and merged after CI passes

#### Scenario 2: Transitioning to Beta

```bash
# Update stage to beta
./scripts/prepare-release.sh 0.14.0 beta

# The script checks if any dependencies are alpha versions
# If found, it will warn you to update them

# Manually update alpha dependencies to beta or stable in Cargo.toml
# e.g., change v0.7.6-alpha to v0.7.6-beta

# Commit the changes
git add .dev-stage.yml Cargo.toml
git commit -m "chore: transition to beta stage"
git push origin main

# From now on, Dependabot will only auto-merge beta and stable updates
```

**What happens next:**
- Dependabot creates PR: "Update wp-connectors to v0.7.7-alpha"
- GitHub Action checks: stage=beta, version=v0.7.7-alpha
- Decision: ⚠️ Review needed (beta rejects alpha)
- PR is labeled for manual review

- Dependabot creates PR: "Update wp-engine to v1.8.3-beta"
- GitHub Action checks: stage=beta, version=v1.8.3-beta
- Decision: ✅ Auto-approve (beta accepts beta versions)
- PR is automatically approved and merged

#### Scenario 3: Preparing Stable Release

```bash
# Update stage to stable
./scripts/prepare-release.sh 0.14.0 stable

# The script checks if any dependencies are alpha/beta versions
# If found, it will error and require you to update them

# Update all dependencies to stable versions in Cargo.toml

# Commit and create release
git add .dev-stage.yml Cargo.toml
git commit -m "chore: prepare stable release v0.14.0"
git tag v0.14.0
git push origin main --tags

# After release, switch back to alpha for next development cycle
./scripts/prepare-release.sh 0.15.0 alpha
git add .dev-stage.yml
git commit -m "chore: start alpha development for v0.15.0"
git push origin main
```

### Manual Overrides

If you need to merge a dependency that was rejected by the automation:

1. **Option A: Temporarily change stage**
   ```bash
   # Change to more permissive stage
   sed -i '' 's/stage: stable/stage: beta/' .dev-stage.yml
   git add .dev-stage.yml
   git commit -m "chore: temporarily allow beta dependencies"

   # Wait for Dependabot to re-run or manually trigger

   # Change back after merge
   sed -i '' 's/stage: beta/stage: stable/' .dev-stage.yml
   git add .dev-stage.yml
   git commit -m "chore: restore stable stage"
   ```

2. **Option B: Manually approve and merge**
   - Review the PR manually
   - Approve and merge through GitHub UI
   - The automation will not interfere with manual reviews

### Configuration Files

#### `.dev-stage.yml`
Defines the current development stage and target version.

```yaml
stage: alpha
next_version: 0.14.0
```

Update this file using:
- `./scripts/prepare-release.sh <version> <stage>`
- Or manually edit and commit

#### `.github/workflows/dependabot-auto-merge.yml`
GitHub Action that implements the auto-merge logic. You typically don't need to modify this.

#### `.github/dependabot.yml`
Configures Dependabot's update schedule and grouping. The automation works with any Dependabot configuration.

### Monitoring

#### Check Dependabot Activity
```bash
# List all Dependabot PRs
gh pr list --author "dependabot[bot]"

# Check auto-approved PRs
gh pr list --label "✅ auto-approved"

# Check PRs needing review
gh pr list --label "⚠️ review-needed"
```

#### Check Current Stage
```bash
# View current stage
cat .dev-stage.yml

# View git history of stage changes
git log --oneline -- .dev-stage.yml
```

### Troubleshooting

#### Problem: Dependabot PR not auto-merging

**Check:**
1. Is the workflow running? → Check Actions tab
2. What's the current stage? → `cat .dev-stage.yml`
3. What's the dependency version? → Check PR diff
4. What was the decision? → Check workflow logs

**Common issues:**
- CI checks not passing → PR won't auto-merge until CI passes
- Stage mismatch → Update `.dev-stage.yml` or manually review
- Auto-merge not enabled on repository → Enable in Settings → General

#### Problem: Want to disable auto-merge temporarily

**Solution 1: Add Dependabot to ignore**
Edit `.github/dependabot.yml`:
```yaml
ignore:
  - dependency-name: "wp-connectors"
    versions: ["*"]  # Temporarily ignore all updates
```

**Solution 2: Disable workflow**
Rename the workflow:
```bash
mv .github/workflows/dependabot-auto-merge.yml \
   .github/workflows/dependabot-auto-merge.yml.disabled
```

---

## 中文

### 概述

WarpParse 使用自动化依赖管理系统，结合 **Dependabot** 和 **GitHub Actions**，根据当前开发阶段自动审查和合并依赖更新。

### 工作原理

```
┌─────────────────────────────────────────────────────────────┐
│  1. 开发者更新 .dev-stage.yml                               │
│     stage: alpha → beta → stable                            │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│  2. Dependabot 检测到新的依赖版本                          │
│     (每周一上午 9:00 运行)                                 │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│  3. Dependabot 创建依赖更新 PR                             │
│     例如: Update wp-connectors from v0.7.5 to v0.7.6      │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│  4. GitHub Action: dependabot-auto-merge.yml 触发          │
│     - 读取 .dev-stage.yml                                  │
│     - 检查新依赖版本                                        │
│     - 应用特定阶段的规则                                    │
└─────────────────────────┬───────────────────────────────────┘
                          │
                ┌─────────┴─────────┐
                │                   │
    ┌───────────▼──────────┐  ┌────▼──────────────┐
    │  5a. 自动批准        │  │  5b. 需要审查     │
    │      + 自动合并      │  │                   │
    │                      │  │                   │
    │  • 添加 ✅ 标签     │  │  • 添加 ⚠️ 标签  │
    │  • 批准 PR           │  │  • 添加评论       │
    │  • 启用自动合并      │  │  • 等待手动审查   │
    │  • CI 通过后合并     │  │                   │
    └──────────────────────┘  └───────────────────┘
```

### 基于阶段的规则

#### Alpha 阶段
**目标：** 快速迭代，接受所有更新

```yaml
stage: alpha
```

**规则：**
- ✅ 接受所有依赖版本
- ✅ 接受 `-alpha` 版本
- ✅ 接受 `-beta` 版本
- ✅ 接受稳定版本

**示例：**
```toml
wp-connectors = { git = "...", tag = "v0.7.6-alpha" }  # ✅ 接受
wp-engine = { git = "...", tag = "v1.8.3-alpha" }      # ✅ 接受
```

#### Beta 阶段
**目标：** 稳定化，不接受 alpha 依赖

```yaml
stage: beta
```

**规则：**
- ❌ 拒绝 `-alpha` 版本
- ✅ 接受 `-beta` 版本
- ✅ 接受稳定版本

**示例：**
```toml
wp-connectors = { git = "...", tag = "v0.7.6-alpha" }  # ❌ 拒绝
wp-connectors = { git = "...", tag = "v0.7.6-beta" }   # ✅ 接受
wp-engine = { git = "...", tag = "v1.8.3" }            # ✅ 接受
```

#### Stable 阶段
**目标：** 生产就绪，仅稳定依赖

```yaml
stage: stable
```

**规则：**
- ❌ 拒绝 `-alpha` 版本
- ❌ 拒绝 `-beta` 版本
- ✅ 仅接受稳定版本

**示例：**
```toml
wp-connectors = { git = "...", tag = "v0.7.6-beta" }   # ❌ 拒绝
wp-connectors = { git = "...", tag = "v0.7.6" }        # ✅ 接受
```

### 使用示例

#### 场景 1: 开始 Alpha 开发

```bash
# 更新阶段为 alpha
./scripts/prepare-release.sh 0.14.0 alpha

# 提交更改
git add .dev-stage.yml
git commit -m "chore: start alpha development for v0.14.0"
git push origin main

# 从现在开始，Dependabot 会自动合并所有依赖更新
```

**后续发生的事情：**
- Dependabot 创建 PR: "Update wp-connectors to v0.7.6-alpha"
- GitHub Action 检查: stage=alpha, version=v0.7.6-alpha
- 决策: ✅ 自动批准 (alpha 接受所有版本)
- CI 通过后 PR 自动批准并合并

#### 场景 2: 过渡到 Beta

```bash
# 更新阶段为 beta
./scripts/prepare-release.sh 0.14.0 beta

# 脚本会检查是否有 alpha 版本依赖
# 如果发现，会警告你更新它们

# 在 Cargo.toml 中手动更新 alpha 依赖到 beta 或 stable
# 例如，将 v0.7.6-alpha 改为 v0.7.6-beta

# 提交更改
git add .dev-stage.yml Cargo.toml
git commit -m "chore: transition to beta stage"
git push origin main

# 从现在开始，Dependabot 只会自动合并 beta 和 stable 更新
```

#### 场景 3: 准备 Stable 发布

```bash
# 更新阶段为 stable
./scripts/prepare-release.sh 0.14.0 stable

# 脚本会检查是否有 alpha/beta 版本依赖
# 如果发现，会报错并要求你更新它们

# 将所有依赖更新为稳定版本

# 提交并创建发布
git add .dev-stage.yml Cargo.toml
git commit -m "chore: prepare stable release v0.14.0"
git tag v0.14.0
git push origin main --tags

# 发布后，切换回 alpha 进行下一个开发周期
./scripts/prepare-release.sh 0.15.0 alpha
git add .dev-stage.yml
git commit -m "chore: start alpha development for v0.15.0"
git push origin main
```

### 故障排查

#### 问题: Dependabot PR 没有自动合并

**检查：**
1. workflow 是否在运行？ → 查看 Actions 标签
2. 当前阶段是什么？ → `cat .dev-stage.yml`
3. 依赖版本是什么？ → 查看 PR diff
4. 决策是什么？ → 查看 workflow 日志

**常见问题：**
- CI 检查未通过 → CI 通过前 PR 不会自动合并
- 阶段不匹配 → 更新 `.dev-stage.yml` 或手动审查
- 仓库未启用自动合并 → 在 Settings → General 中启用
