# 远程工程拉取与规则热更新 SOP

## 适用范围

本文用于整理以下运维任务：

- 在远程机器上初始化一个来自远端版本仓库的 WP 工程
- 后续通过 `wproj conf update` 更新工程内容
- 在不中断 `wparse daemon` 进程的前提下触发规则或模型重载

## 前提条件

远程机器应满足：

- 已安装可用的 `wproj`、`wparse`
- 已约定固定工作目录，例如 `/srv/wp/<project>`
- 目标远端仓库已经包含完整的 WP 工程配置内容

## 启用运行时管理面

规则热更新依赖运行时管理面。按 [admin.md](admin.md) 配置 `conf/wparse.toml`。

## 首次部署

初始化到显式版本：

```bash
wproj init \
  --work-root /srv/wp/<project> \
  --repo https://github.com/wp-labs/editor-monitor-conf.git \
  --version 1.4.2
```

初始化到默认目标版本：

```bash
wproj init \
  --work-root /srv/wp/<project> \
  --repo https://github.com/wp-labs/editor-monitor-conf.git
```

校验工程完整性：

```bash
wproj check
wproj data stat
```

启动 daemon：

```bash
wparse daemon --work-root .
```

检查运行时状态：

```bash
wproj engine status --work-root .
```

## 双仓库模式（models / infra 分离）

### 架构总览

双仓库模式将工程拆分为两个独立更新的组：

```
工程目录结构                    来源仓库
─────────────                  ────────
models/                        models 仓库 (如 wp-rule)
├── wpl/                         管理解析规则
├── oml/                         管理模型定义
└── knowledge/                   管理知识库

conf/        ┐
topology/    ├── infra 组 ──→  infra 仓库 (如 editor-monitor-conf)
connectors/  ┘                   管理配置、拓扑、连接器
```

| 组 | 管理的目录 | 作用 |
|----|-----------|------|
| `models` | `models/` | 解析规则 (wpl)、模型定义 (oml)、知识库 |
| `infra` | `conf/`, `topology/`, `connectors/` | 主配置、数据源/汇拓扑、连接器配置 |

两个仓库独立版本管理，互不影响。models 升级不影响 infra 配置，反之亦然。

### 配置写法

```toml
[project_remote]
enabled = true
# 双仓库模式下 repo 必须留空，不要与 models/infra 混用
repo = ""

[project_remote.models]
repo = "https://github.com/wp-labs/wp-rule.git"
init_version = "0.1.0"       # 首次初始化时使用的版本

[project_remote.infra]
repo = "https://github.com/wp-labs/editor-monitor-conf.git"
init_version = "0.1.6"       # 首次初始化时使用的版本
```

**字段说明：**

| 字段 | 必填 | 说明 |
|------|------|------|
| `[project_remote].enabled` | 是 | 双仓库开关，两个组共用 |
| `[project_remote].repo` | 否 | 双仓库模式下必须留空 `""` |
| `[project_remote.models].repo` | 是 | models 远端 Git 仓库地址 |
| `[project_remote.models].init_version` | 否 | 首次 sync 使用的版本；后续默认取最新 tag |
| `[project_remote.infra].repo` | 是 | infra 远端 Git 仓库地址 |
| `[project_remote.infra].init_version` | 否 | 首次 sync 使用的版本；后续默认取最新 tag |

### 版本选择规则

每次 `wproj conf update --group <group>` 的版本解析逻辑：

1. 如果显式指定 `--version` → 使用指定版本
2. 如果该组**从未初始化过**（state 中没有该组的记录）→ 使用配置中的 `init_version`（若有），否则取远端最新 tag
3. 如果该组**已有记录** → 取远端最新 tag

这确保首次部署和后续更新都有合理的默认行为。

### 同步流程

`wproj conf update --group <group>` 执行步骤：

1. 将远端仓库 clone/update 到本地缓存（`.run/project_remote/remote-<group>/`）
2. fetch 远端 tags，按版本选择规则解析目标版本
3. checkout 到目标 commit
4. 对比缓存与工作目录，若管理目录有差异：
   - 备份当前工作目录的管理目录
   - 将缓存中的管理目录复制到工作目录
