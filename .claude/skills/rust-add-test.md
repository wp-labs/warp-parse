# rust-add-test

为指定函数或模块编写单元测试和集成测试。

## 输入

用户需要提供：
1. 要测试的函数/模块/类型名称或文件路径
2. 可选：需要覆盖的具体场景

如果用户只给了模糊描述（如"给 parser 加测试"），先阅读代码理解后再设计测试用例。

## 工作流程

### 第一步：理解待测代码

阅读目标代码，分析：
- 函数签名（输入类型、返回类型、是否返回 Result）
- 边界条件（空输入、极端值、错误路径）
- 依赖关系（是否需要 mock 或构造复杂上下文）
- 现有测试（避免重复）

```bash
# 查看现有测试
cargo test -p <crate> -- --list 2>&1 | grep <module>
```

### 第二步：设计测试用例

为每个函数设计测试矩阵：

| 类别 | 用例 | 预期结果 |
|------|------|----------|
| 正常路径 | 典型输入 | 正确输出 |
| 边界条件 | 空输入/极值 | 合理行为 |
| 错误路径 | 非法输入 | 返回 Err 或 panic |

### 第三步：编写单元测试

在目标模块文件的底部添加或扩展 `#[cfg(test)]` 模块：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_normal_case() {
        let result = target_function(valid_input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_function_edge_case() {
        let result = target_function(edge_input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_function_error_case() {
        let result = target_function(invalid_input);
        assert!(result.is_err());
    }
}
```

**命名规范：**
- 测试函数名：`test_<函数名>_<场景>`
- 使用描述性名称，让失败信息一目了然

### 第四步：编写集成测试（如需要）

如果需要跨模块或端到端测试，在 `crates/<crate>/tests/` 目录下创建测试文件：

```rust
// crates/<crate>/tests/integration_test.rs
use <crate>::<module>::<function>;

#[test]
fn test_integration_scenario() {
    // 构造环境
    // 执行操作
    // 验证结果
}
```

### 第五步：运行测试

```bash
# 运行新增的测试
cargo test -p <crate> <test_name> -- --nocapture 2>&1

# 确认全量测试无回归
cargo test -p <crate> 2>&1
```

### 第六步：检查覆盖情况

确认测试覆盖了：
- [ ] 正常路径（happy path）
- [ ] 边界条件
- [ ] 错误处理路径
- [ ] 返回值的所有变体（如 enum 的所有 variant）

## 测试工具箱

### 断言宏
- `assert_eq!(actual, expected)` — 相等比较
- `assert_ne!(actual, unexpected)` — 不等比较
- `assert!(condition)` — 布尔条件
- `assert!(result.is_ok())` / `assert!(result.is_err())` — Result 检查
- `assert!(matches!(value, Pattern))` — 模式匹配

### 测试辅助
- `#[should_panic(expected = "message")]` — 预期 panic
- `#[ignore]` — 暂时跳过（附注释说明原因）
- 使用 `indoc!` 宏编写多行字符串测试输入（如果项目已依赖 indoc）

### 构造测试数据
- 优先使用 builder pattern 或工厂函数构造复杂测试数据
- 将通用的测试 fixture 提取到 `mod tests` 内的辅助函数中
- 避免在测试中硬编码大段文本——使用 `include_str!` 加载测试 fixture 文件

## 注意事项

- 测试应该是确定性的，避免依赖系统时间、随机数或文件系统状态
- 每个测试只验证一个行为点
- 测试名称要能说明"测什么"和"预期什么"
- 不要为了凑覆盖率写无意义的测试——每个测试应该验证一个有价值的行为
- 遵循项目现有测试的风格和组织方式
