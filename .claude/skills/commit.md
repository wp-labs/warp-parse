# commit

分析当前工作区的改动，按项目规范生成 commit message 并提交。

## 工作流程

### 第一步：检查工作区状态

```bash
git status
git diff --stat
git diff --staged --stat
```

### 第二步：分析改动内容

读取所有已修改的文件 diff，理解改动的目的和范围：

```bash
git diff
git diff --staged
```

如果有未追踪的新文件，检查是否需要纳入提交（排除 .env、credentials 等敏感文件）。

### 第三步：生成 commit message

根据改动内容生成简洁的 commit message：

**格式规范：**
- 首行：祈使句式，概括改动目的（不超过 72 字符）
- 空行
- 可选的详细说明

**动词选择：**
- `Add` — 全新功能或文件
- `Update` — 对已有功能的增强
- `Fix` — Bug 修复
- `Remove` — 删除代码或功能
- `Refactor` — 重构（不改变行为）
- `Bump` — 依赖版本升级

**示例：**
```
Add semantic config to control NLP dictionary loading
```

### 第四步：向用户确认

展示：
1. 将要提交的文件列表
2. 生成的 commit message

等待用户确认或修改后再执行提交。

### 第五步：执行提交

```bash
git add <specific-files>
git commit -m "<message>"
```

## 注意事项

- 优先使用 `git add <具体文件>` 而非 `git add -A`
- 不要提交 .env、credentials.json 等敏感文件
- 不自动 push，提交后告知用户结果
- 如果工作区没有改动，告知用户无需提交
