# wproj CLI
<!-- 角色：使用配置者 | 最近验证：2025-12-12 -->

wproj 是项目管理工具，用于项目初始化、配置检查、数据管理和模型管理。

## 命令概览

```
wproj <COMMAND>

Commands:
  rule   规则工具（离线解析测试）
  init   一键初始化项目骨架
  check  批量检查项目配置和文件完整性
  data   数据管理工具
  model  模型管理工具
```

## 全局参数

| 参数 | 短选项 | 长选项 | 默认值 | 说明 |
|------|--------|--------|--------|------|
| work_root | `-w` | `--work-root` | `.` | 工作根目录 |
| quiet | `-q` | `--quiet` | false | 隐藏启动 Banner |

## 退出码

| 代码 | 含义 |
|------|------|
| 0 | 成功 |
| 2 | 参数错误 |
| 3 | 执行错误 |
| 4 | 校验失败 |

---

## rule - 规则工具

离线解析测试，验证 WPL 规则。

```bash
wproj rule parse [OPTIONS]
```

| 参数 | 短选项 | 长选项 | 默认值 | 说明 |
|------|--------|--------|--------|------|
| in_path | `-i` | `--in-path` | - | 输入文件路径 |
| rule_file | `-R` | `--rule-file` | - | 规则文件路径 |
| json | - | `--json` | false | JSON 格式输出 |
| quiet | `-q` | `--quiet` | false | 静默模式 |

**示例：**

```bash
# 使用规则执行离线解析测试
wproj rule parse --in-path /data/sample.txt \
    --rule-file /rules/parser.wpl \
    --json \
    -q
```

---

## init - 项目初始化

一键创建项目目录结构和默认配置。

```bash
wproj init [OPTIONS]
```

| 参数 | 短选项 | 长选项 | 默认值 | 说明 |
|------|--------|--------|--------|------|
| work_root | `-w` | `--work-root` | `.` | 工作目录 |
| mode | `-m` | `--mode` | `conf` | 初始化模式 |

**初始化模式：**

| 模式 | 说明 |
|------|------|
| `full` | 完整项目（配置+模型+数据+示例） |
| `model` | 仅模型文件 |
| `conf` | 仅配置文件（默认） |
| `data` | 仅数据目录 |

**示例：**

```bash
# 初始化配置（默认）
wproj init -w /project

# 初始化完整项目
wproj init -w /project --mode full
```

---

## check - 项目检查

批量检查项目配置和文件完整性。

```bash
wproj check [OPTIONS]
```

| 参数 | 短选项 | 长选项 | 默认值 | 说明 |
|------|--------|--------|--------|------|
| work_root | `-w` | `--work-root` | `.` | 根目录 |
| what | - | `--what` | `all` | 检查项 |
| console | - | `--console` | false | 控制台日志输出 |
| fail_fast | - | `--fail-fast` | false | 首次失败即退出 |
| json | - | `--json` | false | JSON 格式输出 |
| only_fail | - | `--only-fail` | false | 仅输出失败项 |

**检查项（--what）：**

| 值 | 说明 |
|----|------|
| `conf` | 主配置文件 |
| `connectors` | 连接器配置 |
| `sources` | 数据源配置 |
| `sinks` | 数据汇配置 |
| `wpl` | WPL 规则语法 |
| `oml` | OML 模型语法 |
| `all` | 全部检查（默认） |

**示例：**

```bash
# 全面检查
wproj check -w /project --what all

# 仅检查配置和规则，首次失败即退出
wproj check -w /project --what conf,wpl --fail-fast

# JSON 输出，仅显示失败项
wproj check -w /project --json --only-fail
```

---

## data - 数据管理

```bash
wproj data <SUBCOMMAND>

Subcommands:
  clean     清理本地输出文件
  check     检查数据源连通性
  stat      统计数据量和性能
  validate  验证数据分布和比例
```

### data clean

清理项目输出数据。

```bash
wproj data clean [OPTIONS]
```

| 参数 | 短选项 | 长选项 | 默认值 | 说明 |
|------|--------|--------|--------|------|
| work_root | `-w` | `--work-root` | `.` | 工作目录 |
| local | - | `--local` | false | 仅本地清理 |

### data stat

统计数据量。

