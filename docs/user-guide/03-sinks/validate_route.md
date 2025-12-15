# Sinks CLI：Validate
<!-- 角色：使用配置者 | 最近验证：2025-12-11 -->

校验配置
```
wproj sinks validate --work-root <WR>
```
（校验结构、白名单覆写、组内重名、expect 位置等）

列出连接器
```
wproj sinks list --work-root <WR>
```

展示路由解析（目标路径、fmt、detail）
```
wproj sinks route --work-root <WR>
```

文件统计与校验
```
wproj stat sink-file -w <WR> [--group ...] [--sink ...] [--json]
wproj validate sink-file -w <WR> [--stats-file ...] [--input-cnt N] [--json]
```

提示
- 脚本使用可加 `-q/--quiet` 关闭 Banner。
- 需要脚本友好输出加 `--json`。
- 配合 `parallel` 与文件 sink `replica_shard`/`file_template`（多文件分片），可用 `wproj sinks route` 查看最终目标路径。

相关
- 最小骨架：`../02-config/03-sinks_minimal.md`
- 设计与配置总览：`../02-config/04-sinks_config.md`
