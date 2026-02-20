# update-changelog

更新 CHANGELOG.md，遵循项目 CLAUDE.md 中定义的版本管理规范。

## 工作流程

### 第一步：确认版本状态

执行以下命令获取当前版本信息：

```bash

cat version.txt
# 查看最近的 tag
git log --oneline --decorate -10 | grep tag

# 查看已发布版本的 CHANGELOG 内容
git show <tag-commit>:CHANGELOG.md | head -50
```

### 第二步：读取当前 CHANGELOG.md

读取 CHANGELOG.md 的头部，了解当前 Unreleased 段落和最新已发布版本。

### 第三步：分析待记录的改动

检查自上次版本以来的改动：

```bash
# 查看自上次 tag 以来的提交
git log <latest-tag>..HEAD --oneline
```

### 第四步：读取主要组件的变更记录

本项目的核心功能依赖以下上游组件，更新 CHANGELOG 前需要了解它们的变更：

| 组件 | 本地路径 | Cargo.toml 中的 tag |
|------|----------|---------------------|
| wp-motor (wp-engine, wp-lang, wp-config 等) | `../wp-motor` | `wp-engine` 条目的 `tag` 字段 |
| wp-connectors | `../wp-connectors` | `wp-connectors` 条目的 `tag` 字段 |

**操作步骤：**

1. 从 Cargo.toml 提取当前依赖版本，从上次发布的 tag 提取旧版本：

```bash
# 当前版本（Cargo.toml）
grep -A1 'wp-engine' Cargo.toml | grep tag
grep -A1 'wp-connectors' Cargo.toml | grep tag

# 上次发布时的版本
git show <latest-tag>:Cargo.toml | grep -A1 'wp-engine' | grep tag
git show <latest-tag>:Cargo.toml | grep -A1 'wp-connectors' | grep tag
```

2. 如果组件版本发生了变化，读取对应组件的 CHANGELOG.md，提取两个版本之间的变更：

```bash
# 读取 wp-motor 的变更记录
cat ../wp-motor/CHANGELOG.md

# 读取 wp-connectors 的变更记录
cat ../wp-connectors/CHANGELOG.md
```

3. 从组件 CHANGELOG 中筛选出与本项目相关的改动，按照已有 CHANGELOG 的格式整理（参见 `0.18.0`、`0.17.0` 等版本的写法模式）。

### 第五步：添加条目

按以下规则添加到 CHANGELOG.md：

- 新改动添加到最顶部的 `## [x.y.z Unreleased]` 段落
- 如果不存在 Unreleased 段落，创建一个（版本号 = 最新已发布版本的下一个合理版本）
- 使用标准分类：`### Added`、`### Changed`、`### Fixed`、`### Removed`
- 每条记录格式：`- **模块名**: 改动描述`
- 不要修改已发布版本（有日期的）的内容

### 格式参考

```markdown
## [x.y.z Unreleased]

### Added
- **Module**: Description

### Changed
- **Module**: Description

### Fixed
- **Module**: Description
```

## 注意事项

- 先询问用户要添加什么改动，或者根据 git log 自动分析
- 模块名使用项目中的 crate 或组件名（如 OML、wp-lang、Engine Config 等）
- 描述使用英文，简洁明了

# 最后同步更新 CHANGELOG.en.md 英文版
