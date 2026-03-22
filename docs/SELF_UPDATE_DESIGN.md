# Warp Parse 自动更新能力设计（wproj self update）

- 文档状态：Draft
- 所属仓库：`warp-parse`
- 适用版本：`0.18.x`
- 最后更新：2026-03-07

## 1. 背景与目标

当前仓库包含 4 个二进制：`wparse`、`wpgen`、`wproj`、`wprescue`（见 `Cargo.toml`）。

本设计目标是在不影响运行面稳定性的前提下，为 Warp Parse 增加可控、可回滚、可审计的自动更新能力。

### 1.1 目标

1. 提供统一更新入口，覆盖整套二进制。
2. 支持手动检查、手动升级、自动检查、可选自动应用。
3. 支持完整校验（版本一致性 + 哈希）与失败回滚。
4. 不把网络下载/二进制替换放进 `wparse` 主链路。

### 1.2 非目标

1. 不在首版支持增量补丁（仅全量包）。
2. 不在首版做 GUI 交互。
3. 不在首版引入多清单兼容层（仅支持 `updates/<channel>/manifest.json`）。

## 2. 放置位置与职责边界

## 2.1 CLI 入口放在 `wproj`

新增命令组：`wproj self ...`

原因：
- `wproj` 现定位为项目与运维管理工具，天然承载升级动作。
- `wparse`/`wprescue` 是运行入口，避免把更新风险引入高频运行路径。

## 2.2 代码组织

建议目录：

- `src/self_update/mod.rs`：总编排（check/update/rollback）
- `src/self_update/model.rs`：Manifest、策略、状态模型
- `src/self_update/client.rs`：manifest 拉取与下载
- `src/self_update/verify.rs`：版本/文件名一致性 + sha256 校验
- `src/self_update/install.rs`：安装、原子替换、备份、回滚
- `src/self_update/state.rs`：状态持久化与文件锁
- `src/wproj/handlers/self_update.rs`：CLI 参数适配层

`src/lib.rs` 暴露 `pub mod self_update;`，供后续其他 bin 读取“更新可用提示”。

## 2.3 各二进制职责

- `wproj`：唯一执行更新动作的入口。
- `wparse`：可选只做“有新版本提示”（读取本地缓存），不执行替换。
- `wpgen`/`wprescue`：不承载更新流程。

## 3. CLI 设计

新增：

```bash
wproj self status
wproj self check [--channel stable|beta|alpha] [--json]
wproj self update [--channel <c>] [--to <version>] [--yes] [--dry-run] [--force]
wproj self rollback [--to <backup_id>]
wproj self auto enable|disable|set --interval <hours> --mode check|apply
```

### 3.1 命令语义

- `status`：输出当前版本、安装路径、渠道、自动策略、最近检查结果。
- `check`：仅检查新版本，不做任何落盘替换。
- `update`：执行下载、校验、安装、健康检查；失败自动回滚。
- `rollback`：回滚到最近或指定备份版本。
- `auto`：设置自动策略（建议默认 `check`，不默认 `apply`）。

## 4. 配置与状态文件

统一放置在：`~/.warp_parse/update/`

- `policy.toml`：自动更新策略
- `state.json`：最近检查/更新结果
- `lock`：文件锁，防并发更新
- `backups/`：版本备份目录（建议保留最近 2~3 份）

`policy.toml` 示例：

```toml
enabled = true
mode = "check"          # check | apply
channel = "stable"      # stable | beta | alpha
interval_hours = 24
```

## 5. 远端 Manifest 设计（独立元数据仓库，仅 updates）

### 5.1 Channel 与分支映射（必须遵循现有发布机制）

当前发布机制固定为：

- `stable` channel <- `main` 分支
- `beta` channel <- `beta` 分支
- `alpha` channel <- `alpha` 分支

更新系统必须按上述映射拉取版本，不允许跨分支混用产物。

建议约束：
1. 远端 manifest 从独立元数据仓库读取：`/Users/zuowenjian/devspace/wp-labs/wp-install`（远端仓库：`wp-labs/wp-install`）。
2. 主清单入口：
   - `stable` -> `updates/stable/manifest.json`
   - `beta` -> `updates/beta/manifest.json`
   - `alpha` -> `updates/alpha/manifest.json`
3. 客户端默认只检查“当前 channel”。
4. 自动更新仅允许同 channel 升级。
5. 跨 channel（如 alpha -> beta）必须显式指定 `--channel`，并要求二次确认。
6. 不再兼容 `dist/install-manifest*.json`。

### 5.2 默认 channel 判定

默认策略：

1. 优先读取 `policy.toml` 的 `channel`。
2. 若无策略文件，可按构建分支推断默认 channel：
   - branch=`main` -> `stable`
   - branch=`beta` -> `beta`
   - branch=`alpha` -> `alpha`