5. 持久化 state 到 `.run/project_remote_state.json`

**缓存目录：**

| 组 | 缓存路径 |
|----|---------|
| models | `.run/project_remote/remote-models/` |
| infra | `.run/project_remote/remote-infra/` |
| 单仓库 | `.run/project_remote/remote/` |

### State 文件格式

双仓库模式下 `.run/project_remote_state.json`：

```json
{
  "models": {
    "version": "0.1.0",
    "tag": "0.1.0",
    "revision": "fcfc9e5..."
  },
  "infra": {
    "version": "0.1.6",
    "tag": "v0.1.6",
    "revision": "e2e84e1..."
  }
}
```

### 初始化顺序

双仓库模式下，**必须先初始化 infra，再初始化 models**。因为 infra 同步会写入 `conf/wparse.toml`（工程主配置），其中包含了双仓库的 repo 地址。models 同步时需要从这个配置读取 models 仓库地址。

> **推荐做法：** infra 仓库自身的 `conf/wparse.toml` 应包含完整的双仓库配置（`[project_remote.models]` + `[project_remote.infra]`）。这样 infra sync 后无需手动修补配置即可直接 sync models。

### 操作命令

更新 models 组：

```bash
wproj conf update --work-root /srv/wp/<project> --group models --version 1.4.3
```

更新 infra 组：

```bash
wproj conf update --work-root /srv/wp/<project> --group infra --version 1.1.0
```

双仓库模式下 `--group` 必填，单仓库模式下不应带 `--group`。

### 回滚行为

按组回滚只影响该组管理的目录：

- `--group models` 回滚 → 只恢复 `models/`，`conf/`/`topology/`/`connectors/` 不受影响
- `--group infra` 回滚 → 只恢复 `conf/`/`topology/`/`connectors/`，`models/` 不受影响

回滚前会自动备份当前状态，如果回滚过程中出错，会恢复到备份。

## 日常规则更新 SOP

先更新工程内容：

```bash
wproj conf update --work-root /srv/wp/<project>
```

显式切换到某个版本：

```bash
wproj conf update --work-root /srv/wp/<project> --version 1.4.3
```

双仓库模式下按组更新：

```bash
wproj conf update --work-root /srv/wp/<project> --group models --version 1.4.3
```

重载前最小校验：

```bash
wproj check --what wpl --fail-fast
```

触发仅重载本地已更新内容：

```bash
wproj engine reload \
  --work-root . \
  --request-id rule-$(date +%Y%m%d%H%M%S) \
  --reason "rule reload"
```

更新并重载：

```bash
wproj engine reload \
  --work-root . \
  --update \
  --request-id update-$(date +%Y%m%d%H%M%S) \
  --reason "rule update and reload"
```

双仓库模式下按组更新并重载：

```bash
wproj engine reload \
  --work-root . \
  --update \
  --group models \
  --request-id update-models-$(date +%Y%m%d%H%M%S) \
  --reason "models update and reload"
```

## 回滚 SOP

单仓库模式回滚：

```bash
wproj conf update --work-root /srv/wp/<project> --version 1.4.2
wproj engine reload \
  --work-root /srv/wp/<project> \
  --request-id rollback-$(date +%Y%m%d%H%M%S) \
  --reason "rollback rule set"
```

双仓库模式下按组回滚：

```bash
wproj conf update --work-root /srv/wp/<project> --group models --version 1.4.2
wproj engine reload \
  --work-root /srv/wp/<project> \
  --request-id rollback-$(date +%Y%m%d%H%M%S) \
  --reason "rollback models"
```

## 远端覆盖方式

```bash
wproj engine status \
  --work-root /srv/wp/<project> \
  --admin-url http://127.0.0.1:19090 \
  --token-file "${HOME}/.warp_parse/admin_api.token"
```

## 相关文档

- 运行时管理面使用说明: [admin.md](admin.md)
- 对应英文版: [../../en/operations/project-sync.md](../../en/operations/project-sync.md)
