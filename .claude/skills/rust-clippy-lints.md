# rust-clippy-lints

配置和使用 Clippy 进行代码质量控制：lint 分级、项目级配置、常见 lint 处理。

## 适用场景

- 配置项目级别的 Clippy lint 规则
- 分析和修复 Clippy 警告
- 了解特定 lint 的含义和最佳处理方式

## 工作流程

### 第一步：运行 Clippy

```bash
# 基本检查（项目标准）
cargo clippy --workspace -- -D warnings 2>&1

# 包含更严格的 lint
cargo clippy --workspace -- -W clippy::pedantic 2>&1

# 查看所有可用 lint
cargo clippy --workspace -- -W clippy::all -W clippy::pedantic -W clippy::nursery 2>&1
```

### 第二步：项目级 Clippy 配置

在 workspace 根目录创建 `clippy.toml`：

```toml
# 允许的通配符导入模块（如 prelude）
allowed-wildcard-imports = ["prelude"]

# 认知复杂度阈值
cognitive-complexity-threshold = 30

# 函数参数过多阈值
too-many-arguments-threshold = 8

# 类型复杂度阈值
type-complexity-threshold = 300
```

在 `lib.rs` / `main.rs` 顶部配置 lint 级别：

```rust
// 推荐的项目级 lint 配置
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
// 按需允许某些过于严格的 pedantic lint
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
```

### 第三步：Lint 分级策略

| Lint 组 | 级别 | 说明 |
|---------|------|------|
| `clippy::correctness` | deny | 几乎确定是 bug |
| `clippy::suspicious` | warn | 可疑的代码模式 |
| `clippy::style` | warn | 不符合惯例的风格 |
| `clippy::complexity` | warn | 可以简化的代码 |
| `clippy::perf` | warn | 性能问题 |
| `clippy::pedantic` | 可选 warn | 更严格的惯例 |
| `clippy::nursery` | 可选 | 实验性 lint |
| `clippy::restriction` | 按需 | 极度限制性（不要整体开启） |

### 第四步：常见 Lint 及处理

#### 值得修复的 Lint

```rust
// clippy::needless_return — 不必要的 return
// Bad
fn foo() -> i32 { return 42; }
// Good
fn foo() -> i32 { 42 }

// clippy::redundant_clone — 不必要的 clone
// Bad
let s = String::from("hello");
let s2 = s.clone();
drop(s);
// Good
let s = String::from("hello");
let s2 = s;

// clippy::manual_map — 手动实现 Option::map
// Bad
match opt {
    Some(x) => Some(x + 1),
    None => None,
}
// Good
opt.map(|x| x + 1)

// clippy::implicit_clone — 隐式克隆
// Bad
let s2 = s.to_string(); // s 已经是 String
// Good
let s2 = s.clone();

// clippy::uninlined_format_args — 格式字符串中内联变量
// Bad
format!("hello {}", name)
// Good
format!("hello {name}")
```

#### 合理抑制的 Lint

```rust
// 抑制单个表达式
#[allow(clippy::cast_possible_truncation)]
let byte = value as u8;

// 抑制函数级别
#[allow(clippy::too_many_arguments)]
fn complex_constructor(/* ... */) { }

// 抑制时 **必须** 说明原因
#[allow(clippy::cast_possible_truncation)] // value is guaranteed < 256 by prior validation
let byte = value as u8;
```

### 第五步：自动修复

```bash
# 自动应用 Clippy 建议的修复
cargo clippy --fix --workspace --allow-dirty

# 仅应用确定安全的修复
cargo clippy --fix --workspace
```

### 第六步：CI 集成

```bash
# CI 中推荐的命令：失败即阻止合并
cargo clippy --workspace --all-targets -- -D warnings
```

`--all-targets` 包含测试、bench、example 中的代码。

## 推荐的 Pedantic Lint

以下 `clippy::pedantic` lint 值得开启：

| Lint | 说明 |
|------|------|
| `needless_pass_by_value` | 参数不需要所有权时应改为引用 |
| `redundant_closure_for_method_calls` | `.map(\|x\| x.foo())` → `.map(Type::foo)` |
| `cloned_instead_of_copied` | `.cloned()` 对 Copy 类型应改为 `.copied()` |
| `flat_map_option` | `.filter_map(\|x\| x)` → `.flatten()` |
| `explicit_iter_loop` | `for x in v.iter()` → `for x in &v` |
| `manual_let_else` | 手动 match + return → `let ... else` |
| `unnested_or_patterns` | `A \| B \| C` 替代多个 match arm |
| `semicolon_if_nothing_returned` | 无返回值的表达式末尾加分号 |

以下 pedantic lint 可能过于严格，按需允许：

| Lint | 允许原因 |
|------|----------|
| `module_name_repetitions` | `config::ConfigError` 的命名是合理的 |
| `must_use_candidate` | 不是所有返回值都需要 `#[must_use]` |
| `missing_errors_doc` | 私有模块不需要 `# Errors` 文档 |
| `missing_panics_doc` | 某些 panic 是不可达的 |

## 审查清单

- [ ] `cargo clippy --workspace -- -D warnings` 零警告
- [ ] 所有 `#[allow(clippy::...)]` 都附有注释说明原因
- [ ] 项目有统一的 lint 配置（`lib.rs` 顶部或 `clippy.toml`）
- [ ] CI 中包含 Clippy 检查
