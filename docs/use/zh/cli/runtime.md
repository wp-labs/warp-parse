# `wparse` 运行时使用指南

## 适用范围

本文说明 `wparse` 这个主运行时二进制在日常使用中的定位、常见命令和适用场景。

如果你要做的是：

- 在线状态查询和 reload：看 [../operations/admin.md](../operations/admin.md)
- 工程初始化、检查和模型查看：看 [project.md](project.md)
- 测试数据生成：看 [generator.md](generator.md)

## 命令定位

`wparse` 是真正执行解析引擎的入口，目前只暴露两个模式：

- `daemon`：常驻模式，适合在线接入
- `batch`：批处理模式，适合离线回放和联调验证

## 常见命令

查看帮助：

```bash
wparse --help
```

启动常驻实例：

```bash
wparse daemon --work-root .
```

执行批处理：

```bash
wparse batch --work-root .
```

## 常用参数

- `--work-root`：工程根目录
- `-n, --max-line`：限制本次最多处理的行数
- `-w, --parse-workers`：指定解析 worker 数
- `--stat`：设置统计输出周期
- `-p, --print_stat`：打印统计信息
- `--robust`：设置异常处理策略
- `--log-profile`：覆盖日志配置
- `--wpl`：临时覆盖 WPL 规则目录

## `daemon` 与 `batch` 的选择

优先使用 `daemon` 的情况：

- 需要长期接收日志或事件
- 需要配合 `wproj engine status` 和 `wproj engine reload`
- 需要在线热更新规则或模型

优先使用 `batch` 的情况：

- 本地规则联调
- 对样本数据做一次性回放
- 在上线前做离线验证

## 使用约束

- 管理面只在 `daemon` 模式下可用
- `batch` 更适合测试和离线任务，不承担在线 reload
- 当前没有额外的 `work` 子命令，只暴露 `daemon` 和 `batch`

## 典型流程

### 本地离线验证

1. 准备工程目录和规则
2. 使用 `wparse batch --work-root .` 运行一次处理
3. 再配合 `wproj data stat` 或 `wproj model route` 检查结果

### 在线运行

1. 使用 `wparse daemon --work-root .` 启动服务
2. 使用 `wproj engine status` 检查状态
3. 需要变更模型时，使用 `wproj engine reload`

## 相关文档

- 功能与 CLI 使用指南: [index.md](index.md)
- 运行时管理面使用说明: [../operations/admin.md](../operations/admin.md)
- 项目工具使用指南: [project.md](project.md)
