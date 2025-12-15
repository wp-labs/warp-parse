# wprescue CLI
<!-- 角色：使用配置者 | 最近验证：2025-12-12 -->

wprescue 是数据恢复工具，用于从救援目录中恢复数据并按照项目配置的 Sink 路由输出到目标。

## 命令概览

```
wprescue <COMMAND>

Commands:
  batch   批处理模式（仅支持此模式）
  daemon  守护进程模式（不支持，会报错退出）
```

**重要：** wprescue 仅支持 batch 模式。尝试使用 daemon 模式会显示错误并退出（退出码 2）。

## 命令行参数

```bash
wprescue batch [OPTIONS]
```

| 参数 | 短选项 | 长选项 | 默认值 | 说明 |
|------|--------|--------|--------|------|
| work_root | - | `--work-root` | `.` | 工作根目录 |
| mode | `-m` | `--mode` | `p` | 恢复模式 |
| max_line | `-n` | `--max-line` | - | 最大处理行数 |
| parse_workers | `-w` | `--parse-workers` | - | 恢复线程数 |
| stat_sec | - | `--stat` | - | 统计输出间隔（秒） |
| stat_print | `-p` | `--print_stat` | false | 周期打印统计信息 |

## 使用示例

```bash
# 基本恢复操作
wprescue batch --work-root /project

# 多线程加速恢复
wprescue batch --work-root /project \
    -w 8 \
    --parse-workers 8

# 限制恢复行数并输出统计
wprescue batch --work-root /project \
    -n 50000 \
    --stat 5 \
    -p

# 尝试守护模式（会失败）
wprescue daemon --work-root /project
# 输出: wprescue 仅支持 batch 模式
# 退出码: 2
```

## 工作原理

1. 读取救援目录（`./data/rescue`）中的数据
2. 按照项目配置的 Sink 路由进行处理
3. 输出到目标位置
4. 处理完成后自动退出

## 工作目录结构

wprescue 与 wparse 共用工作目录结构：

```
project/
├── conf/
│   └── wparse.toml          # 主配置（rescue_root 指向救援数据根）
├── models/
│   └── sinks/
│       ├── business.d/      # 业务路由
│       ├── infra.d/         # 基础设施路由
│       └── defaults.toml    # 默认配置
├── connectors/
│   └── sink.d/              # 连接器定义
└── data/
    └── rescue/              # 救援数据目录（默认输入）
```

## 退出与日志

- batch 模式下，读取完救援数据后优雅退出
- 关键日志与 wparse 一致：
  - 每个源结束：`数据源 '...' picker 正常结束`
  - 全局收尾：`all routine group await end!`
- 日志目录默认为 `./logs/`
- 日志格式：`{时间} [LEVEL] [target] {message}`
- 超过 10MB 自动滚动（保留 10 份，gz 压缩）

## 错误处理

wprescue 包含特殊的错误诊断机制：

- 启动失败时会打印详细诊断信息
- 退出码反映错误类型：
  - 0：成功
  - 2：参数错误（如使用 daemon 模式）
  - 其他：运行时错误

## 与 wparse 的区别

| 特性 | wparse | wprescue |
|------|--------|----------|
| 运行模式 | daemon/batch | 仅 batch |
| 输入源 | 多种数据源 | 救援目录 |
| 用途 | 正常解析 | 数据恢复 |
| Sink 工厂 | 完整 | file/null/test_rescue |
