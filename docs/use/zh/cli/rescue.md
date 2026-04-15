# `wprescue` 与 rescue 数据使用指南

## 适用范围

本文覆盖两件事：

- 如何使用 `wprescue` 处理 rescue 数据流程
- 如何使用 `wproj rescue stat` 统计 rescue 目录

## `wprescue` 的定位

`wprescue` 是面向 rescue 数据链路的独立处理入口。

当前只支持 `batch` 方式运行，不支持 `daemon`。

查看帮助：

```bash
wprescue --help
```

运行方式：

```bash
wprescue batch --work-root .
```

常用参数与 `wparse batch` 基本一致：

- `--work-root`
- `-n, --max-line`
- `-w, --parse-workers`
- `--stat`
- `-p, --print_stat`
- `--robust`
- `--log-profile`
- `--wpl`

使用约束：

- 执行 `wprescue daemon` 会直接报错退出
- 更适合处理 rescue 数据，而不是承担普通主流程解析

## rescue 目录统计

如果目标只是看 rescue 目录里的数据规模和分布，应优先使用 `wproj rescue stat`。

统计 rescue 目录：

```bash
wproj rescue stat --work-root .
```

查看明细：

```bash
wproj rescue stat --work-root . --detail
```

输出 JSON：

```bash
wproj rescue stat --work-root . --json
```

如果 rescue 目录不在默认位置，可显式覆盖：

```bash
wproj rescue stat --work-root . --rescue-path ./data/rescue
```

## 适用场景

- 排查异常数据是否进入 rescue
- 粗看不同 sink 的失败分布
- 给补偿处理或回放处理准备基础统计
- 单独回收和处理 rescue 链路中的数据

## 推荐做法

1. 先用 `wproj rescue stat` 看整体规模
2. 需要看单文件分布时，加 `--detail`
3. 真正要处理 rescue 数据时，再执行 `wprescue batch`

## 相关文档

- 功能与 CLI 使用指南: [index.md](index.md)
- 项目工具使用指南: [project.md](project.md)
- `wparse` 运行时使用指南: [runtime.md](runtime.md)
