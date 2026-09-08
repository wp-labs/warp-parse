# 需求文档：wpgen.toml 支持 [models].wpl 配置 WPL 规则目录

| 项 | 内容 |
|---|---|
| 文档编号 | REQ-wpgen-config-001 |
| 优先级 | Medium |
| 影响组件 | wpgen（`wp-motor` / `wpgen`） |
| 提出背景 | streaming 管线 models 目录共享后，wparse 可通过 `wparse.toml` 的 `[models].wpl` 指定路径，wpgen 却只能用命令行 `--wpl`，配置不一致，容易遗漏导致 `found 0 files` |

---

## 1. 问题

`wparse.toml` 支持 `[models]` 段指定 WPL/OML/knowledge 路径：

```toml
[models]
wpl = "../../models/wpl"
oml = "../../models/oml"
knowledge = "../../models/knowledge"
```

`wpgen.toml` 不支持任何 WPL 路径配置——命令行 `--wpl` 是唯一途径：

```bash
wpgen sample --wpl ../../models/wpl -n 3000
```

当目录结构调整（如 models 移到共享父目录）时，wparse 只需改 `wparse.toml` 一处，wpgen 却要在每个调用点（`run.sh`、脚本、CI）都加 `--wpl`。遗漏时 wpgen 回退到 `./models/wpl/`，`found 0 files`，不发送任何数据，且无明确报错——最终表现为管线无 alert，排查困难。

## 2. 需求

### R1：`wpgen.toml` 支持 `[models] wpl` 配置

`wpgen.toml` 新增 `[models]` 段，字段语义与 `wparse.toml` 保持一致。

```toml
# wpgen.toml
version = "1.0"

[models]
wpl = "../../models/wpl"          # WPL 规则与样本文件目录

[generator]
count = 1000
...

[output]
...
```

配置优先级：`--wpl` CLI 参数 > `wpgen.toml` `[models].wpl` > 默认 `./models/wpl/`。

### R2：配置校验

`[models].wpl` 指向的目录不存在或无 `*.dat` / `*.wpl` 文件时，启动阶段报错（而非 `found 0 files` 的静默行为）。

错误示例：
```
[config] wpgen: [models].wpl "../../models/wpl" directory not found or empty
  hint: check the wpl path, or place sample.dat / parse.wpl in the directory
  at: wparse/conf/wpgen.toml
```

---

## 3. 验收标准

| 编号 | 验收项 |
|---|---|
| AC1 | `wpgen.toml` 配置 `[models].wpl = "../../models/wpl"` 时，不加 `--wpl` 也能找到 sample.dat |
| AC2 | `--wpl` CLI 参数优先级高于 `[models].wpl` |
| AC3 | 未配置 `[models].wpl` 且无 `--wpl` 时，行为不变（默认 `./models/wpl/`） |
| AC4 | `[models].wpl` 指向无效目录时，启动报错（不静默 `found 0 files`） |

---

## 4. 影响

- **向后兼容**：新增 `[models]` 段，旧 `wpgen.toml` 无此段时行为不变。
- **本仓库 streaming 管线**：可直接在 `wpgen.toml` 中配置 `[models].wpl`，去掉 `run.sh` 中的 `--wpl`。

## 5. 关联证据

- 当前 `wpgen.toml` 不支持 `[models]`、`wpl`、`rule_root` 任何 WPL 路径字段（均报 `配置错误: load object from toml file`）
- `wparse.toml` 已支持 `[models].wpl`，两者配置不一致是本需求的直接动因
- streaming 管线因目录结构调整后 wpgen `found 0 files`，排查过程中确认只能靠 `--wpl` 修复
