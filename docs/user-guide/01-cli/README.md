# 命令行工具
<!-- 角色：使用配置者 | 最近验证：2025-12-12 -->

本文档介绍 warp-parse 系统中各种命令行工具的使用方法。

## 工具概览

| 工具 | 用途 | 子命令 |
|------|------|--------|
| **wparse** | 数据解析引擎 | `daemon` / `batch` |
| **wpgen** | 数据生成器 | `rule` / `sample` / `conf` / `data` |
| **wproj** | 项目管理工具 | `rule` / `init` / `check` / `data` / `model` |
| **wprescue** | 数据恢复工具 | `batch` |

## 快速参考

```bash
# wparse - 数据解析
wparse daemon --stat 5 -p              # 常驻服务模式
wparse batch -n 3000 --stat 2 -p       # 批处理模式

# wpgen - 数据生成
wpgen conf init -w .                   # 初始化配置
wpgen rule -n 10000 -p                 # 基于规则生成
wpgen sample -n 5000 -s 1000 -p        # 基于样本生成

# wproj - 项目管理
wproj init -w . --mode full            # 初始化完整项目
wproj check -w . --what all            # 全面检查
wproj data stat file                   # 统计数据
wproj model sources                    # 列出源连接器

# wprescue - 数据恢复
wprescue batch --work-root /project    # 批处理恢复
```

## 文档导航

- [wparse 运行模式](02-run_modes.md) - daemon/batch 模式详解、退出策略
- [wpgen CLI](03-wpgen.md) - 数据生成器完整参数说明
- [wproj CLI](04-wproj.md) - 项目管理工具完整参数说明
- [wprescue CLI](05-wprescue.md) - 数据恢复工具使用指南
- [日志配置](06-logging.md) - 日志系统配置说明
- [快速入门](01-getting_started.md) - 基于用例的配置指南

## 通用参数

所有工具共享的通用参数：

| 参数 | 短选项 | 长选项 | 说明 |
|------|--------|--------|------|
| work_root | `-w` | `--work-root` | 工作根目录（默认 `.`） |
| quiet | `-q` | `--quiet` | 隐藏启动 Banner |
| stat_print | `-p` | `--print_stat` | 周期打印统计信息 |
| stat_sec | - | `--stat` | 统计输出间隔（秒） |

## 退出码

| 代码 | 含义 |
|------|------|
| 0 | 成功 |
| 2 | 参数错误 |
| 3 | 执行错误 |
| 4 | 校验失败 |

## 技术栈

- **异步运行时**: Tokio（多线程）
- **内存分配器**: jemalloc（高性能多线程）
- **CLI 框架**: Clap
- **可选连接器**: Kafka, MySQL, ClickHouse, Elasticsearch, Prometheus
