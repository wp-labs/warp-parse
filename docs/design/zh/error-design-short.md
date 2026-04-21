# Error Design Short Guide

更新时间：2026-04-22

这是一份可直接放进仓库 `README` / `CONTRIBUTING` 的短版错误设计规范。

## 目标

- 错误必须有稳定分类
- 能保留 source 时必须保留 source
- 错误只能在边界处转换，不能在中间层反复压平

## 标准做法

### 1. 优先返回结构化错误

- 返回 `RunResult<T>` 或领域自己的结构化结果
- 不要返回 `Result<T, String>`

### 2. 正确选择 reason

- `RunReason::from_conf()`
  - 配置缺失
  - 配置值非法
  - 输入参数不满足声明式约束
- `RunReason::from_biz()`
  - 业务状态冲突
  - 请求被拒绝
  - 当前时机不允许执行操作
- `RunReason::from_logic()`
  - 内部结构异常
  - 运行约束不满足
  - 不支持的路径 / 文件类型 / 平台分支

### 3. 原始标准错误走 `into_as(...)` / `with_std_source(...)`

对 `io` / `serde_json` / `reqwest` / `git2` 等标准错误，优先使用：

```rust
op()
    .into_as(RunReason::from_conf(), "load config failed")
    .with(path)
    .doing("load config")?;
```

如果是手工构造结构化错误，就显式保留 raw source：

```rust
RunReason::from_conf()
    .to_err()
    .with_detail("encode state failed")
    .with_std_source(err);
```

### 4. 已结构化错误走 `wrap_as(...)` / `wrap(...)` / `with_struct_source(...)`

如果上游已经是 `StructError<_>`，不要再把它当普通 source 挂进去。

```rust
upstream()
    .wrap_as(RunReason::from_conf(), "load engine config failed")?;
```

或：

```rust
RunReason::from_conf()
    .to_err()
    .with_detail("load engine config failed")
    .with_struct_source(source);
```

### 5. 非 `StdError` 错误不要硬套 source

如果底层错误不能安全保留 source：

- 直接选定 `reason`
- 把底层错误文本放进 `detail`
- 补齐上下文

```rust
parser.parse(data).map_err(|err| {
    RunReason::from_conf()
        .to_err()
        .with_detail(format!("invalid syntax: {}", err))
        .with(file)
})?;
```

### 6. `with(...)` 和 `doing(...)` 必须有

- `with(...)`：补“对谁失败”
- `doing(...)`：补“做什么失败”

推荐组合：

```rust
op()
    .into_as(RunReason::from_conf(), "read engine config failed")
    .with(path)
    .doing("read engine config")?;
```

## 真实链路

历史主链是：

`owe_conf_source -> with -> want -> err_conv -> print_run_error`

当前收敛后的主链是：

- raw `StdError`
  - `into_as(...)` / `with_std_source(...)`
  - `with(...)`
  - `doing(...)`
  - `print_run_error(...)`
- structured `StructError<_>`
  - `wrap_as(...)` / `wrap(...)` / `with_struct_source(...)`
  - `with(...)`
  - `doing(...)`
  - `print_run_error(...)`

`err_conv()` 现在只保留给“已可稳定转换的结构化错误”做跨模块透传，不再作为所有错误的统一入口。

## 反模式

禁止以下写法：

- `Result<T, String>`
- 对已经是结构化错误的结果再走 raw-source 路径
- 用一个泛化 helper 把所有错误都压成“配置错误”
- 只写 `"xxx failed"`，不带对象和操作上下文

## 当前状态

截至 2026-04-22：

- 生产代码中的 `.owe_conf_source()` / `.owe_source(...)` / `.err_wrap(...)` 已清零
- 主路径已切换为：
  - 结构化错误：`wrap_as(...)` / `wrap(...)` / `with_struct_source(...)`
  - 原始标准错误：`into_as(...)` 或显式 `with_std_source(...)`
- 顶层统一出口保持为：

```rust
print_run_error("wparse", &e);
```
