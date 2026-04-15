# Warp Parse 功能与 CLI 使用指南

## 适用范围

本文作为使用类文档的总导航，帮助使用者快速理解几个核心二进制的分工，并跳转到对应的专题页。

## 工具分工

Warp Parse 当前主要提供 4 个可执行入口：

- `wparse`：主运行时，负责真正执行解析引擎
- `wpgen`：生成器工具，负责初始化和运行测试数据生成
- `wproj`：项目管理工具，负责工程初始化、检查、模型查看、运行时管理
- `wprescue`：rescue 数据处理入口，负责对 rescue 数据链路做独立处理

## 建议阅读顺序

1. 先看 [../overview/product.md](../overview/product.md)，理解产品定位和组件关系
2. 再看下面几篇 CLI 专题页，按角色理解具体工具
3. 涉及在线实例时，再看 [../operations/admin.md](../operations/admin.md)
4. 涉及远端版本同步和热更新时，再看 [../operations/project-sync.md](../operations/project-sync.md)

## CLI 专题页

- `wparse` 运行时使用指南: [runtime.md](runtime.md)
- `wpgen` 生成器使用指南: [generator.md](generator.md)
- `wproj` 项目工具使用指南: [project.md](project.md)
- `wprescue` 与 rescue 数据使用指南: [rescue.md](rescue.md)

## 快速入口

查看帮助：

```bash
wparse --help
wpgen --help
wproj --help
wprescue --help
```

本地开发联调的常见顺序：

1. `wproj init`
2. `wpgen conf init` / `wpgen conf check`
3. `wpgen rule` 或 `wpgen sample`
4. `wproj check --what wpl`
5. `wparse batch --work-root .`
6. `wproj model route` 与 `wproj data stat`

线上运维的常见顺序：

1. `wproj conf update`
2. `wproj check --what wpl --fail-fast`
3. `wproj engine status`
4. `wproj engine reload`
5. `wproj engine status`
6. 如有异常，再看 `wproj rescue stat`

## 当前边界

- `wparse` 当前没有单独的 `work` 子命令，只暴露 `daemon` 和 `batch`
- `wprescue` 当前只支持 `batch`
- `wproj rule` 目前实际暴露的是 `parse` 能力，尚不是完整的规则工作台
- 更细的配置字段说明，仍应以对应配置文件和专项文档为准

## 相关文档

- 产品概览: [../overview/product.md](../overview/product.md)
- 运行时管理面使用说明: [../operations/admin.md](../operations/admin.md)
- 远程工程拉取与规则热更新 SOP: [../operations/project-sync.md](../operations/project-sync.md)
- 对应英文版总览: [../../en/cli/index.md](../../en/cli/index.md)