3. 若无法判定，回落到 `stable`。

### 5.3 manifest 示例（updates v2）

```json
{
  "version": "0.12.2-alpha",
  "channel": "alpha",
  "published_at": "2026-03-05T16:24:24Z",
  "git_ref": "v0.12.2-alpha",
  "git_commit": "5809f446e3a35cefa060a0494830ba4f0ce14102",
  "assets": {
    "aarch64-apple-darwin": {
      "url": "https://github.com/galaxy-sec/galaxy-flow/releases/download/v0.12.2-alpha/galaxy-flow-v0.12.2-alpha-aarch64-apple-darwin.tar.gz",
      "sha256": "0cf7ea3267cd207029b05827ce18db8532c4a0cef653247bb2fa82408ef703ce"
    },
    "x86_64-unknown-linux-gnu": {
      "url": "https://github.com/galaxy-sec/galaxy-flow/releases/download/v0.12.2-alpha/galaxy-flow-v0.12.2-alpha-x86_64-unknown-linux-gnu.tar.gz",
      "sha256": "d85438444991a125582b15fd26cae7f8505fc82121f103c900b14697590da511"
    },
    "x86_64-unknown-linux-musl": {
      "url": "https://github.com/galaxy-sec/galaxy-flow/releases/download/v0.12.2-alpha/galaxy-flow-v0.12.2-alpha-x86_64-unknown-linux-musl.tar.gz",
      "sha256": "b3d57a8f04b3d57c85b32921488bd796afe0b6a2d12d69b6fb92f63137af11dd"
    }
  }
}
```

要求：
1. 客户端只读取 `updates/<channel>/manifest.json`。
2. `version` 支持带/不带 `v` 前缀；比较前统一归一化为 semver。
3. manifest 中 `channel` 必须与路径 channel 一致。
4. 目标平台必须存在对应 `assets[target]` 条目。
5. `assets[target].sha256` 必须为 64 位十六进制字符串（非空）。
6. MVP 平台支持：`aarch64-apple-darwin`、`aarch64-unknown-linux-gnu`、`x86_64-unknown-linux-gnu`（不支持 Intel mac）。

## 6. 更新流程（时序）

1. 读取当前版本（`build::PKG_VERSION`）。
2. 按 channel 从元数据仓库拉取 `updates/<channel>/manifest.json`。
3. semver 比较，判定是否可升级。
4. 下载目标资产并校验 sha256。
5. 解压到临时目录，准备替换集（4 个 bin）。
6. 获取更新锁，创建备份。
7. 原子替换（优先 `rename` 语义）。
8. 健康检查（`--version`）。
9. 成功写入 state；失败自动回滚并记录失败原因。

## 7. 安全与可靠性约束

1. 强制 channel 隔离与版本/文件名一致性校验（防止错配）。
2. 下载域名白名单（官方发布域名）。
3. 超时/重试/断点续传（首版可先超时+重试）。
4. 文件锁防并发。
5. 原子替换 + 备份 + 回滚闭环。
6. 日志与状态可审计（成功/失败原因、版本、时间）。
7. 状态记录需包含来源清单文件名（便于审计）。

## 8. 与包管理器安装的兼容

检测到来自包管理器（brew/apt/yum）安装时：

- `check` 正常可用。
- `update` 默认不直接替换，提示使用包管理器升级命令。
- `--force` 可覆盖（需显式确认，默认关闭）。

## 9. 自动更新触发策略

建议：

- 默认开启 `auto-check`，默认关闭 `auto-apply`。
- 触发点放在 `wproj` 启动流程中（轻量、可跳过）。
- 与业务运行时解耦：`wparse` 不做在线更新。
- 自动检查/自动应用都只在当前 channel 内生效，不做跨 channel 跳转。

## 10. MVP 实施范围

### 10.1 首批实现

1. `wproj self status/check/update/rollback`。
2. manifest 拉取（`updates` 主入口）与版本归一化。
3. 版本/文件名一致性校验。
4. 全量包安装 + 原子替换 + 备份回滚。
5. `policy.toml` + `state.json` 持久化。

### 10.2 次批实现

1. `wproj self auto ...` 完整策略管理。
2. `wparse` 启动时版本可用提示（只读 state）。
3. 更细粒度错误码与可观测指标。

## 11. 验收标准

1. 更新成功后 4 个二进制版本一致。
2. 人为注入校验失败/替换失败时可自动回滚。
3. 并发触发更新不会破坏安装（锁生效）。
4. 包管理器安装路径下不会误替换系统文件（默认行为）。
5. `stable/main`、`beta/beta`、`alpha/alpha` 映射在检查与更新流程中被严格执行。
6. `updates/<channel>/manifest.json` 的 channel/版本/资产一致性校验通过。

## 12. 风险与缓解

