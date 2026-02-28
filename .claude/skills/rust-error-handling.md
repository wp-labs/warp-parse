# rust-error-handling

设计和实现符合 Rust 社区最佳实践的错误处理：自定义错误类型、错误传播、用户友好的错误信息。

## 输入

用户可能提供：
- 要改进错误处理的模块或函数
- 新模块需要设计错误类型
- 现有代码中 `.unwrap()` 过多需要清理
- 或者只是"错误处理不够好"

## 工作流程

### 第一步：评估现有错误处理

分析目标代码的错误处理现状：

```bash
# 查找 unwrap/expect 使用
grep -rn '\.unwrap()' crates/<crate>/src/ --include="*.rs"
grep -rn '\.expect(' crates/<crate>/src/ --include="*.rs"

# 查看现有错误类型
grep -rn 'enum.*Error' crates/<crate>/src/ --include="*.rs"
grep -rn 'thiserror' crates/<crate>/Cargo.toml
```

### 第二步：选择错误处理策略

根据场景选择合适的策略：

| 场景 | 推荐方案 | 说明 |
|------|----------|------|
| 库 crate（供外部使用） | `thiserror` 自定义错误 | 调用者需要精确匹配错误类型 |
| 应用 crate / CLI | `anyhow::Result` | 只需传播错误，不需要匹配 |
| 库内部 + 应用入口 | 混合使用 | 库用 thiserror，应用层用 anyhow |
| 简单的"不可能发生" | `expect("reason")` | 附带清晰的理由说明 |

### 第三步：设计错误类型（库 crate）

遵循 Rust 社区最佳实践设计错误枚举：

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unexpected token '{found}' at line {line}, expected {expected}")]
    UnexpectedToken {
        found: String,
        expected: String,
        line: usize,
    },

    #[error("unterminated string literal starting at line {0}")]
    UnterminatedString(usize),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

**设计原则：**

- 每个 variant 代表一种**可恢复的**错误场景
- `#[error("...")]` 消息面向最终用户，清晰描述出了什么问题
- 使用 `#[from]` 实现底层错误的自动转换
- 使用 `#[source]` 保留错误链（当不需要 From 转换时）
- variant 命名用名词（`NotFound`），不用动词（`FailedToFind`）

### 第四步：实现错误传播

**用 `?` 操作符替代手动 match：**

```rust
// Bad
let value = match parse(input) {
    Ok(v) => v,
    Err(e) => return Err(e.into()),
};

// Good
let value = parse(input)?;
```

**为跨类型转换实现 From：**

```rust
// thiserror 的 #[from] 自动生成 From impl
#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error(transparent)]
    Config(#[from] ConfigError),
}
```

### 第五步：处理 unwrap/expect

按以下优先级替换 `.unwrap()`：

1. **可以传播** → 使用 `?`
2. **有合理默认值** → 使用 `.unwrap_or()` / `.unwrap_or_default()` / `.unwrap_or_else()`
3. **逻辑上不可能失败** → 使用 `.expect("invariant: reason")` 并说明原因
4. **在测试中** → `.unwrap()` 可以接受，失败即 panic 是测试的正确行为

### 第六步：添加错误上下文

使用 anyhow 的 `context()` 为错误添加上下文（应用层）：

```rust
use anyhow::{Context, Result};

fn load_config(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;

    toml::from_str(&content)
        .context("failed to parse config as TOML")?
}
```

### 第七步：验证

```bash
cargo check -p <crate> 2>&1
cargo test -p <crate> 2>&1
```

## 最佳实践清单

- [ ] 库 crate 不依赖 `anyhow`，使用 `thiserror` 定义具体错误类型
- [ ] 错误消息是完整的句子（小写开头，无句号），描述"出了什么问题"
- [ ] 使用 `#[error(transparent)]` 包装底层错误，保留完整错误链
- [ ] 公共 API 返回 `Result<T, SpecificError>`，不返回 `Box<dyn Error>`
- [ ] `unwrap()` 仅用于测试代码和逻辑上不可达的分支（改用 `expect` 并注明理由）
- [ ] 避免在错误消息中重复 "error" 或 "failed to" 前缀（调用方会加）
- [ ] 错误类型实现 `Send + Sync + 'static`（thiserror 默认满足）

## 反模式

| 反模式 | 问题 | 改进 |
|--------|------|------|
| `Box<dyn Error>` 做公共返回类型 | 调用者无法 match | 定义具体错误枚举 |
| 错误信息含 "Error:" 前缀 | 链式显示时重复 | 直接描述问题 |
| 一个巨大的 Error 枚举 | 语义模糊 | 按模块拆分错误类型 |
| `panic!` 处理可恢复错误 | 库不应 panic | 返回 Result |
| 吞掉错误 `let _ = ...` | 隐藏问题 | 至少 log 或注释原因 |
