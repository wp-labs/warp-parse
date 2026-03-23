# Warp Parse 发布管理策略

- 版本: 1.0
- 状态: Official

## 概览

Warp Parse 采用三分支发布模型：

- `alpha`: 日常开发
- `beta`: 测试与稳定化
- `main`: 生产稳定发布

该模型与依赖成熟度控制配套使用，确保不同分支只接收其应承载的依赖稳定级别。

## 分支职责

### Alpha

- 用途: 开发和试验
- 稳定性: 不稳定
- 依赖策略: 接收 alpha、beta、stable
- 发布标签: `vX.Y.Z-alpha.N`

### Beta

- 用途: 预发测试和稳定化
- 稳定性: 基本稳定，但允许存在待修复问题
- 依赖策略: 只接收 beta 和 stable
- 发布标签: `vX.Y.Z-beta.N`

### Main

- 用途: 生产发布
- 稳定性: 最高
- 依赖策略: 只接收 stable
- 发布标签: `vX.Y.Z`

## 合并方向

只允许前向推进：

```text
alpha -> beta -> main
```

除受控 hotfix 同步外，不允许逆向合并。

## 版本规则

采用带成熟度后缀的语义化版本：

- `v0.14.0-alpha.1`
- `v0.14.0-beta.2`
- `v0.14.0`
- `v0.14.1`

## 依赖管理

Dependabot 可以在各分支发起 PR，但是否允许合入由分支策略决定：

- alpha: 接收全部成熟度
- beta: 拒绝 alpha 依赖
- main: 拒绝 alpha 和 beta 依赖

必要防线：

- 分支保护规则
- 必选状态检查
- 分支级评审要求
- `dependabot-branch-filter` 工作流

## 发布流程

### Alpha 发布

1. 在 `alpha` 上完成开发
2. 校验 CI 和依赖状态
3. 打出 `vX.Y.Z-alpha.N`

### Beta 发布

1. 将 `alpha` 合并到 `beta`
2. 清理或升级不符合 beta 成熟度要求的依赖
3. 完成测试和稳定化验证
4. 打出 `vX.Y.Z-beta.N`

### Stable 发布

1. 将 `beta` 合并到 `main`
2. 确认所有依赖均为 stable
3. 完整执行发布校验
4. 打出 `vX.Y.Z`

### Hotfix

1. 从 `main` 拉出修复分支
2. 实现并验证修复
3. 合回 `main`
4. 发布 patch tag
5. 将修复同步回 `beta` 和 `alpha`

## 日常工作方式

开发者：

1. 默认从 `alpha` 开发
2. 保持依赖成熟度与目标分支一致
3. 当稳定性足够时再向前晋级

发布管理者：

1. 晋级前先检查成熟度
2. 确认状态检查和审批完成
3. 只在正确分支打对应版本标签

## 最佳实践

- 不要绕过前向晋级路径
- 避免对 `beta` 和 `main` 直接提交
- 把依赖成熟度视为发布质量的一部分
- 让发布说明与晋级过程保持一致

## 排障

- `main` 上出现预发布 Dependabot PR: 直接关闭
- `beta` 晋级受阻: 检查是否残留 alpha 依赖
- 稳定版发布失败: 检查签名提交、审批和 stable-only 依赖约束

## 对应英文版

- [../en/release_management.md](../en/release_management.md)
