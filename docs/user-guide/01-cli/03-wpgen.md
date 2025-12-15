# wpgen CLI
<!-- 角色：使用配置者 | 最近验证：2025-12-12 -->

wpgen 是数据生成工具，用于基于 WPL 规则或样本文件生成测试数据。

## 命令概览

```
wpgen <COMMAND>

Commands:
  rule    基于 WPL 规则生成数据
  sample  基于样本文件生成数据
  conf    配置管理（init/clean/check）
  data    数据管理（clean/check）
```

## 子命令详解

### rule - 基于规则生成

基于 WPL 规则生成测试数据，支持规则验证和性能分析。

```bash
wpgen rule [OPTIONS]
```

| 参数 | 短选项 | 长选项 | 默认值 | 说明 |
|------|--------|--------|--------|------|
| work_root | `-w` | `--work-root` | `.` | 工作根目录 |
| wpl_dir | - | `--wpl` | - | WPL 规则目录覆盖 |
| conf_name | `-c` | `--conf` | `wpgen.toml` | 配置文件名 |
| stat_print | `-p` | `--print_stat` | false | 周期打印统计信息 |
| line_cnt | `-n` | - | - | 总行数覆盖 |
| gen_speed | `-s` | - | - | 生成速度（行/秒）覆盖 |
| stat_sec | - | `--stat` | 1 | 统计输出间隔（秒） |

### sample - 基于样本生成

基于样本文件（sample.dat）生成测试数据。

```bash
wpgen sample [OPTIONS]
```

参数与 `rule` 子命令相同。

### conf - 配置管理

```bash
wpgen conf <SUBCOMMAND>

Subcommands:
  init   初始化生成器配置（conf/wpgen.toml）
  clean  清理生成器配置
  check  检查配置有效性
```

| 参数 | 短选项 | 长选项 | 默认值 | 说明 |
|------|--------|--------|--------|------|
| work_root | `-w` | `--work-root` | `.` | 工作根目录 |

### data - 数据管理

```bash
wpgen data <SUBCOMMAND>

Subcommands:
  clean  清理已生成的输出数据
  check  数据检查（暂不支持）
```

| 参数 | 短选项 | 长选项 | 默认值 | 说明 |
|------|--------|--------|--------|------|
| work_root | `-w` | `--work-root` | `.` | 工作根目录 |
| conf_name | `-c` | `--conf` | `wpgen.toml` | 配置文件名 |
| local | - | `--local` | false | 仅本地清理 |

## 运行语义

### count（总产出条数）

启动时按 `parallel` 精确均分到每个 worker，余数前置分配。各 worker 跑完本地任务量即退出，总量严格等于 `count`。

### speed（全局速率）

- `speed = 0`：无限制（不等待）
- `speed > 0`：每 worker 速率为 `floor(speed / parallel)`

### parallel（并行数）

生成 worker 的并行数。对 `blackhole_sink` 消费端也会并行，其它 sink 默认单消费者。

### 退出行为

生成组先完成 → 向 router/sinks/monitor 广播 Stop → 各组件自然退出。

## 使用示例

```bash
# 配置初始化
wpgen conf init -w .
wpgen conf check -w .

# 基于规则生成 10000 条数据
wpgen rule -w . -n 10000 -p

# 自定义规则目录和生成速度
wpgen rule --work-root /project \
    --wpl /custom/rules \
    -c custom.toml \
    -s 1000 \
    --stat 2 \
    -p

# 基于样本文件生成
wpgen sample -w . -n 50000 -s 5000 --stat 5 -p

# 清理生成的数据
wpgen data clean -w . -c wpgen.toml --local
```

## 诊断日志

关键日志标记：

| 组件 | 启动 | 结束 |
|------|------|------|
| Worker | `gen worker start …` | `gen worker end` |
| Router | `router start` | `router exit` |
| Sink | `sink recv cmd …` | `sink dat channel closed; exit` |
| Monitor | `monitor proc start …` | `monitor proc end` |

## 常见问题

**Q：生成不能退出？**

A：检查是否出现 `… channel closed; exit` 或 `router recv stop … / router exit` 日志。确保运行的是 release 版本。

**Q：产出不足预期？**

A：`count` 被精确分配给每个 worker。检查日志中 `limit : …` 是否符合预期。

## 配置文件

默认配置文件路径：`conf/wpgen.toml`

主要配置项：

```toml
[generator]
count = 10000      # 总生成条数
speed = 1000       # 生成速度（行/秒），0 表示无限制
parallel = 4       # 并行 worker 数

[output]
# 输出配置...
```

生成文件通常位于 `./data/in_dat/`，可在配置中调整目标路径。