- 风险：跨仓库发布存在短暂不同步窗口（资产已发布但 manifest 未更新）。
  - 缓解：发布流水线采用“先资产可达校验，再写元数据仓库 latest 指针”的顺序。
- 风险：manifest 结构约束不严格导致解析歧义。
  - 缓解：在元数据仓库 CI 增加 schema 校验与样例回归测试。
- 风险：不同平台文件权限差异导致替换失败。
  - 缓解：平台适配层 + 安装后健康检查 + 回滚。
- 风险：自动应用影响稳定性。
  - 缓解：默认仅检查，应用需显式开启。

## 13. 后续任务拆分（建议）

1. CLI 接口与参数：`wproj/args.rs`、`wproj/handlers/cli.rs`
2. 核心模块骨架：`src/self_update/*`
3. 发布产物与 manifest 规范对齐（CI/CD）
4. 集成测试：成功路径、并发锁、回滚路径

## 14. CI/CD 发布侧对齐规范（含三 channel，跨仓）

本节定义自动更新所需的发布侧规范，确保与当前三通道机制严格一致：

- `stable` channel <- `main` 分支
- `beta` channel <- `beta` 分支
- `alpha` channel <- `alpha` 分支

### 14.1 触发与 channel 判定规则

沿用现有 `release.yml` 的 tag 触发方式（`v*.*.*`），并固定判定：

1. tag 含 `-alpha` -> `channel=alpha`
2. tag 含 `-beta` -> `channel=beta`
3. 其他 semver tag -> `channel=stable`

示例：

- `v0.19.0-alpha.3` -> `alpha`
- `v0.19.0-beta.2` -> `beta`
- `v0.19.0` -> `stable`

### 14.2 分支归属强校验（新增）

CI 必须增加“tag 来源分支校验”步骤，防止错发：

1. `channel=alpha` 时，tag commit 必须可追溯到 `origin/alpha`。
2. `channel=beta` 时，tag commit 必须可追溯到 `origin/beta`。
3. `channel=stable` 时，tag commit 必须可追溯到 `origin/main`。
4. 校验失败则 release job 直接失败，不发布 manifest/资产。

### 14.3 更新资产与 manifest 发布布局（元数据仓库）

建议在独立元数据仓库（`wp-labs/wp-install`）维护如下结构：

```text
updates/
  stable/manifest.json
  stable/versions/v0.19.0.json      # 可选：版本归档
  beta/manifest.json
  beta/versions/v0.19.0-beta.2.json
  alpha/manifest.json
  alpha/versions/v0.19.0-alpha.3.json
```

`updates/<channel>/manifest.json` 最小字段：

1. `version`
2. `channel`
3. `assets[target].url`
4. `assets[target].sha256`

### 14.4 客户端拉取规则（与发布侧一一对应）

客户端按 channel 取清单：

- `stable` -> `updates/stable/manifest.json`
- `beta` -> `updates/beta/manifest.json`
- `alpha` -> `updates/alpha/manifest.json`

并执行以下一致性校验：

1. 请求路径 channel 与清单字段映射一致
2. manifest `version` 与资产文件名/URL 中版本一致
3. 目标平台必须存在对应 artifact

### 14.5 主仓 `release.yml` 最小改造建议

在主仓 `.github/workflows/release.yml` 上增加以下步骤（不改变现有构建矩阵）：

1. `determine-channel`：从 tag 推断 `channel`（已有逻辑可复用）
2. `verify-branch-ownership`：执行 tag commit 与 channel 分支归属校验（新增）
3. `build-and-publish-assets`：发布二进制资产并计算 `sha256`（已有/增强）
4. `generate-updates-manifest`：生成 `updates/<channel>/manifest.json`（新增）
5. `push-metadata-repo`：使用机器人凭据提交到元数据仓库（新增）
6. `update-latest-pointer`：最后更新 `updates/<channel>/manifest.json`（原子覆盖，新增）

### 14.6 元数据仓库工作流建议

在元数据仓库增加校验工作流（示例 `validate-manifest.yml`）：

1. 校验 JSON schema（`updates`）。
2. 校验资产 URL 可达性与 `sha256` 字段非空。
3. 校验 channel 路径与 manifest `channel` 字段一致。
4. 校验通过后才允许合并/发布。

### 14.7 验收补充（CI 侧）

1. 三个 channel 均能独立产出对应 manifest，并写入元数据仓库。
2. 任一 channel 的 tag 不会覆盖其他 channel 的 `updates` 清单。
3. 错分支打 tag 会被 CI 拦截并失败。
4. 主仓发布成功后，元数据仓库会自动更新 latest 清单。
5. 客户端在 `--channel` 不同取值下能命中正确 `updates` 清单地址。

---

该设计文档用于先行评审；评审通过后按 MVP 范围进入开发。
