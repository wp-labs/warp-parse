# defaults 与 expect
<!-- 角色：使用配置者 | 最近验证：2025-12-11 -->

目标
- 配置期望（比例/阈值/窗口）并理解分母口径与忽略规则；在 CLI 中进行校验。

核心概念（权威见 `../02-config/04-sinks_config.md`）
- defaults：组的默认期望，`$SINK_ROOT/defaults.toml` 中的 `[defaults.expect]`；可被组/单 sink 覆盖。
- group expect：`[sink_group].expect`；优先于 defaults。
- sink expect：`[[sink_group.sinks]].expect`；仅支持局部字段（ratio/tol 或 min/max，二者不可混用）。
- 分母口径：`total_input | group_input | model`（具体定义见权威文档）。

示例（defaults）
```toml
version = "2.0"
[defaults]
tags = ["env:dev"]
[defaults.expect]
basis = "group_input"
min_samples = 100
mode = "warn"
```

示例（组级与单 sink）
```toml
[sink_group]
name = "/sink/demo"
[sink_group.expect]
basis = "group_input"
mode  = "fail"

[[sink_group.sinks]]
name = "ff"
connect = "file_json_sink"
[sink_group.sinks.expect]
ratio = 0.98   # 与 min/max 互斥
tol   = 0.01
```

CLI 校验
```bash
wproj validate sink-file -w <WR> [--stats-file ./out/stats.json] [--input-cnt N] [--json]
```

常见错误
- `ratio/tol cannot be combined with min/max`：二选一使用。
- 分母为 0 或样本不足：按 `min_samples` 忽略组校验（打印提示，不失败）。

相关
- 权威：`../02-config/04-sinks_config.md`
- 设计：`../../50-dev/design/sinks_tags.md`
