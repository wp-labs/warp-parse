# 需求文档：连接器配置错误应显式报错（而非静默回退/空操作）

| 项 | 内容 |
|---|---|
| 文档编号 | REQ-connector-param-001 |
| 优先级 | High |
| 影响组件 | `wp-core-connectors`、`wp-motor`（sink factory 注册） |
| 受影响工程 | wparse / wpgen |

---

## 1. 问题概述

排查 streaming 和 kafka 两条示例管线时发现两类"静默失败"，均表现为数据正常流转但最终不产 alert，全程无任何 ERROR / WARN：

**A. 参数类型不匹配时静默回退默认值。** `wpsrc.toml` 配置 `port = "9801"`（字符串），connector 默认 `port = 9000`（整数）。wparse 实际监听 `127.0.0.1:9000`，wpgen 发往 `9801`，两端对不上但无任何报错。原因是 `from_params` 中 `params.get("port").and_then(|v| v.as_i64()).unwrap_or(9000)` —— `as_i64()` 对字符串返回 `None`，`unwrap_or(9000)` 静默接管，不区分"键缺失"与"键存在但类型错"。同类问题还存在于 `instances`、`tcp_recv_bytes` 等数值字段。

**B. kafka sink factory 未注册时静默创建空壳。** wparse 未带 `--features kafka` 编译时，`builtin_factories.rs` 中无 kafka factory。但 `validate sink conf` 通过、`create sink` 不报错、空壳 sink 正常排水，最终 0 条消息写入 Kafka，无任何提示。

---

## 2. 需求

### R1：参数值类型不匹配时必须报错

`from_params` 中通过 `as_i64()` / `as_bool()` 等消费参数值时，须区分"键缺失"与"键存在但类型错"：

- **键缺失** → 使用默认值（保持向后兼容）；
- **键存在但类型错** → 报错退出，含参数名、用户值、期望类型、配置文件路径。

```rust
// 期望
let port = match params.get("port") {
    None => 9000,
    Some(v) => v.as_i64()
        .ok_or_else(|| anyhow!("parameter \"port\" expects integer, got {}", v))?,
};
```

错误信息示例：
```
[config] source 'tcp_1' (connector 'tcp_src'): parameter "port" type mismatch
  provided: "9801" (string), expected: integer
  hint: write it without quotes, e.g. port = 9801
  at: wparse/topology/sources/wpsrc.toml
```

### R2：sink factory 未注册时必须报错

`build sink` 阶段通过 `kind` 查找 factory 未命中时，必须返回错误，不得降级为空壳。错误信息须指明 sink 名、缺失的 factory、修复提示。

```
[config] sink 'nginx_to_kafka/kafka_output': factory 'kafka' not registered
  hint: kafka is a compile-time feature. Rebuild with: cargo build --features kafka --bins
  at: wparse/topology/sinks/business.d/nginx_to_kafka.toml (connect = "kafka_sink")
```

补充：`register_builtin_factories()` 应在启动时以 INFO 级别打印已注册的 factory 列表；`wparse version` 应列出已编译 feature。

### R3（可选，兜底）：配置 lint

在配置加载阶段扫描 `[*.params]`，对每个键与对应 connector `default_params` 做类型比对，不一致即告警。可在 `wparse lint` / `wpgen check` 阶段运行，无需启动引擎。

---

## 3. 验收标准

| 编号 | 验收项 | 验证方法 |
|---|---|---|
| AC1 | `port = "9801"`（字符串）时启动报错退出 | streaming 示例回退为字符串端口，应见明确报错而非监听 9000 |
| AC2 | `port = 9801`（整数）行为不变 | streaming 当前修复态运行正常 |
| AC3 | 省略 `port` 键时仍回退默认 9000 | 删除 wpsrc.toml 的 port 行，正常监听 9000 |
| AC4 | 报错覆盖 `instances`、`tcp_recv_bytes` 等数值字段 | 各字段分别构造字符串用例，均报错 |
| AC5 | 未带 `--features kafka` 启动 kafka 管线时报错退出 | 含 "kafka factory not registered" 及修复提示 |

---

## 4. 影响评估

- **向后兼容**：R1 会使历史上字符串型数值参数的配置升级后启动失败（预期行为）。建议先以 WARN 发布一个版本，再升级为 ERROR。
- **波及面**：所有 wparse/wpgen 部署中，连接器参数写成字符串型数值/布尔的配置均受影响。
- **本仓库示例**：streaming 示例已改为整数端口，不受影响。

---

## 5. 实施建议

### 5.1 R1（参数类型校验）

**改动点**：
- `wp-core-connectors/src/sources/tcp/config.rs:23` — `TcpSourceSpec::from_params` 中 `port`/`instances`/`tcp_recv_bytes` 的取值，改为区分 `None` vs `Some + 类型错`
- 同类 sink 的 `from_params` 中所有 `as_i64().unwrap_or(default)` / `as_bool().unwrap_or(default)` 模式

**分阶段**：v1 → WARN + 回退；v2 → ERROR。

### 5.2 R2（sink factory 缺失）

**改动点**：
- `wp-motor/src/sinks/backends/` — `build_sink` 中 `kind` 查 factory 注册表未命中时返回 `Err` 而非空壳
- `wp-motor/src/sinks/builtin_factories.rs` — 启动日志打印已注册 factory 列表

**分阶段**：v1 → WARN + 空壳；v2 → ERROR。

### 5.3 辅助

- `wparse version` 输出已编译 feature 列表
- `wparse lint` 增加参数类型检查规则

---

## 6. 关联证据

### 问题 A：端口类型静默回退

- `wparse.log`: `TCP listen 'tcp_1' addr=127.0.0.1:9000`（应为 9801）
- `wp-core-connectors-0.5.6/src/sources/tcp/config.rs:23`: `params.get("port").and_then(|v| v.as_i64()).unwrap_or(9000)`
- 修复验证：整数端口配置下 `scan.ndjson` / `traffic.ndjson` 正常产出 alert

### 问题 B：kafka factory 缺失

- wparse 启动日志仅注册 `BlackHole, File, Syslog, Tcp, TestRescue` — 无 kafka
- kafka sink: `drain complete` 瞬间完成，topic offset 始终为 0
- `builtin_factories.rs:8-14` — kafka 为条件编译注入，不在内置注册表中
- 二进制 `strings` 证据: `cargo build --features kafka --bins`
- 旧二进制（commit 66a762ee）带 kafka feature 时正常产出 83 条消息；新二进制（commit 993e16fa，built 2026-07-05）未带 feature 后立即 0 消息，无任何报错
