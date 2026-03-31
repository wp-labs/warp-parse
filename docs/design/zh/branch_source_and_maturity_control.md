# 分支来源与成熟度控制

## 目的

本策略定义两件事：

- 哪些分支可以向哪些分支供给变更
- 每个分支允许接收什么成熟度级别的依赖

## 来源控制规则

### Alpha

允许：

- 直接提交
- 功能分支 PR
- 任意成熟度的 Dependabot PR

禁止：

- 从 `beta` 合并
- 从 `main` 合并

### Beta

允许：

- 从 `alpha` 合并
- beta 或 stable 的 Dependabot PR
- 有明确理由的紧急 cherry-pick

禁止：

- 常规直接提交
- 从 `main` 合并
- alpha 依赖 PR

### Main

允许：

- 从 `beta` 合并
- stable-only Dependabot PR
- 受控 hotfix 分支

禁止：

- 常规直接提交
- 从 `alpha` 合并
- alpha 或 beta 依赖 PR

## 依赖成熟度规则

- `alpha` 接收 alpha、beta、stable
- `beta` 接收 beta、stable
- `main` 只接收 stable

这条规则同时适用于代码变更和自动依赖更新。

## 合并流向

标准流向：

```text
alpha -> beta -> main
```

例外流向：

- 关键 hotfix 从 `main` 开始
- 发布后再回同步到 `beta` 和 `alpha`

## 落地方式

需要配套使用：

- 分支保护
- 评审
- 状态检查
- `dependabot-branch-filter`

任何一层缺失，都容易出现策略漂移。

## 操作建议

- 晋级时显式检查依赖 tag
- 不要把不稳定依赖误带到更高成熟度分支
- 把“分支来源”和“依赖成熟度”当成同一套治理规则处理

## 对应英文版

- [../en/branch_source_and_maturity_control.md](../en/branch_source_and_maturity_control.md)
