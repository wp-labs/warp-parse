# rust-add-module

在指定 crate 下新增模块：创建文件、注册 mod、添加基本结构和测试骨架。

## 输入

用户需要提供：
1. 目标 crate 名称（如 `wf-core`、`wf-lang`）
2. 模块名称
3. 模块用途简述

如果用户未提供，通过提问确认。

## 工作流程

### 第一步：确认目标位置

```bash
ls crates/<crate-name>/src/
```

了解现有模块结构，确认新模块应放在哪一层（顶层 mod 还是子模块）。

### 第二步：创建模块文件

根据模块复杂度决定结构：

- **简单模块**：创建 `crates/<crate>/src/<module>.rs`
- **复杂模块**：创建 `crates/<crate>/src/<module>/mod.rs`，按需拆分子文件

### 第三步：注册模块

在父模块（通常是 `lib.rs` 或 `main.rs`）中添加 `mod` 声明：

```rust
pub mod <module>;
```

遵循现有代码的可见性风格（`pub mod` vs `mod` vs `pub(crate) mod`）。

### 第四步：添加基本结构

根据模块用途创建骨架代码，通常包含：

- 核心 struct / enum 定义
- 必要的 `impl` 块
- 必要的 trait 实现
- 如果模块需要对外暴露，在 `lib.rs` 中添加 `pub use` re-export

### 第五步：添加测试骨架

在模块文件底部添加测试模块：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder() {
        // TODO: 根据实际功能补充测试
    }
}
```

### 第六步：验证编译

```bash
cargo check -p <crate-name> 2>&1
```

确保新模块能通过编译。

## 注意事项

- 遵循项目现有的命名风格（snake_case 模块名）
- 查看相邻模块的 `use` 导入风格并保持一致
- edition 2024 不需要 `mod.rs` 来声明子模块目录，但要检查项目现有风格
- 如果 crate 间有依赖关系，可能需要在 `Cargo.toml` 中添加依赖
- 不要过度设计——先创建最小可用的骨架，让用户在此基础上扩展
