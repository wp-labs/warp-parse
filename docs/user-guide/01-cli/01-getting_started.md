# Wparse 配置指南
<!-- 角色：使用配置者 | 最近验证：2025-12-11 -->

本文基于 `usecase/core/getting_started` 用例，梳理一次从初始化到运行、统计与校验的完整配置流程，适合首次接入与本地自测。

## 前置准备
- 构建二进制（或使用现有 `target/<profile>` 产物）
  ```bash
  cargo build --workspace --all-features
  ```
- PATH 可找到以下 CLI（也可直接用 `target/debug/<bin>` 替代）：
  - `wparse`（解析引擎）
  - `wproj`（统一工具入口）
  - `wpgen`（数据生成器）
- 建议在目录：`usecase/core/getting_started` 下执行命令

## 一、初始化工作目录
- 清理并初始化配置与模板
  ```bash
  wproj conf clean || true
  wproj conf init
  wproj conf check
  ```
- 同步初始化生成器（可选，但 getting_started 用例需要）
  ```bash
  wpgen conf clean || true
  wpgen conf init
  ```

执行完成后，工作目录将包含：
- `conf/wparse.toml` — 引擎主配置
- `conf/wpgen.toml` — 生成器配置
- `connectors/source.d/` — 源连接器模板（默认含文件源）
- `models/` 下的规则/OML/Sink 模板
- `data/` 运行目录：`in_dat/`、`out_dat/`、`rescue/`、`logs/`

> getting_started 用例中已将日志目录统一为 `./logs/`，与脚本输出一致。

## 二、主配置（conf/wparse.toml）要点
- 目录根：
  - `[models]` 聚合模型路径：`sources`/`wpl`/`oml`/`sinks`（默认指向 `models/` 子目录；sources/sinks 为复数）
  - `[rescue].path` 默认为 `./data/rescue`
- 运行参数：
  - `--parse-workers` 并行 worker 数、`[performance].rate_limit_rps` 速率、`robust` 稳健等级
- 日志：
  - `[log_conf]` 运行日志；推荐：
    ```toml
    [log_conf]
    output = "File"
    [log_conf.file]
    path = "./logs/"
    ```
- 统计项：使用 `[stat]` + `[[stat.pick]]/[[stat.parse]]/[[stat.sink]]` 配置三阶段统计；全量目标用 `target = "*"`；`window_sec` 仅在 `[stat]` 顶层设置。

## 三、源配置与连接器
- `conf/wpsrc.toml`（由 `wproj conf init` 生成）：
  - 采用统一写法，支持 `[[sources]] + connect/params_override`
  - 通过 `connect` 关联 `connectors/source.d/*.toml` 中的连接器
- `connectors/source.d/*.toml`：定义连接器 `id`、`type`（`file/kafka/syslog` 等）与 `params`

检查/构建数据源：
```bash
# 仅解析与注册表校验（不构建、不做 I/O）
wproj conf check

# （预留）解析 + 构建：当前 shim 未实现 `wproj data check`，请仅执行 `wproj conf check`
```

> 提示：`wproj` 会在启动时注册内置的源/汇构建器（包括文件源），确保 `file` 等类型可用。本仓库的 `wparse/wproj/wprescue` 还会注册开发期连接器适配器（MySQL/Kafka），以便在生成/CLI 端使用 `conn_url` 简写（例如 `mysql://...`、`kafka://...`）自动展开为 `params`。生产环境建议接入正式适配器并在应用启动时注册。

## 四、Sink 配置
- 目录：`models/sinks/`（`infra.d/` 与 `business.d/`）
- 路由与分组在上述目录中维护；`wproj stat/validate` 将基于该目录统计/校验。

## 五、生成数据与清理
```bash
# 清理输出（文件型 sink、本地数据）
wproj data clean || true
wpgen  data clean || true

# 生成样本（示例 3000 行，3 秒统计间隔）
wpgen sample -n 3000 --stat 3
```

## 六、运行解析
```bash
# 批处理（-n 指定条数，-p 打印统计；失败时查看 ./logs/ 下日志）
wparse batch --stat 3 -p -n 3000 --parse-workers 2
```

## 七、统计与校验
```bash
# 同时统计源与文件型 sink
wproj stat file

# 仅统计源/仅统计 sink
wproj stat src-file
wproj stat sink-file

# 校验 sink（按 expect 比例/区间）
wproj validate sink-file -v
# 可选：--input-cnt 指定总输入条数；--json 输出 JSON
```

## 常见问题与排查
- No builder registered for source kind 'file'
  - 需在 CLI 启动前注册源构建器；`wproj` 会默认注册。自定义流程需调用 `plugins::register_sources_factory_only()`。
- connectors/source.d not found
  - 确保使用 `wproj conf init` 初始化并在正确工作目录执行命令（或传 `-w/--work-root`）。
- 日志位置不一致
  - getting_started 用例中 `conf/wparse.toml` 已设置 `log_conf.file.path = "./logs/"`。

## 一键验证（推荐）
在 `usecase/core/getting_started` 下可直接执行：
```bash
./case_verify.sh
```
脚本会完成预构建 → conf/data 初始化 → 样本生成 → 解析运行 → 统计与校验的整套流程。

## 附录：最小可用示例（wpsrc.toml 与 connectors）

以下示例为“文件源”最小可用配置。目录均以工作目录为相对路径。

1) conf/wpsrc.toml（最小可用）

```toml
# 源配置：定义启用的源，并通过 connect 关联 connectors 中的某个连接器。

[[sources]]
key = "file_1"            # 源标识（唯一，用于统计与路由）
enable = true             # 是否启用该源
connect = "file_src"# 连接器 id（见 connectors/source.d/*.toml）
tags = ["env:local"]      # 可选标签（用于标识/排查）

# 可选：覆盖连接器中的部分参数；若不需要可留空表。
params_override = { }

# 示例：若需要在此覆盖 path/encode，可写：
# params_override = { base = "./data/in_dat", file = "gen.dat", encode = "text" }
```

2) connectors/source.d/00-file-default.toml（带注释的默认文件源连接器）

```toml
# 定义一个文件源连接器记录。多个连接器可在同一文件内用多个 [[connectors]] 块，或分文件放置。

[[connectors]]
id = "file_src"     # 连接器 id，wpsrc.toml 的 connect 字段需引用此 id
type = "file"              # 源类型：file/syslog/kafka 等
allow_override = [         # 允许被 sources.params_override 覆盖的参数名
  "path",
  "encode",
]

# 连接器默认参数；文件源至少应提供 path（输入文件路径）与 encode（编码方式）。
[connectors.params]
base = "data/in_dat"            # 输入基础目录；通常指向 wpgen 生成的样本
file = "gen.dat"               # 文件名
encode = "text"                 # 文本编码；可选：text/base64/hex
```
