# rust-api-design

遵循 Rust API Guidelines 设计符合社区惯例的公共 API：类型签名、命名、人体工学。

## 适用场景

- 设计新的公共 API（pub fn、pub struct、pub trait）
- 审查现有 API 是否符合 Rust 惯例
- 重构 API 以提高人体工学

## 核心原则

### 1. 命名规范

遵循 [Rust API Guidelines - Naming](https://rust-lang.github.io/api-guidelines/naming.html)：

| 类别 | 规范 | 示例 |
|------|------|------|
| 类型 | UpperCamelCase | `ConfigBuilder`, `ParseError` |
| 函数/方法 | snake_case | `from_str`, `into_inner` |
| 常量 | SCREAMING_SNAKE_CASE | `MAX_RETRIES` |
| 模块 | snake_case | `config`, `error` |
| 生命周期 | 短小写 | `'a`, `'de`, `'src` |
| 转换方法 | `as_`/`to_`/`into_` | 见下文 |

**转换方法命名：**

| 前缀 | 成本 | 所有权 | 示例 |
|------|------|--------|------|
| `as_` | 零成本 | 借用 → 借用 | `as_str()`, `as_bytes()` |
| `to_` | 有成本 | 借用 → 拥有 | `to_string()`, `to_vec()` |
| `into_` | 零/低成本 | 拥有 → 拥有 | `into_inner()`, `into_bytes()` |
| `from_` | — | 构造 | `from_str()`, `from_path()` |

**Getter 命名：**
- 字段同名 getter：`fn name(&self) -> &str`（不要 `get_name`）
- 布尔 getter：`fn is_empty(&self) -> bool`、`fn has_children(&self) -> bool`

### 2. 类型签名设计

**输入参数——尽量泛化：**

```rust
// Bad: 只接受 String
fn process(input: String) -> Result<Output> { ... }

// Good: 接受任何能转为 &str 的类型
fn process(input: &str) -> Result<Output> { ... }

// Good: 接受 String 和 &str（需要所有权时）
fn process(input: impl Into<String>) -> Result<Output> { ... }
```

**常用输入泛化：**

| 如果需要 | 参数类型写 | 接受 |
|----------|-----------|------|
| 只读字符串 | `&str` | `&String`, `&str`, `String` (via deref) |
| 只读切片 | `&[T]` | `&Vec<T>`, `&[T]`, arrays |
| 只读路径 | `&Path` 或 `impl AsRef<Path>` | `&str`, `&String`, `PathBuf`, `&Path` |
| 迭代器 | `impl IntoIterator<Item = T>` | `Vec`, `&[]`, 任何迭代器 |
| 需要所有权的字符串 | `impl Into<String>` | `String`, `&str` |

**返回值——尽量具体：**

```rust
// Bad: 返回 trait object
fn items(&self) -> Box<dyn Iterator<Item = &Item>> { ... }

// Good: 返回具体类型（如果可以）
fn items(&self) -> std::slice::Iter<'_, Item> { ... }

// Good: 返回 impl Trait（不暴露具体类型）
fn items(&self) -> impl Iterator<Item = &Item> { ... }
```

### 3. Builder 模式

复杂对象构造使用 Builder：

```rust
pub struct ServerConfig {
    host: String,
    port: u16,
    max_connections: usize,
}

pub struct ServerConfigBuilder {
    host: String,
    port: u16,
    max_connections: usize,
}

impl ServerConfigBuilder {
    pub fn new(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: 8080,
            max_connections: 100,
        }
    }

    /// 链式调用，消费 self 并返回
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn max_connections(mut self, n: usize) -> Self {
        self.max_connections = n;
        self
    }

    pub fn build(self) -> ServerConfig {
        ServerConfig {
            host: self.host,
            port: self.port,
            max_connections: self.max_connections,
        }
    }
}
```

**何时用 Builder：**
- 构造需要 3 个以上参数
- 大多数参数有合理默认值
- 参数组合有约束需要在 `build()` 时校验

### 4. 标准 Trait 实现

每个公共类型应考虑实现以下 trait：

| Trait | 何时实现 | 方式 |
|-------|----------|------|
| `Debug` | 几乎所有类型 | `#[derive(Debug)]` |
| `Clone` | 值语义类型 | `#[derive(Clone)]` |
| `PartialEq` / `Eq` | 需要比较 | `#[derive(PartialEq, Eq)]` |
| `Hash` | 用作 HashMap key | `#[derive(Hash)]`（需同时有 Eq） |
| `Display` | 面向用户的输出 | 手动 impl |
| `Default` | 有合理零值 | `#[derive(Default)]` 或手动 impl |
| `From` / `Into` | 类型转换 | 只 impl `From`，`Into` 自动获得 |
| `Serialize` / `Deserialize` | 需要序列化 | `#[derive(Serialize, Deserialize)]` |

**规则：如果能 derive 就 derive，不需要手写。**

### 5. 文档和示例

公共 API 必须有文档：

```rust
/// 解析 OML 源码为 AST。
///
/// # Errors
///
/// 当输入包含语法错误时返回 [`ParseError`]。
///
/// # Examples
///
/// ```
/// use wf_lang::parse;
///
/// let ast = parse("key = 42")?;
/// assert_eq!(ast.entries().len(), 1);
/// # Ok::<(), wf_lang::ParseError>(())
/// ```
pub fn parse(input: &str) -> Result<Ast, ParseError> { ... }
```

### 6. 可见性控制

```
pub          — 真正的公共 API（对外承诺稳定性）
pub(crate)   — crate 内部共享，对外不可见
pub(super)   — 仅父模块可见
私有（默认）  — 仅当前模块
```

**原则：默认私有，按需放开。**

## 审查清单

- [ ] 函数参数使用最泛化的类型（`&str` 而非 `String`，`&[T]` 而非 `Vec<T>`）
- [ ] 返回值使用具体类型或 `impl Trait`
- [ ] 所有 pub 类型都 derive 了 `Debug`
- [ ] 所有 pub 类型考虑了 `Clone`、`PartialEq`、`Default`
- [ ] 转换方法命名遵循 `as_`/`to_`/`into_`/`from_` 约定
- [ ] Getter 不使用 `get_` 前缀
- [ ] 复杂构造使用 Builder 模式而非长参数列表
- [ ] pub 函数都有文档注释（`///`），包含 `# Errors` 和 `# Examples`
- [ ] 错误类型是 `Send + Sync + 'static`
- [ ] 不暴露实现细节（如内部使用的第三方类型不出现在公共签名中）
