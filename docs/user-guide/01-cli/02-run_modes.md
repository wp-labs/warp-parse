# wparse 运行模式
<!-- 角色：使用配置者 | 最近验证：2025-12-12 -->

本文档说明 wparse 的运行模式（daemon/batch）、命令行参数和退出策略。

## TL;DR

- 两种运行模式：`wparse daemon`（常驻服务）和 `wparse batch`（批处理）
- 批处理模式读完文件后自动退出，daemon 模式需信号触发退出
- batch 模式下会忽略网络类源（TCP/Syslog TCP），如需接收 TCP 数据请使用 daemon 模式

## 命令行参数

```
wparse <COMMAND>

Commands:
  daemon  守护进程模式（常驻服务）
  batch   批处理模式（读完即退）
```

### 通用参数

| 参数 | 短选项 | 长选项 | 默认值 | 说明 |
|------|--------|--------|--------|------|
| work_root | - | `--work-root` | `.` | 工作根目录 |
| mode | `-m` | `--mode` | `p` | 解析模式 |
| max_line | `-n` | `--max-line` | - | 最大处理行数 |
| parse_workers | `-w` | `--parse-workers` | - | 解析线程数 |
| check_stop | `-S` | `--check-stop` | - | 停止检查阈值 |
| check_continue | `-s` | `--check-continue` | - | 继续检查阈值 |
| stat_sec | - | `--stat` | - | 统计输出间隔（秒） |
| robust | - | `--robust` | - | 鲁棒性模式 |
| stat_print | `-p` | `--print_stat` | false | 周期打印统计信息 |
| log_profile | - | `--log-profile` | - | 日志配置文件 |
| wpl_dir | - | `--wpl` | - | WPL 规则目录覆盖 |

## 使用示例

```bash
# 批处理模式：处理 3000 条后退出，每 2 秒输出统计
wparse batch -n 3000 --stat 2 -p

# 批处理模式：指定工作目录和多线程
wparse batch --work-root /project -w 4 --parse-workers 4

# 守护进程模式：常驻服务，每 5 秒输出统计
wparse daemon --stat 5 -p

# 自定义日志和规则目录
wparse daemon --log-profile custom.toml --wpl /custom/rules
```

## 退出策略

### 批处理模式（batch）

单源（picker）结束条件（任一满足）：
- 上游 EOF（文件读取完毕）
- 达到条数上限 `-n`
- 收到 Stop 指令
- 致命错误（触发全局停机）

进程退出流程：
1. 所有数据源的 picker 结束
2. 主组完成
3. sink/infra 组依序下线
4. 进程退出

关键日志：
- 每个源结束：`数据源 '...' picker 正常结束`
- 全局收尾：`all routine group await end!`

### 守护进程模式（daemon）

- 启动 acceptor（网络监听等）
- 进程保持常驻运行
- 退出触发方式：
  - SIGTERM/SIGINT/SIGQUIT 信号
  - 控制总线 Stop 指令（企业版）

## 错误与重试策略

| 错误类型 | 策略 | 说明 |
|----------|------|------|
| EOF | Terminate | 优雅结束当前源 |
| 断线/可重试 | FixRetry | 指数退避后继续 |
| 数据/业务可容忍 | Tolerant | 记录后继续 |
| 致命错误 | Throw | 触发全局停机 |

## 常见问题

**Q：为什么 batch 下不启动 acceptor？**

A：acceptor 是常驻组件（监听网络），会阻塞主组完成。batch 目标是"源结束 → 主组完成 → 进程退出"。

**Q：如何限制最大处理条数？**

A：使用 `-n <line_max>`。达到上限后该源结束，所有源结束即退出。

**Q：如何验证 batch 是否生效？**

A：观察日志中的"picker 正常结束"与"all routine group await end!"。

**Q：Prometheus sink 在 batch 模式下如何处理？**

A：Prometheus 需要常驻 HTTP 服务，batch 模式下建议改用文件输出。
