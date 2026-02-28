# rust-performance

Rust 性能分析与优化：基准测试、profiling、常见优化模式。

## 适用场景

- 需要对关键路径进行性能基准测试
- 发现性能瓶颈需要分析
- 优化已确认的热点代码

## 工作流程

### 第一步：建立基准测试

使用 `criterion` 编写基准测试（Rust 社区标准）：

**添加依赖（在 crate 的 Cargo.toml 中）：**

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "bench_main"
harness = false
```

**编写基准测试（`benches/bench_main.rs`）：**

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse(c: &mut Criterion) {
    let input = include_str!("../testdata/sample.oml");

    c.bench_function("parse_sample", |b| {
        b.iter(|| {
            let result = wf_lang::parse(black_box(input));
            black_box(result)
        })
    });
}

fn bench_with_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_by_size");
    for size in [10, 100, 1000, 10000] {
        let input = generate_input(size);
        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(size),
            &input,
            |b, input| b.iter(|| wf_lang::parse(black_box(input))),
        );
    }
    group.finish();
}

criterion_group!(benches, bench_parse, bench_with_sizes);
criterion_main!(benches);
```

**运行基准测试：**

```bash
cargo bench -p <crate>
```

### 第二步：Profiling（定位瓶颈）

**macOS（Instruments）：**

```bash
# 编译 release + debug info
cargo build --release -p <crate>

# 使用 cargo-instruments（需安装）
cargo install cargo-instruments
cargo instruments -t "Time Profiler" --release -p <crate> --bench bench_main
```

**通用（flamegraph）：**

```bash
cargo install flamegraph

# 生成火焰图
cargo flamegraph --bench bench_main -p <crate> -- --bench
```

**编译时间分析：**

```bash
# 查看编译各阶段耗时
cargo build --timings --release

# 查看哪些 crate 编译慢
cargo build --timings 2>&1
```

### 第三步：常见优化模式

#### 内存分配优化

| 问题 | 优化 | 说明 |
|------|------|------|
| 频繁小字符串分配 | `SmallVec` / `CompactString` | 栈上存储小数据 |
| 循环中重复分配 Vec | 循环外 `Vec::with_capacity()` + 循环内 `clear()` 复用 | 避免反复分配 |
| 大量字符串拼接 | `String::with_capacity()` + `push_str()` | 避免多次 realloc |
| HashMap 已知大小 | `HashMap::with_capacity(n)` | 避免 rehash |
| 不需要所有权 | `&str` 代替 `String`，`Cow<str>` 按需克隆 | 减少拷贝 |

#### 算法和数据结构

| 场景 | 考虑 |
|------|------|
| 频繁查找 | `HashMap` / `HashSet`（`FxHashMap` 更快） |
| 有序遍历 + 查找 | `BTreeMap` / `BTreeSet` |
| 小集合（<16 元素） | `Vec` + 线性搜索（缓存友好） |
| 频繁插入/删除 | `VecDeque`（两端）或 `LinkedList`（极少用） |
| 字符串 intern | `string_interner` crate |

#### 零成本抽象

```rust
// 迭代器链 vs 手写循环 — 编译后性能相同
// 优先选择可读性更好的写法

// Good: 迭代器链（通常更清晰）
let sum: i64 = items.iter()
    .filter(|item| item.is_valid())
    .map(|item| item.value())
    .sum();

// 也 Good: 手写循环（复杂逻辑时可能更清晰）
let mut sum = 0i64;
for item in &items {
    if item.is_valid() {
        sum += item.value();
    }
}
```

#### 编译器优化提示

```rust
// 标记不太可能的分支（nightly 或用 likely_stable crate）
#[cold]
fn handle_error() { ... }

// 避免边界检查（仅在确认安全时）
// 优先使用迭代器（自动消除边界检查）而非索引访问

// 内联小函数
#[inline]
fn is_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}
```

### 第四步：验证优化效果

```bash
# 运行基准测试对比
cargo bench -p <crate>

# criterion 会自动和上次结果对比，显示：
# - 改善/恶化百分比
# - 置信区间
# - 是否统计显著
```

## 性能优化原则

1. **先测量，后优化** — 不要凭直觉优化，用 benchmark/profiling 数据说话
2. **优化热点** — 80% 的时间花在 20% 的代码上，找到那 20%
3. **算法优先** — O(n) → O(log n) 比微优化更有效
4. **减少分配** — 堆分配是 Rust 中最常见的性能瓶颈
5. **编译器很聪明** — 大多数"手动优化"编译器已经做了，先确认确实有必要
6. **Release 模式测试** — Debug 模式性能没有参考意义

## 注意事项

- 基准测试使用 `black_box()` 防止编译器优化掉目标代码
- 不要在 CI 中运行 benchmark 做性能回归检测（环境不稳定），用专门的性能测试环境
- 微优化之前先检查是否有算法层面的改进空间
- `unsafe` 优化需要非常充分的理由和极其谨慎的审查
