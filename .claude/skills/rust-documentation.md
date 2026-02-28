# rust-documentation

编写符合 Rust 社区标准的文档：rustdoc 注释、doc-test、模块文档。

## 适用场景

- 为公共 API 添加文档
- 审查现有文档是否完整
- 编写 crate 级别的文档和示例

## 工作流程

### 第一步：检查文档覆盖率

```bash
# 检查缺少文档的公共 API
# 在 crate 的 lib.rs 顶部临时加上：
# #![warn(missing_docs)]
cargo check -p <crate> 2>&1

# 生成文档并检查
cargo doc -p <crate> --no-deps --open
```

### 第二步：编写文档注释

**基本格式：**

```rust
/// 单行摘要（第一段会出现在类型/函数列表中）。
///
/// 更详细的说明，可以跨多段。包含背景知识、使用场景、
/// 注意事项等。
///
/// # Examples
///
/// ```
/// use my_crate::MyType;
///
/// let obj = MyType::new("hello");
/// assert_eq!(obj.len(), 5);
/// ```
pub struct MyType { ... }
```

**函数文档的标准节（section）：**

```rust
/// 解析输入文本为结构化 AST。
///
/// 接受 OML 格式的文本输入，返回完整的抽象语法树。
/// 解析器支持增量解析和错误恢复。
///
/// # Arguments
///
/// * `input` - OML 格式的源代码文本
/// * `options` - 解析选项，控制错误恢复策略等
///
/// # Returns
///
/// 成功时返回 AST 根节点；失败时返回包含所有
/// 语法错误的 `ParseError`。
///
/// # Errors
///
/// 当输入包含无法恢复的语法错误时返回 [`ParseError`]：
/// - [`ParseError::UnexpectedToken`] — 遇到意外的 token
/// - [`ParseError::UnterminatedString`] — 字符串字面量未闭合
///
/// # Panics
///
/// 当 `input` 长度超过 `usize::MAX / 2` 时 panic（实际不会发生）。
///
/// # Examples
///
/// ```
/// use wf_lang::{parse, ParseOptions};
///
/// let ast = parse("key = 42", &ParseOptions::default())?;
/// assert_eq!(ast.entries().len(), 1);
/// # Ok::<(), wf_lang::ParseError>(())
/// ```
pub fn parse(input: &str, options: &ParseOptions) -> Result<Ast, ParseError> { ... }
```

**必要的节：**

| 节 | 何时需要 |
|----|----------|
| 摘要（第一段） | 始终需要 |
| `# Errors` | 函数返回 `Result` 时 |
| `# Panics` | 函数可能 panic 时 |
| `# Examples` | 所有公共 API |
| `# Safety` | `unsafe fn` |

**可选的节：**

| 节 | 适用场景 |
|----|----------|
| `# Arguments` | 参数较多或语义不明显时 |
| `# Returns` | 返回值不明显时 |
| `# Performance` | 有性能特征需要说明时 |

### 第三步：编写 Doc-Test

Doc-test 是 rustdoc 中的代码块，会在 `cargo test` 时自动运行。

**基本规则：**

```rust
/// ```
/// // 这段代码会被编译和执行
/// let x = 1 + 1;
/// assert_eq!(x, 2);
/// ```
```

**隐藏辅助代码：**

```rust
/// ```
/// # // 以 # 开头的行不会显示在文档中，但会编译
/// # use my_crate::Config;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = Config::from_str("key = value")?;
/// assert_eq!(config.get("key"), Some("value"));
/// # Ok(())
/// # }
/// ```
```

**标记不运行的代码：**

```rust
/// ```no_run
/// // 编译但不运行（如网络操作、文件写入）
/// let data = fetch_from_api("https://example.com").await?;
/// ```

/// ```compile_fail
/// // 预期编译失败（展示 API 不允许的用法）
/// let mut x = vec![1, 2, 3];
/// let first = &x[0];
/// x.push(4);  // 编译错误：不能在有不可变借用时修改
/// println!("{}", first);
/// ```

/// ```ignore
/// // 完全跳过（最后手段，尽量少用）
/// ```
```

### 第四步：模块和 Crate 级别文档

**模块文档（`//!` 注释）：**

```rust
//! # wf-lang
//!
//! OML 语言的解析器和 AST 定义。
//!
//! 本 crate 提供：
//! - OML 文本解析（[`parse`] 函数）
//! - AST 节点类型（[`ast`] 模块）
//! - 语法错误类型和恢复策略
//!
//! ## Quick Start
//!
//! ```
//! use wf_lang::parse;
//!
//! let ast = parse("key = 42")?;
//! for entry in ast.entries() {
//!     println!("{}: {}", entry.key(), entry.value());
//! }
//! # Ok::<(), wf_lang::ParseError>(())
//! ```

pub mod ast;
pub mod parse;
```

### 第五步：链接和交叉引用

```rust
/// 返回 [`Config`] 中指定 key 的值。
///
/// 参见 [`Config::get_or_default`] 获取带默认值的版本。
///
/// 行为类似 [`HashMap::get`](std::collections::HashMap::get)。
pub fn get(&self, key: &str) -> Option<&str> { ... }
```

**链接语法：**
- `[`Type`]` — 当前作用域内的类型
- `[`Type::method`]` — 方法
- `[`module::Type`]` — 带路径的类型
- `[显示文本](`Type`)` — 自定义显示文本
- `[`std::collections::HashMap`]` — 标准库类型

### 第六步：验证文档

```bash
# 编译文档（检查链接和 doc-test）
cargo doc -p <crate> --no-deps 2>&1

# 运行 doc-test
cargo test -p <crate> --doc 2>&1

# 检查文档链接是否有效（CI 推荐）
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps 2>&1
```

## 文档风格指南

- **摘要用第三人称陈述句**：`"Parses the input..."` 而非 `"Parse the input"`
- **保持简洁** — 文档是为了帮助理解，不是为了凑字数
- **示例优先** — 一个好的示例胜过三段描述
- **描述"是什么"和"为什么"** — 代码已经说了"怎么做"
- **避免 "Returns ..."** 开头的摘要 — 直接说函数做什么

## 审查清单

- [ ] 所有 `pub` 项都有 `///` 文档注释
- [ ] 每个公共函数至少有一个 `# Examples` 代码块
- [ ] 返回 `Result` 的函数有 `# Errors` 节
- [ ] 可能 panic 的函数有 `# Panics` 节
- [ ] `unsafe fn` 有 `# Safety` 节
- [ ] Doc-test 全部通过（`cargo test --doc`）
- [ ] 文档链接有效（`RUSTDOCFLAGS="-D warnings" cargo doc`）
- [ ] Crate 根文件（`lib.rs`）有 `//!` 级别的概述文档
