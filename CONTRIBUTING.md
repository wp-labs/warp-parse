# Contributing to WarpParse

Thank you for your interest in contributing to WarpParse! We welcome contributions from the community, whether it's bug fixes, new features, documentation improvements, or performance optimizations.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Code Standards](#code-standards)
- [Submitting Issues](#submitting-issues)
- [Submitting Pull Requests](#submitting-pull-requests)
- [Commit Message Guidelines](#commit-message-guidelines)
- [Code Review Process](#code-review-process)

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally
3. Create a new branch for your changes
4. Make your changes and commit them
5. Push your branch to your fork
6. Submit a Pull Request to the main repository

## Development Setup

### Prerequisites

- Rust 1.75 or later
- Cargo
- Git

### Building from Source

```bash
# Clone the repository
git clone https://github.com/wp-labs/warp-parse.git
cd warp-parse

# Build the project
cargo build --release

# Run tests
cargo test --all

# Run with default features (community edition)
cargo build --features community --release

# Run with specific features
cargo build --no-default-features --features core --release
```

### Running Locally

```bash
# Run the main parser
./target/release/wparse --help

# Run the rule generator
./target/release/wpgen --help

# Run the project manager
./target/release/wproj --help

# Run the recovery tool
./target/release/wprescue --help
```

## Code Standards

### Rust Code Style

We follow standard Rust conventions:

- Use `cargo fmt` to format your code
- Use `cargo clippy` to check for common mistakes and improvements
- Ensure all tests pass with `cargo test`
- Maintain test coverage for new features

### Code Formatting

```bash
# Format all code
cargo fmt --all

# Check formatting without modifying
cargo fmt --all -- --check

# Run clippy lints
cargo clippy --all --all-targets --all-features -- -D warnings
```

### Documentation

- Add documentation comments to public APIs
- Use `///` for public items and `//!` for module-level documentation
- Include examples in documentation where applicable
- Keep README and docs updated with any API changes

### Testing

- Write tests for all new features
- Write tests for bug fixes (including regression tests)
- Use `#[test]` for unit tests
- Use `#[cfg(test)]` for test modules
- Ensure tests are deterministic and don't depend on external state

```bash
# Run all tests
cargo test --all

# Run tests for a specific crate
cargo test -p warp-parse

# Run tests with all features
cargo test --all-features

# Run specific test
cargo test test_name -- --nocapture
```

## Submitting Issues

### Before Creating an Issue

- Search existing issues to avoid duplicates
- Check if the issue is already fixed in the latest version
- Gather as much information as possible

### Issue Template

Please include the following information:

```markdown
## Description
Brief description of the issue

## Reproduction Steps
1. Step 1
2. Step 2
3. ...

## Expected Behavior
What you expected to happen

## Actual Behavior
What actually happened

## Environment
- OS: (e.g., macOS, Linux, Windows)
- Rust version: (output of `rustc --version`)
- WarpParse version: (output of `wparse --version`)

## Additional Context
Any additional information that might be helpful
```

### Issue Types

- **Bug Report**: Something isn't working as expected
- **Feature Request**: Suggestions for new functionality
- **Documentation**: Issues with documentation clarity or accuracy
- **Performance**: Performance-related issues or optimization suggestions

## Submitting Pull Requests

### Before Starting

1. Check existing pull requests to avoid duplicate work
2. Open an issue to discuss major changes first
3. For large changes, get feedback in an issue before coding

### PR Guidelines

1. **One feature/fix per PR**: Keep PRs focused and manageable
2. **Clear title**: Use a descriptive title that explains the change
3. **Detailed description**: Explain what changed and why
4. **Link to issues**: Reference related issues using `#issue_number`
5. **Keep it small**: Smaller PRs are reviewed faster

### PR Template

```markdown
## Description
Brief description of the changes

## Type of Change
- [ ] Bug fix (non-breaking change)
- [ ] New feature (non-breaking change)
- [ ] Breaking change
- [ ] Documentation update
- [ ] Performance improvement

## Related Issues
Fixes #(issue number)

## Testing
- [ ] Tests added/updated
- [ ] All tests pass locally
- [ ] Changes tested manually

## Checklist
- [ ] Code follows style guidelines
- [ ] Documentation updated
- [ ] No new warnings generated
- [ ] Changes are backward compatible
```

## Commit Message Guidelines

We follow conventional commit style:

```
type(scope): subject

body

footer
```

### Types

- **feat**: A new feature
- **fix**: A bug fix
- **docs**: Documentation only changes
- **style**: Changes that don't affect code logic (formatting, etc.)
- **refactor**: Code change that neither fixes a bug nor adds a feature
- **perf**: Code change that improves performance
- **test**: Adding or updating tests
- **chore**: Changes to build process, dependencies, etc.

### Examples

```
feat(parser): support new WPL syntax for conditional parsing

Implement support for if-else conditions in WPL rules, enabling
more complex parsing workflows.

Closes #123
```

```
fix(connector): handle null values in Kafka messages

Previously, null values in Kafka messages would cause a panic.
Now they are safely handled and logged.

Fixes #456
```

## Code Review Process

1. **Automated Checks**: GitHub Actions will run tests and linters
2. **Code Review**: Project maintainers will review your code
3. **Feedback**: Address any feedback or questions
4. **Approval**: Once approved, your PR can be merged
5. **Merge**: The maintainer will merge the PR

### Tips for Faster Review

- Write clear, descriptive commit messages
- Add comments explaining complex logic
- Include tests for your changes
- Keep PRs focused and reasonably sized
- Respond promptly to feedback

## Development Tips

### Useful Commands

```bash
# Run all checks (format, clippy, tests)
cargo fmt --all && cargo clippy --all && cargo test --all

# Check specific feature
cargo build --no-default-features --features kafka

# Generate documentation
cargo doc --no-deps --open

# Profile performance
cargo build --release
```

### Feature Flags

WarpParse has several feature flags:

- `core`: Minimal runtime
- `community`: Community edition with all connectors (default)
- `kafka`: Kafka connector support
- `mysql`: MySQL connector support
- `clickhouse`: ClickHouse connector support
- `elasticsearch`: Elasticsearch connector support
- `prometheus`: Prometheus connector support

### Getting Help

- Check existing documentation in `docs/`
- Look at existing tests for examples
- Check GitHub Issues for similar questions
- Ask in GitHub Discussions

## License

By contributing to WarpParse, you agree that your contributions will be licensed under the Apache License 2.0. See [LICENSE](LICENSE) for details.

## Code of Conduct

Please be respectful and constructive in all interactions with other community members.

---

# 为 WarpParse 做贡献

感谢你对 WarpParse 的贡献！我们欢迎来自社区的各类贡献，包括 Bug 修复、新功能、文档改进或性能优化。

## 目录

- [快速开始](#快速开始)
- [开发环境设置](#开发环境设置)
- [代码规范](#代码规范)
- [提交 Issue](#提交-issue)
- [提交 Pull Request](#提交-pull-request)
- [提交信息规范](#提交信息规范)
- [代码审查流程](#代码审查流程)

## 快速开始

1. 在 GitHub 上 Fork 本仓库
2. 克隆你的 Fork 到本地
3. 为你的改动创建一个新分支
4. 进行代码修改并提交
5. 推送分支到你的 Fork
6. 向主仓库提交 Pull Request

## 开发环境设置

### 前置要求

- Rust 1.75 或更高版本
- Cargo
- Git

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/wp-labs/warp-parse.git
cd warp-parse

# 构建项目
cargo build --release

# 运行测试
cargo test --all

# 构建社区版（包含所有连接器）
cargo build --features community --release

# 构建指定特性的版本
cargo build --no-default-features --features core --release
```

### 本地运行

```bash
# 运行主解析器
./target/release/wparse --help

# 运行规则生成工具
./target/release/wpgen --help

# 运行项目管理工具
./target/release/wproj --help

# 运行恢复工具
./target/release/wprescue --help
```

## 代码规范

### Rust 代码风格

我们遵循标准的 Rust 编码约定：

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查常见问题和改进建议
- 确保所有测试通过：`cargo test`
- 为新功能维护测试覆盖率

### 代码格式化

```bash
# 格式化所有代码
cargo fmt --all

# 检查格式（不修改）
cargo fmt --all -- --check

# 运行 clippy 检查
cargo clippy --all --all-targets --all-features -- -D warnings
```

### 文档

- 为公开 API 添加文档注释
- 使用 `///` 处理公开项目，使用 `//!` 处理模块级文档
- 在文档中包含示例（如适用）
- 更新 README 和相关文档以反映 API 变化

### 测试

- 为所有新功能编写测试
- 为 Bug 修复编写测试（包括回归测试）
- 使用 `#[test]` 编写单元测试
- 使用 `#[cfg(test)]` 处理测试模块
- 确保测试是确定性的，不依赖外部状态

```bash
# 运行所有测试
cargo test --all

# 运行特定 crate 的测试
cargo test -p warp-parse

# 运行所有特性的测试
cargo test --all-features

# 运行指定测试
cargo test test_name -- --nocapture
```

## 提交 Issue

### 提交前

- 搜索现有 Issue，避免重复
- 检查问题是否已在最新版本中解决
- 收集尽可能多的信息

### Issue 模板

请包含以下信息：

```markdown
## 问题描述
简要描述问题

## 复现步骤
1. 步骤 1
2. 步骤 2
3. ...

## 预期行为
你期望发生的事情

## 实际行为
实际发生的事情

## 环境信息
- 操作系统：(例如 macOS、Linux、Windows)
- Rust 版本：(运行 `rustc --version` 的输出)
- WarpParse 版本：(运行 `wparse --version` 的输出)

## 额外信息
任何可能有帮助的额外信息
```

### Issue 类型

- **Bug 报告**：某些功能未按预期工作
- **功能请求**：新功能建议
- **文档**：文档清晰度或准确性问题
- **性能**：性能相关问题或优化建议

## 提交 Pull Request

### 开始前

1. 检查现有 Pull Request 以避免重复工作
2. 对于重大改动，先开 Issue 讨论
3. 获取反馈后再开始编码

### PR 指南

1. **一个功能/修复一个 PR**：保持 PR 专注且易于管理
2. **清晰的标题**：使用描述性标题说明改动内容
3. **详细描述**：解释改动内容和原因
4. **关联 Issue**：使用 `#issue_number` 关联相关 Issue
5. **保持简洁**：较小的 PR 审查速度更快

### PR 模板

```markdown
## 描述
改动的简要说明

## 改动类型
- [ ] Bug 修复（非破坏性改动）
- [ ] 新功能（非破坏性改动）
- [ ] 破坏性改动
- [ ] 文档更新
- [ ] 性能改进

## 关联 Issue
Fixes #(issue 号)

## 测试
- [ ] 已添加/更新测试
- [ ] 本地所有测试通过
- [ ] 已进行手动测试

## 检查清单
- [ ] 代码遵循风格指南
- [ ] 文档已更新
- [ ] 没有新的警告
- [ ] 改动向后兼容
```

## 提交信息规范

我们遵循 Conventional Commit 风格：

```
type(scope): subject

body

footer
```

### 类型

- **feat**: 新功能
- **fix**: Bug 修复
- **docs**: 仅文档改动
- **style**: 不影响代码逻辑的改动（格式化等）
- **refactor**: 代码重构（既不修复 Bug 也不添加功能）
- **perf**: 性能改进
- **test**: 添加或更新测试
- **chore**: 构建过程、依赖等改动

### 例子

```
feat(parser): WPL 支持条件解析语法

实现 WPL 规则中的 if-else 条件支持，
使复杂的解析工作流成为可能。

Closes #123
```

```
fix(connector): 处理 Kafka 消息中的空值

之前 Kafka 消息中的空值会导致崩溃。
现在安全处理并记录日志。

Fixes #456
```

## 代码审查流程

1. **自动检查**：GitHub Actions 运行测试和检查
2. **代码审查**：项目维护者审查你的代码
3. **反馈**：处理反馈或回答问题
4. **批准**：审查通过后 PR 可合并
5. **合并**：维护者合并 PR

### 加快审查的提示

- 编写清晰、描述性的提交信息
- 为复杂逻辑添加注释
- 为改动包含测试
- 保持 PR 专注且适度大小
- 及时回应反馈

## 开发提示

### 常用命令

```bash
# 运行所有检查（格式化、clippy、测试）
cargo fmt --all && cargo clippy --all && cargo test --all

# 检查特定特性
cargo build --no-default-features --features kafka

# 生成文档
cargo doc --no-deps --open

# 性能分析
cargo build --release
```

### 特性标志

WarpParse 提供多个特性标志：

- `core`: 最小运行时
- `community`: 社区版，包含所有连接器（默认）
- `kafka`: Kafka 连接器支持
- `mysql`: MySQL 连接器支持
- `clickhouse`: ClickHouse 连接器支持
- `elasticsearch`: Elasticsearch 连接器支持
- `prometheus`: Prometheus 连接器支持

### 获取帮助

- 查看 `docs/` 中的现有文档
- 查看现有测试作为示例
- 搜索 GitHub Issues 中的类似问题
- 在 GitHub Discussions 中提问

## 许可协议

通过为 WarpParse 做贡献，你同意你的贡献将根据 Apache License 2.0 授权。详见 [LICENSE](LICENSE)。

## 行为准则

请在与其他社区成员的所有互动中保持尊重和建设性态度。
