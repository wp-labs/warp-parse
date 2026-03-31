# 分支保护规则

## 目标

这些规则用于在 GitHub 上落实三分支策略，防止依赖绕过应有的成熟度晋级路径。

## 默认分支

将默认分支设置为 `alpha`，确保新开发默认从开发通道开始。

## Alpha 规则

- 合并前必须走 PR
- 所需审批数: `0`
- 必须通过 CI
- 禁止 force push 和删除

原因：

- 保持开发速度
- 同时保留基本构建和测试保障

## Beta 规则

- 合并前必须走 PR
- 所需审批数: `1`
- 新提交后清除过期审批
- 必须通过 CI 和 `dependabot-branch-filter`
- 必须解决会话讨论
- 仅允许发布管理者推送

原因：

- beta 是稳定化通道
- 必须阻止 alpha 级依赖进入

## Main 规则

- 合并前必须走 PR
- 所需审批数: `2`
- 需要 Code Owners 评审（如配置）
- 必须通过 CI 和 `dependabot-branch-filter`
- 必须使用签名提交
- 必须保持线性历史
- 仅允许发布管理者推送
- 仅为紧急管理员保留有限绕过能力

原因：

- main 面向生产
- 依赖成熟度和审计能力要求最高

## Dependabot 处理原则

即使分支策略会拒绝，Dependabot 仍可能先创建 PR。建议同时使用：

- 必选评审
- 必选 `filter` 状态检查
- 异常情况下的人工确认

如果 PR 目标是 `beta` 或 `main`，且依赖成熟度不足，应直接按策略关闭。

## 立即配置清单

1. 将默认分支改为 `alpha`
2. 配置 `alpha`、`beta`、`main` 的保护规则
3. 在 `beta` 和 `main` 上要求 `filter` 工作流为必过
4. 收紧发布分支推送权限
5. 确认三个分支都存在对应工作流

## 对应英文版

- [../en/branch_protection_rules.md](../en/branch_protection_rules.md)
