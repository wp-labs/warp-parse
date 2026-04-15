# `wproj` 项目工具使用指南

## 适用范围

本文说明 `wproj` 这组项目级命令的职责划分和常见操作路径。

`wproj` 是最应该优先掌握的工具，因为大部分工程初始化、检查、模型查看、运行时操作都通过它完成。

## 查看帮助

```bash
wproj --help
```

## 初始化工程

只做本地骨架初始化：

```bash
wproj init --work-root .
```

从远端仓库初始化工程：

```bash
wproj init \
  --work-root /srv/wp/demo \
  --repo https://github.com/example/project-conf.git
```

初始化到显式版本：

```bash
wproj init \
  --work-root /srv/wp/demo \
  --repo https://github.com/example/project-conf.git \
  --version 1.4.2
```

适用场景：

- 新建项目目录
- 首次拉取远端规则工程
- 为线上实例准备标准化工程骨架

## 批量检查工程

全量检查：

```bash
wproj check --work-root .
```

只检查 WPL：

```bash
wproj check --work-root . --what wpl --fail-fast
```

JSON 输出：

```bash
wproj check --work-root . --json
```

当前 `--what` 常见取值：

- `conf`
- `connectors`
- `sources`
- `sinks`
- `wpl`
- `oml`
- `all`

建议把 `wproj check` 作为上线前固定步骤，而不是出问题后再补执行。

## 检查与清理数据

检查数据源配置和连通性：

```bash
wproj data check --work-root .
```

清理工程数据输出：

```bash
wproj data clean --work-root .
```

## 统计输入输出

同时看文件源与文件型 sink：

```bash
wproj data stat --work-root .
```

只看 source 文件输入量：

```bash
wproj data stat src-file --work-root .
```

只看 sink 文件输出量：

```bash
wproj data stat sink-file --work-root .
```

按组或 sink 过滤：

```bash
wproj data stat \
  --work-root . \
  --group demo-group \
  --sink file_out
```

## 验证输出比例

```bash
wproj data validate --work-root .
```

如果源统计不可用，可以显式指定输入条数：

```bash
wproj data validate --work-root . --input-cnt 100000
```

这个命令适合做结果分布检查，判断 sink 侧比例是否明显异常。

## 查看模型拓扑

查看 source：

```bash
wproj model sources --work-root .
```

查看 sink：

```bash
wproj model sinks --work-root .
```

查看规则到 sink 的路由路径：

```bash
wproj model route --work-root .
```

按组过滤：

```bash
wproj model route \
  --work-root . \
  --group demo-group
```

这些命令适合在联调时回答几个直接问题：

- 当前启用了哪些 source
- 某个规则最终会流向哪些 sink
- OML 是否挂在了预期链路上

## 运行时状态与 reload

查询状态：

```bash
wproj engine status --work-root .
```

触发 reload：

```bash
wproj engine reload \
  --work-root . \
  --reason "manual reload"
```

更新远端工程后再 reload：

```bash
wproj engine reload \
  --work-root . \
  --update \
  --reason "update and reload"
```

这部分属于在线运维路径，详细说明见 [../operations/admin.md](../operations/admin.md)。

## 远端版本同步

执行远端工程更新：

```bash
wproj conf update --work-root .
```

切换到指定版本：

```bash
wproj conf update --work-root . --version 1.4.3
```

详细流程见 [../operations/project-sync.md](../operations/project-sync.md)。

## rescue 目录统计

统计 rescue 目录：

```bash
wproj rescue stat --work-root .
```

查看明细：

```bash
wproj rescue stat --work-root . --detail
```

输出 JSON：

```bash
wproj rescue stat --work-root . --json
```

适用场景：

- 排查异常数据是否进入 rescue
- 粗看不同 sink 的失败分布
- 给后续补偿处理准备基础统计

## 自更新

检查更新：

```bash
wproj self check
```

执行更新：

```bash
wproj self update --yes
```

这个能力主要服务于二进制分发和版本切换，不替代工程配置更新。

## 推荐使用顺序

### 本地开发联调

1. `wproj init` 初始化工程
2. 配合 `wpgen` 准备测试数据
3. `wproj check --what wpl`
4. `wproj model route` 与 `wproj data stat` 核对结果

### 线上运维

1. `wproj conf update`
2. `wproj check --what wpl --fail-fast`
3. `wproj engine status`
4. `wproj engine reload`
5. `wproj engine status`
6. 如有异常，再看 `wproj rescue stat`

## 当前边界

- `wproj rule` 目前实际暴露的是 `parse` 能力，尚不是完整的规则工作台
- 更细的配置字段语义仍应以专项文档和对应配置说明为准

## 相关文档

- 功能与 CLI 使用指南: [index.md](index.md)
- 运行时管理面使用说明: [../operations/admin.md](../operations/admin.md)
- 远程工程拉取与规则热更新 SOP: [../operations/project-sync.md](../operations/project-sync.md)
