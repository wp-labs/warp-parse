# rust-dependency-audit

依赖管理与安全审计：最小化依赖、审计已知漏洞、管理 feature flags。

## 工作流程

### 第一步：审计已知漏洞

```bash
# 安装（首次）
cargo install cargo-audit

# 扫描已知安全漏洞
cargo audit
```

如果发现漏洞，按严重程度分级：
- **Critical/High**：立即升级或替换
- **Medium**：评估是否影响项目使用场景
- **Low/Informational**：计划中处理

### 第二步：检查依赖树

```bash
# 查看完整依赖树
cargo tree

# 查看某个依赖被谁引入
cargo tree -i <crate-name>

# 查找重复依赖（不同版本共存）
cargo tree -d
```

**重复依赖处理：**
- 如果两个版本共存（如 `syn 1.x` 和 `syn 2.x`），检查是否可以统一
- 在 `[workspace.dependencies]` 中统一管理公共依赖版本

### 第三步：最小化 feature flags

```bash
# 查看 crate 的可用 features
cargo metadata --format-version 1 | jq '.packages[] | select(.name == "<crate>") | .features'
```

**原则：只启用实际使用的 feature。**

```toml
# Bad: 默认 features 可能引入不需要的依赖
serde = "1.0"

# Good: 只启用需要的 features
serde = { version = "1.0", features = ["derive"] }

# Good: 禁用默认 features，按需启用
some-crate = { version = "1.0", default-features = false, features = ["needed-feature"] }
```

### 第四步：使用 cargo-deny 做全面检查

```bash
# 安装
cargo install cargo-deny

# 初始化配置
cargo deny init
```

**`deny.toml` 推荐配置：**

```toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"

[licenses]
unlicensed = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-3.0",
]

[bans]
multiple-versions = "warn"    # 警告重复依赖
wildcards = "deny"            # 禁止通配符版本

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

```bash
# 运行全面检查
cargo deny check
```

### 第五步：依赖更新策略

```bash
# 查看可更新的依赖
cargo update --dry-run

# 更新所有兼容版本（patch/minor）
cargo update

# 查看过时的依赖（需要 cargo-outdated）
cargo install cargo-outdated
cargo outdated -R  # 仅直接依赖
```

**更新原则：**
- Patch 更新（`1.0.1` → `1.0.2`）：安全，直接更新
- Minor 更新（`1.0` → `1.1`）：一般安全，但需测试
- Major 更新（`1.x` → `2.x`）：可能有 breaking changes，需要逐个评估

### 第六步：Cargo.toml 最佳实践

**workspace 级别统一管理依赖版本：**

```toml
# 根 Cargo.toml
[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
anyhow = "1.0"

# crate 的 Cargo.toml
[dependencies]
serde = { workspace = true }
tokio = { workspace = true }
```

**版本约束写法：**

```toml
# 推荐：指定最低兼容版本
anyhow = "1.0"          # 等同于 >=1.0.0, <2.0.0

# 精确版本（仅在确实需要时）
foo = "=1.2.3"

# 避免：过于宽泛
bar = "*"                # Bad: 任意版本
```

## 审查清单

- [ ] `cargo audit` 无 Critical/High 漏洞
- [ ] 无不必要的重复依赖（`cargo tree -d`）
- [ ] 所有依赖使用最小 feature set
- [ ] 公共依赖在 `[workspace.dependencies]` 中统一版本
- [ ] 无通配符版本（`*`）
- [ ] 定期更新依赖（至少每月一次 `cargo update`）
- [ ] 许可证兼容（建议使用 `cargo-deny` 自动检查）

## 添加新依赖的决策框架

在引入新依赖前考虑：

1. **是否真的需要？** — 如果只用了几个函数，考虑自己实现
2. **维护状态如何？** — 检查 GitHub stars、最近更新时间、issue 响应
3. **依赖传递大小？** — `cargo tree -p <new-dep>` 看会引入多少传递依赖
4. **编译时间影响？** — proc-macro crate（如 `syn`）会显著增加编译时间
5. **许可证兼容？** — 确认与项目许可证兼容
