# cargo-check-all

对整个 workspace 执行完整的 Rust 代码质量检查，依次运行编译检查、clippy 静态分析和全量测试，汇总报告结果。

## 工作流程

依次执行以下三个阶段，任何阶段失败时分析原因并给出修复建议：

### 阶段 1：编译检查

```bash
cargo check --workspace 2>&1
```

### 阶段 2：Clippy 静态分析

```bash
cargo clippy --workspace -- -D warnings 2>&1
```

### 阶段 3：全量测试

```bash
cargo test --workspace 2>&1
```

## 结果汇总

所有阶段执行完毕后，输出汇总表格：

| 阶段 | 状态 | 备注 |
|------|------|------|
| cargo check | PASS/FAIL | 编译错误数 |
| cargo clippy | PASS/FAIL | 警告数 |
| cargo test | PASS/FAIL | 通过/失败/忽略数 |

## 失败处理

- 如果 clippy 报出警告，列出所有警告并提供逐条修复方案
- 如果测试失败，区分是已知的 flaky test（如 test_sql_debug 并发竞争）还是真正的回归
- 编译失败时直接展示错误并分析原因
