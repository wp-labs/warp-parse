# 排障指南（Troubleshooting）
<!-- 角色：使用配置者 | 最近验证：2025-12-11 -->

常见问题
- 未找到 connectors 目录：确认 `connectors/source.d` 或 `connectors/sink.d` 是否存在；遵守“从 models/<side> 向上查找最近目录（≤32 层）”。
- 工厂未注册：确保调用 `plugins::register_sinks()` 与 `register_sources_factory_only()`；`wproj` 默认会做。
- 覆写报错：检查 `allow_override` 白名单；覆写表禁止嵌套 `params/params_override`。
- tags 校验未过：数量/字符集/长度是否符合；减少高基数。
- feature 缺失：Kafka/DB 等需按 Cargo feature 启用。

定位建议
- 使用 `wproj sinks validate|list|route` 和 `wproj sources list|route` 先“看清配置解析结果”。
- 打开 `conf/wparse.toml`，确认 `sink_root/src_root` 指向的 models 目录存在。

延伸阅读
- 文档导航：docs/README.md（Quick Triage）
