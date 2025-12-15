# 日志设置与问题定位（wparse/wpgen/wproj/wprescue）

本文面向使用者与一线开发，给出在 Warp Flow 中开启/调整日志与进行常见问题定位的实操指南。日志实现基于 `log + log4rs`，初始化入口为 `wp_log::conf::log_init`，统一输出格式为：

```
{YYYY-mm-dd HH:MM:SS.ssssss} [LEVEL] [target] message
```

- 目标域（targets）见《开发者文档：Logging Targets And Levels》。
- 文件日志默认滚动：10MB/10 份，gzip 压缩。

## 快速上手

### wparse / wprescue / wproj（conf/wparse.toml）

生产推荐（文件输出 + 低噪声）：
```toml
[log_conf]
output = "File"   # Console|File|Both
level  = "warn,ctrl=info,launch=info,source=info,sink=info,stat=info,runtime=warn,oml=warn,wpl=warn,klib=warn,orion_error=error,orion_sens=warn"
[log_conf.file]
path = "./data/logs"   # 目录会自动创建；文件名按可执行名自动命名，如 wparse.log
```

本地联调（同时输出到控制台）：
```toml
[log_conf]
output = "Both"
level  = "debug,ctrl=info,launch=info,source=debug,sink=debug,stat=debug,runtime=debug,connector=debug"
[log_conf.file]
path = "./data/logs"
```

> 提示：当前实现以 `level` 串为准，不解析结构化 `[log_conf.levels]`。如需精确控制，请直接在 `level` 串中设置 `ctrl=info,source=debug,...`。

### wpgen（conf/wpgen.toml）

最简推荐：
```toml
[logging]
level = "warn"
output = "file"           # stdout|file|both
file_path = "./data/logs" # 目录；文件名按可执行名自动命名（wpgen.log）
```

## 目标域与常用级别建议

- 启停/装配：`ctrl`(info)、`launch`(info)
- 数据入口：`source`(debug)
- 解析/规则：`parse`(debug)、`rule`(debug)、`wpl/oml`(info|debug)
- 数据下游：`sink`(debug)
- 外部连接：`connector`(debug|trace)
- 运行时框架：`runtime`(debug)
- 性能指标：`stat`(info|debug)
- 数据样本：`data`(trace，仅本地调试，避免生产泄露)

## 查看与验证

- 文件日志：`tail -F ./data/logs/wparse.log`（或 `wpgen.log` / `wprescue.log`）
- 控制台输出：将 `output` 设为 `Console` 或 `Both`；`wproj conf check` 可加 `--console`
- 统计到控制台（便于观察吞吐变化）：
  - wparse：`wparse batch --stat 3 --print_stat`
  - wpgen：`wpgen sample -n 1000 --stat 3 --print_stat`

说明：`--print_stat` 仅影响控制台统计输出，不改变日志级别。

## 常见问题与定位

1) “没有日志/没有落盘”
- 检查 `output` 是否为 `File/Both`，以及目录是否可写：`ls -ld ./data/logs`
- 首次运行会自动创建目录；权限不够会启动报错，修正 `path` 或目录权限后重试

2) “我设置了模块级别但不生效（wparse/wprescue/wproj）”
- 当前仅解析 `level` 串；请将 `ctrl=info,source=debug,...` 直接写进 `level`

3) “连接器/扩展的 info 日志看不到”
- 部分扩展使用标准 `log::info!`（无 target），受 root 级别控制；将 root 调到 info，例如：`level = "info,ctrl=info,launch=info,..."`

4) “源有输入但汇端无输出”
- 打开 `sink=debug,connector=debug`；观察连接建立、重试、报错
- 对网络类汇（如 syslog/kafka）：使用抓包/客户端确认链路（例如 `tcpdump -i any -n port 514 -A` 或 `kcat -b <brokers> -t <topic>`）

5) “解析不符合预期/规则没命中”
- 打开 `parse=debug,rule=debug,wpl=info,oml=info`；必要时短时打开 `data=trace` 仅在本地调试

## 实用级别模板

- 生产最小噪音：
```toml
[log_conf]
output = "File"
level  = "warn,ctrl=info,launch=info,source=info,sink=info,stat=info,runtime=warn"
[log_conf.file]
path = "./data/logs"
```

- 汇联调（含连接器细节）：
```toml
[log_conf]
output = "Both"
level  = "info,ctrl=info,launch=info,sink=debug,connector=debug,stat=info"
[log_conf.file]
path = "./data/logs"
```

## 采集统计与日志协同

- `--stat <sec>` 设置窗口；`--print_stat` 控制是否在控制台打印统计（不影响日志文件）
- 观察 `stat` 与 `source/sink` 联动：吞吐异常通常伴随上/下游错误或重试日志

## 注意事项

- 当前不支持通过 `RUST_LOG` 环境变量配置日志级别（整体使用 log4rs）。
- 同一进程内重复初始化日志会报错；CLI 工具每次重启不会有此问题，批量检查已做单次初始化保护。
- 避免在生产环境开启 `data=trace`，并避免打印敏感信息。

## 进一步阅读
- 《开发者文档：Logging Targets And Levels》：目标域定义、默认级别与使用建议。
