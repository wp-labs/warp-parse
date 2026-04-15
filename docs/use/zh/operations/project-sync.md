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

## 日常规则更新 SOP

先更新工程内容：

```bash
wproj conf update --work-root /srv/wp/<project>
```

显式切换到某个版本：

```bash
wproj conf update --work-root /srv/wp/<project> --version 1.4.3
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

## 回滚 SOP

```bash
wproj conf update --work-root /srv/wp/<project> --version 1.4.2
wproj engine reload \
  --work-root /srv/wp/<project> \
  --request-id rollback-$(date +%Y%m%d%H%M%S) \
  --reason "rollback rule set"
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
