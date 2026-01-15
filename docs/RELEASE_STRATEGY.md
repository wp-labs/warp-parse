# Release Strategy

[English](#english) | [中文](#中文)

---

## English

### Overview

WarpParse uses a single-branch development model with semantic versioning tags to manage three maturity levels: **alpha**, **beta**, and **stable**.

### Branch Strategy

```
main (development branch)
  ├── v0.14.0-alpha.1  (Alpha: development testing)
  ├── v0.14.0-alpha.2
  ├── v0.14.0-beta.1   (Beta: pre-production testing)
  ├── v0.14.0-beta.2
  └── v0.14.0          (Stable: production ready)
      └── release/v0.14.x (optional: for hotfixes)
```

### Workflow

#### 1. Daily Development
- All development happens on the `main` branch
- Dependencies (like `wp-connectors`) are merged directly to `main`
- CI runs on every commit to ensure code quality

#### 2. Alpha Release
When ready for internal testing:
```bash
# Ensure all changes are committed
git tag v0.14.0-alpha.1
git push origin v0.14.0-alpha.1
```

**Alpha characteristics:**
- For internal development and testing
- May contain unstable features
- Dependencies may use `-alpha` or `-beta` tags
- GitHub Release marked as "pre-release"

#### 3. Beta Release
When features are feature-complete and ready for broader testing:
```bash
git tag v0.14.0-beta.1
git push origin v0.14.0-beta.1
```

**Beta characteristics:**
- For pre-production testing
- Feature-complete but may have bugs
- Dependencies should use `-beta` or stable tags
- GitHub Release marked as "pre-release"
- Includes integration tests with wp-examples

#### 4. Stable Release
When thoroughly tested and production-ready:
```bash
git tag v0.14.0
git push origin v0.14.0
```

**Stable characteristics:**
- Production-ready
- All dependencies should use stable versions
- GitHub Release marked as official release
- Full CI/CD pipeline including Docker images
- Comprehensive documentation

#### 5. Hotfix (Optional)
For critical fixes to stable releases:
```bash
# Create release branch from stable tag
git checkout -b release/v0.14.x v0.14.0

# Make fixes
git commit -m "fix: critical bug"

# Tag the hotfix
git tag v0.14.1
git push origin v0.14.1

# Merge back to main
git checkout main
git merge release/v0.14.x
```

### Dependency Management

#### Automated Dependency Updates

WarpParse uses an **automated dependency management system** that combines Dependabot with GitHub Actions.

**Key Features:**
- 🤖 Automatic review and merge of dependency updates
- 🎯 Stage-aware: respects alpha/beta/stable development phases
- ⚡ Fast: merges safe updates without manual intervention
- 🔒 Safe: rejects incompatible versions automatically

**See:** [Automated Dependency Management Guide](./AUTOMATED_DEPENDENCY_MANAGEMENT.md)

#### wp-connectors Updates

Since `wp-connectors` updates frequently, the automation handles it based on current stage:

1. **Alpha Stage**: All updates auto-merged
   ```toml
   wp-connectors = { git = "https://github.com/wp-labs/wp-connectors", tag = "v0.7.6-alpha" }
   ```
   - ✅ `-alpha` versions → Auto-approved
   - ✅ `-beta` versions → Auto-approved
   - ✅ Stable versions → Auto-approved

2. **Beta Stage**: Only beta and stable auto-merged
   ```toml
   wp-connectors = { git = "https://github.com/wp-labs/wp-connectors", tag = "v0.7.6-beta" }
   ```
   - ❌ `-alpha` versions → Manual review required
   - ✅ `-beta` versions → Auto-approved
   - ✅ Stable versions → Auto-approved

3. **Stable Stage**: Only stable versions auto-merged
   ```toml
   wp-connectors = { git = "https://github.com/wp-labs/wp-connectors", tag = "v0.7.6" }
   ```
   - ❌ `-alpha` versions → Manual review required
   - ❌ `-beta` versions → Manual review required
   - ✅ Stable versions → Auto-approved

#### Dependabot Integration

Dependabot automatically creates PRs for:
- Git dependencies (wp-engine, wp-connectors)
- Crates.io dependencies
- GitHub Actions

The automation system (`dependabot-auto-merge.yml`) then:
1. Reads current development stage from `.dev-stage.yml`
2. Checks if the dependency version matches stage requirements
3. Auto-approves and merges compatible updates
4. Flags incompatible updates for manual review

### Version Numbering

Follow [Semantic Versioning 2.0.0](https://semver.org/):

- **MAJOR** (v1.0.0): Breaking changes
- **MINOR** (v0.14.0): New features, backward compatible
- **PATCH** (v0.14.1): Bug fixes

Maturity indicators:
- `-alpha.N`: Alpha releases (incremental)
- `-beta.N`: Beta releases (incremental)
- No suffix: Stable release

### Release Checklist

#### Before Alpha Release
- [ ] All tests pass locally
- [ ] Update `Cargo.toml` version
- [ ] Update dependencies to appropriate versions
- [ ] Run `cargo check` and `cargo test`

#### Before Beta Release
- [ ] All alpha issues resolved
- [ ] Integration tests pass (wp-examples)
- [ ] Update CHANGELOG.md
- [ ] Dependencies use beta or stable versions
- [ ] Documentation updated

#### Before Stable Release
- [ ] All beta issues resolved
- [ ] Full CI/CD pipeline passes
- [ ] All dependencies use stable versions
- [ ] CHANGELOG.md finalized
- [ ] Release notes prepared
- [ ] Documentation complete and reviewed

---

## 中文

### 概述

WarpParse 采用单分支开发模式，通过语义化版本标签管理三种成熟度级别：**alpha**、**beta** 和 **stable**。

### 分支策略

```
main (开发分支)
  ├── v0.14.0-alpha.1  (Alpha: 开发测试)
  ├── v0.14.0-alpha.2
  ├── v0.14.0-beta.1   (Beta: 准生产测试)
  ├── v0.14.0-beta.2
  └── v0.14.0          (Stable: 生产就绪)
      └── release/v0.14.x (可选：用于热修复)
```

### 工作流程

#### 1. 日常开发
- 所有开发在 `main` 分支进行
- 依赖更新（如 `wp-connectors`）直接合并到 `main`
- 每次提交触发 CI 确保代码质量

#### 2. Alpha 发布
准备内部测试时：
```bash
# 确保所有更改已提交
git tag v0.14.0-alpha.1
git push origin v0.14.0-alpha.1
```

**Alpha 特征：**
- 用于内部开发和测试
- 可能包含不稳定特性
- 依赖可使用 `-alpha` 或 `-beta` 标签
- GitHub Release 标记为 "pre-release"

#### 3. Beta 发布
功能完整且准备更广泛测试时：
```bash
git tag v0.14.0-beta.1
git push origin v0.14.0-beta.1
```

**Beta 特征：**
- 用于准生产测试
- 功能完整但可能存在 bug
- 依赖应使用 `-beta` 或稳定标签
- GitHub Release 标记为 "pre-release"
- 包含 wp-examples 集成测试

#### 4. Stable 发布
充分测试并准备生产时：
```bash
git tag v0.14.0
git push origin v0.14.0
```

**Stable 特征：**
- 生产就绪
- 所有依赖使用稳定版本
- GitHub Release 标记为正式发布
- 完整 CI/CD 流程包括 Docker 镜像
- 完善的文档

#### 5. 热修复（可选）
针对稳定版本的关键修复：
```bash
# 从稳定标签创建发布分支
git checkout -b release/v0.14.x v0.14.0

# 进行修复
git commit -m "fix: critical bug"

# 打热修复标签
git tag v0.14.1
git push origin v0.14.1

# 合并回 main
git checkout main
git merge release/v0.14.x
```

### 依赖管理

#### 自动化依赖更新

WarpParse 使用**自动化依赖管理系统**，结合 Dependabot 和 GitHub Actions。

**主要特性：**
- 🤖 自动审查和合并依赖更新
- 🎯 阶段感知：遵循 alpha/beta/stable 开发阶段
- ⚡ 快速：无需手动干预即可合并安全更新
- 🔒 安全：自动拒绝不兼容版本

**详见：** [自动化依赖管理指南](./AUTOMATED_DEPENDENCY_MANAGEMENT.md)

#### wp-connectors 更新

由于 `wp-connectors` 更新频繁，自动化系统根据当前阶段处理：

1. **Alpha 阶段**：所有更新自动合并
   ```toml
   wp-connectors = { git = "https://github.com/wp-labs/wp-connectors", tag = "v0.7.6-alpha" }
   ```
   - ✅ `-alpha` 版本 → 自动批准
   - ✅ `-beta` 版本 → 自动批准
   - ✅ 稳定版本 → 自动批准

2. **Beta 阶段**：仅 beta 和稳定版本自动合并
   ```toml
   wp-connectors = { git = "https://github.com/wp-labs/wp-connectors", tag = "v0.7.6-beta" }
   ```
   - ❌ `-alpha` 版本 → 需要手动审查
   - ✅ `-beta` 版本 → 自动批准
   - ✅ 稳定版本 → 自动批准

3. **Stable 阶段**：仅稳定版本自动合并
   ```toml
   wp-connectors = { git = "https://github.com/wp-labs/wp-connectors", tag = "v0.7.6" }
   ```
   - ❌ `-alpha` 版本 → 需要手动审查
   - ❌ `-beta` 版本 → 需要手动审查
   - ✅ 稳定版本 → 自动批准

#### Dependabot 集成

Dependabot 自动为以下内容创建 PR：
- Git 依赖（wp-engine、wp-connectors）
- Crates.io 依赖
- GitHub Actions

自动化系统（`dependabot-auto-merge.yml`）然后：
1. 从 `.dev-stage.yml` 读取当前开发阶段
2. 检查依赖版本是否符合阶段要求
3. 自动批准并合并兼容的更新
4. 标记不兼容的更新以供手动审查

### 版本编号

遵循[语义化版本 2.0.0](https://semver.org/lang/zh-CN/)：

- **主版本**（v1.0.0）：破坏性变更
- **次版本**（v0.14.0）：新功能，向后兼容
- **修订版本**（v0.14.1）：问题修复

成熟度标识：
- `-alpha.N`：Alpha 版本（递增）
- `-beta.N`：Beta 版本（递增）
- 无后缀：稳定版本

### 发布检查清单

#### Alpha 发布前
- [ ] 本地所有测试通过
- [ ] 更新 `Cargo.toml` 版本号
- [ ] 更新依赖到适当版本
- [ ] 运行 `cargo check` 和 `cargo test`

#### Beta 发布前
- [ ] 所有 alpha 问题已解决
- [ ] 集成测试通过（wp-examples）
- [ ] 更新 CHANGELOG.md
- [ ] 依赖使用 beta 或稳定版本
- [ ] 文档已更新

#### Stable 发布前
- [ ] 所有 beta 问题已解决
- [ ] 完整 CI/CD 流程通过
- [ ] 所有依赖使用稳定版本
- [ ] CHANGELOG.md 完成
- [ ] 发布说明准备完毕
- [ ] 文档完整并已审阅
