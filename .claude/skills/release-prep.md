# release-prep

准备新版本发布：更新版本号、CHANGELOG 日期、打 git tag。

## 前置条件

执行前先确认：
1. 所有代码已提交（工作区干净）
2. 测试全部通过（建议先运行 /cargo-check-all）

## 工作流程

### 第一步：确认发布版本

```bash
# 查看当前版本和最近的 tag
git log --oneline --decorate -10 | grep tag
head -20 CHANGELOG.md
```

读取 CHANGELOG.md 顶部的 `[x.y.z Unreleased]` 确认要发布的版本号。
如果没有 Unreleased 段落，询问用户要发布的版本号。

### 第二步：询问用户确认

向用户确认：
- 版本号是否正确
- CHANGELOG 中的改动是否完整
- 是否需要调整任何条目

### 第三步：更新 CHANGELOG 日期

将 `## [x.y.z Unreleased]` 改为 `## [x.y.z] - YYYY-MM-DD`（使用今天的日期）。

### 第四步：更新 Cargo.toml 版本号

检查根 `Cargo.toml` 和各 crate 的 `Cargo.toml` 中的 version 字段：

```bash
grep -n '^version' Cargo.toml
```

如果版本号需要更新，逐一修改。注意只改需要变动的 crate。

### 第五步：提交版本变更

```bash
git add CHANGELOG.md Cargo.toml Cargo.lock
git commit -m "Release vx.y.z"
```

### 第六步：打 tag

```bash
git tag vx.y.z
```

## 注意事项

- 每一步都需要用户确认后再继续
- 不自动 push 到远端，留给用户决定
- tag 格式统一为 `vx.y.z`（如 `v1.17.0`）
- 打完 tag 后，提示用户可以运行 `git push && git push --tags`
