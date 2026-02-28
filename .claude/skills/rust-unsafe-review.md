# rust-unsafe-review

审查 unsafe 代码块：验证安全不变量、检查 UB 风险、确保文档完整。

## 适用场景

- 审查现有 unsafe 代码的正确性
- 需要编写新的 unsafe 代码
- 检查项目中是否有不必要的 unsafe

## 工作流程

### 第一步：定位 unsafe 使用

```bash
# 查找所有 unsafe 块
grep -rn 'unsafe' crates/ --include="*.rs"

# 统计 unsafe 使用量
grep -c 'unsafe' crates/*/src/**/*.rs

# 使用 cargo-geiger 分析（需安装）
cargo install cargo-geiger
cargo geiger
```

### 第二步：分类审查

每个 unsafe 块按以下类别分类：

| 类别 | 常见场景 | 风险等级 |
|------|----------|----------|
| FFI 调用 | 调用 C 库函数 | 中-高 |
| 裸指针操作 | `*const T` / `*mut T` 解引用 | 高 |
| 类型转换 | `transmute`、`from_raw_parts` | 高 |
| 全局可变状态 | `static mut` | 高 |
| 实现 unsafe trait | `Send`、`Sync` 手动实现 | 中 |
| 性能优化 | 跳过边界检查 `get_unchecked` | 中 |

### 第三步：验证安全不变量

对每个 unsafe 块检查以下内容：

**1. 是否有 `// SAFETY:` 注释？**

```rust
// Good: 说明为什么这是安全的
// SAFETY: `ptr` is guaranteed non-null and properly aligned because
// it was obtained from `Box::into_raw` in the same function.
let value = unsafe { *ptr };

// Bad: 没有解释
let value = unsafe { *ptr };
```

**2. 不变量是否在所有路径上成立？**

- 指针非空且已对齐
- 引用的生命周期有效
- 类型的内存布局匹配
- 不存在数据竞争
- 不违反借用规则（aliasing XOR mutation）

**3. 是否可以替换为 safe 代码？**

| unsafe 用法 | safe 替代 |
|-------------|-----------|
| `slice::from_raw_parts` | 使用 safe 切片操作 |
| `get_unchecked` | 使用 `.get()` 或迭代器 |
| `transmute` | `as` 转换、`From`/`Into` |
| `static mut` | `std::sync::OnceLock`、`Mutex`、`AtomicXxx` |
| 手动 `Send`/`Sync` | 验证是否真的需要，或改用 safe wrapper |

### 第四步：检查常见 UB 模式

**1. 悬垂指针（Dangling Pointer）**
```rust
// UB: 使用已释放的内存
let ptr = Box::into_raw(Box::new(42));
unsafe { drop(Box::from_raw(ptr)); }
let val = unsafe { *ptr };  // UB!
```

**2. 数据竞争（Data Race）**
```rust
// UB: 多线程同时读写 static mut
static mut COUNTER: u32 = 0;
// 在多线程中操作 COUNTER 是 UB
```

**3. 无效的类型转换**
```rust
// UB: bool 只能是 0 或 1
let b: bool = unsafe { std::mem::transmute(2u8) };  // UB!
```

**4. 违反 aliasing 规则**
```rust
// UB: 同时存在可变引用和不可变引用
let mut x = 42;
let ptr = &mut x as *mut i32;
let ref_x = &x;
unsafe { *ptr = 0; }  // UB: ref_x 还活着
println!("{}", ref_x);
```

### 第五步：使用工具验证

```bash
# Miri — 检测未定义行为（nightly only）
rustup +nightly component add miri
cargo +nightly miri test

# Address Sanitizer（检测内存错误）
RUSTFLAGS="-Z sanitizer=address" cargo +nightly test

# 运行常规测试确保功能不变
cargo test --workspace 2>&1
```

### 第六步：编写 unsafe 代码的正确方式

如果确实需要 unsafe，遵循以下规范：

```rust
/// 从裸指针构建切片。
///
/// # Safety
///
/// 调用者必须确保：
/// - `ptr` 非空且已正确对齐
/// - `ptr` 指向的内存区域包含至少 `len` 个连续的有效 `T` 值
/// - 返回的切片生命周期内，`ptr` 指向的内存不被修改
pub unsafe fn make_slice<'a, T>(ptr: *const T, len: usize) -> &'a [T] {
    debug_assert!(!ptr.is_null());
    debug_assert!(ptr.is_aligned());
    // SAFETY: 由调用者保证 ptr 有效、对齐、内存可读
    unsafe { std::slice::from_raw_parts(ptr, len) }
}
```

**关键要素：**
- `# Safety` 文档节：列出调用者必须满足的前置条件
- `// SAFETY:` 行注释：解释此处为何满足安全条件
- `debug_assert!`：在 debug 模式下检查不变量
- unsafe 块尽量小，只包裹真正 unsafe 的操作

## 审查清单

- [ ] 每个 `unsafe` 块都有 `// SAFETY:` 注释说明理由
- [ ] 每个 `unsafe fn` 都有 `# Safety` 文档说明前置条件
- [ ] 已检查是否存在 safe 替代方案
- [ ] 不变量在所有代码路径上成立（包括 panic 和 early return）
- [ ] 关键 debug_assert! 检查就位
- [ ] 如果可行，用 Miri 跑过测试
- [ ] unsafe 块范围最小化（不包含不必要的 safe 代码）