```bash
wproj data stat [OPTIONS] <SUBCOMMAND>

Subcommands:
  file       统计源+汇文件
  src-file   仅统计源文件
  sink-file  仅统计汇文件
```

| 参数 | 短选项 | 长选项 | 默认值 | 说明 |
|------|--------|--------|--------|------|
| work_root | `-w` | `--work-root` | `.` | 工作目录 |
| json | - | `--json` | false | JSON 格式输出 |

### data validate

验证数据分布和比例。

```bash
wproj data validate [OPTIONS]
```

| 参数 | 短选项 | 长选项 | 默认值 | 说明 |
|------|--------|--------|--------|------|
| work_root | `-w` | `--work-root` | `.` | 工作目录 |
| input_cnt | - | `--input-cnt` | - | 输入总数（分母） |
| stats_file | - | `--stats-file` | - | 统计文件路径 |
| verbose | `-v` | `--verbose` | false | 详细输出 |

**示例：**

```bash
# 清理输出数据
wproj data clean -w /project --local

# 统计源+汇文件行数
wproj data stat -w /project file

# 仅统计源文件
wproj data stat -w /project src-file --json

# 验证数据分布
wproj data validate -w /project \
    --input-cnt 10000 \
    --stats-file stats.json \
    -v
```

---

## model - 模型管理

```bash
wproj model <SUBCOMMAND>

Subcommands:
  sources  列出并检查源连接器
  sinks    列出汇组和路由
  route    显示数据流路径
  knowdb   知识库管理
```

### model sources

列出并验证源连接器配置。

```bash
wproj model sources [OPTIONS]
```

### model sinks

列出汇组和路由配置。

```bash
wproj model sinks [OPTIONS]
```

### model route

显示数据流路径（规则 → OML → 汇）。

```bash
wproj model route [OPTIONS]
```

| 参数 | 短选项 | 长选项 | 默认值 | 说明 |
|------|--------|--------|--------|------|
| work_root | `-w` | `--work-root` | `.` | 工作目录 |
| group | - | `--group` | - | 指定汇组 |
| sink | - | `--sink` | - | 指定汇 |
| path_like | - | `--path-like` | - | 路径匹配 |

### model knowdb

知识库管理。

```bash
wproj model knowdb <SUBCOMMAND>

Subcommands:
  init   生成 KnowDB 骨架
  check  校验 KnowDB 结构
  clean  清理 KnowDB 缓存
```

**knowdb init 参数：**

| 参数 | 短选项 | 长选项 | 默认值 | 说明 |
|------|--------|--------|--------|------|
| work_root | `-w` | `--work-root` | `.` | 工作目录 |
| full | - | `--full` | false | 生成完整模板 |

**示例：**

```bash
# 列出所有源连接器
wproj model sources -w /project

# 列出所有汇及其配置
wproj model sinks -w /project

# 显示指定汇的数据流路径
wproj model route -w /project \
    --group "group1" \
    --sink "sink1" \
    --path-like "/output"

# 初始化知识库（完整模板）
wproj model knowdb init -w /project --full

# 校验知识库结构
wproj model knowdb check -w /project

# 清理知识库缓存
wproj model knowdb clean -w /project
```

---

## JSON 输出格式

### stat src-file

```json
{
  "ok": true,
  "summary": { "total_enabled_lines": 12345 },
  "items": [
    { "key": "file_1", "path": "/abs/a.log", "enabled": true, "lines": 12000, "error": null }
  ]
}
```

### stat sink-file

```json
{
  "ok": true,
  "summary": { "total_lines": 9876 },
  "items": [
    { "group": "http", "sink": "ok_sink", "path": "./out/http/ok.dat", "framework": false, "lines": 9600 }
  ]
}
```

### validate

```json
{
  "ok": false,
  "items": [
    { "severity": "ERROR", "group": "http", "sink": "residue_sink", "msg": "actual ratio 0.04 > max 0.02" }
  ]
}
```

---

## 完整示例

```bash
# 项目初始化
wproj init -w /project --mode full

# 项目检查
wproj check -w /project --what all

# 数据统计与校验
wproj data stat -w /project sink-file --json
wproj data validate -w /project --input-cnt 10000

# 模型管理
wproj model sources -w /project
wproj model sinks -w /project
wproj model knowdb init -w /project --full
```
