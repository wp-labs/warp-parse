# 实例号（WP_INS_ID）

## 适用范围

本文用于多实例部署时的实例号分配与排查：

- 同一套 WPL/OML 规则被多个 `wparse` 实例同时运行
- 多个实例写同一张下游表，需要 `wp_event_id` 全局唯一
- 实例扩容、迁移、重建容器时的编号管理

单实例部署也建议读一遍"部署检查清单"一节。

## 为什么需要实例号

`wp_event_id` 是 64 位整数，布局固定：

```text
 63            32 31        22 21                    0
+----------------+-----------+-----------------------+
|  秒 (32 bit)   | 实例(10)  |  每秒计数器 (22 bit)  |
+----------------+-----------+-----------------------+
```

- **秒**：Unix 时间戳（1970 起算），可用到 2106-02-07；在高位，按数值比较即时间序
- **实例号**：`WP_INS_ID - 100`，占 10 位
- **每秒计数器**：每个实例每秒最多 4,194,304 条，超出时逻辑秒前移一格

**唯一性完全由实例号保证**：只要两个实例的 `WP_INS_ID` 不同，它们写入的 ID 段互不相交，永远不可能重复。反过来，实例号撞了就是**持续性重复**（不是偶发），这也是它必须由部署侧严格保证的原因。

## 取值范围

| 项 | 值 |
| --- | --- |
| 有效范围 | `100 ..= 1123` |
| 实例数上限 | 1024 |
| 默认值 | `100`（未设置时） |

**编号写进历史数据后不可更改。** 修改某个已运行实例的 `WP_INS_ID` 不会造成 ID 重复（新 lane 是全新的号段），但该实例的历史数据会分布在两个 lane 上，按实例做聚合统计时会分裂。

建议预先分段，便于人工识别与扩容：

```text
100 - 199   生产环境
200 - 299   预发 / 灰度
300 - 399   测试
400 - 499   开发 / 本地
```

## 配置方式

解析顺序与 `WP_GLOBAL_TAGS` 完全一致，**先命中者生效**：

1. 进程环境变量（`docker -e` / systemd `Environment=` / K8s `env`）
2. `<work_root>/.env`
3. `<work_root>/.warp_parse/.env`

进程环境优先，`.env` 不会覆盖已存在的变量，因此编排层永远拥有最终决定权。

### 单机 / systemd

```ini
[Service]
Environment=WP_INS_ID=101
WorkingDirectory=/srv/wp/proj-a
ExecStart=/usr/local/bin/wparse daemon
```

### Docker Compose

每个服务实例写死一个不同编号：

```yaml
services:
  wparse-a:
    image: wparse:0.36.2-alpha
    environment:
      WP_INS_ID: "101"
    volumes: [ ./proj-a:/work ]
  wparse-b:
    image: wparse:0.36.2-alpha
    environment:
      WP_INS_ID: "102"
    volumes: [ ./proj-b:/work ]
```

### Kubernetes

StatefulSet 的 Pod 名是 `<statefulset>-<ordinal>`，序号由控制器保证唯一，正好可以映射成实例号。用 Init 容器把结果写进 `.env`：

```yaml
initContainers:
  - name: assign-instance-id
    image: busybox:1.36
    command:
      - sh
      - -c
      - |
        ordinal=${HOSTNAME##*-}
        echo "WP_INS_ID=$((100 + ordinal))" > /work/.env
    volumeMounts:
      - { name: work, mountPath: /work }
```

> 注意：`HOSTNAME` 在 Pod 内就是 `metadata.name`，不需要额外注入。

Deployment 没有序号，只能由你显式分配（每副本一份 ConfigMap，或用外部注册表）。

**不要用 `metadata.uid`、Pod 名做哈希。** 把高熵名字压进 10 位会产生**持续**碰撞：1000 个实例在 1024 个槽位里按哈希分配，期望冲突对数是 `1000² / (2 × 1024) ≈ 488` —— 远高于随机 ID 方案，而且撞上就是永久重复。

如果实例数接近 1024，请提前规划扩容：这个字段只有 10 位，无法“再加一个 lane”。

### 实例 / 工程目录

```bash
# /srv/wp/proj-a/.env
WP_INS_ID=101
```

```bash
# /srv/wp/proj-a/.warp_parse/.env（等价备选位置）
WP_INS_ID=101
```

## 启动校验

| 情况 | 行为 |
| --- | --- |
| 未设置 | 使用默认值 `100`，输出 WARN |
| 合法 | 输出 `instance id: 101` |
| 非数字 / 越界（<100 或 >1123） | **启动失败**，错误信息包含取值范围 |
| 首次发号后被改为其它值 | **启动失败** |

未设置时的日志：

```text
WARN WP_INS_ID not set; using default instance id 100 (multiple instance will collide)
```

非法值时：

```text
ERROR  wparse error
配置错误
detail: invalid WP_INS_ID: WP_INS_ID 必须在 [100, 1123] 之间，当前为 99
```

**不要依赖默认值做多实例部署** —— 所有漏配的实例都会拿到 `100`，产出的 ID 100% 重复。

## 部署检查清单

1. 每个实例都有唯一且不同的 `WP_INS_ID`
2. 启动日志里能看到 `instance id: NNN`，且没有 `WP_INS_ID not set` 的 WARN
3. 抽查一条记录，确认 ID 的实例段符合预期：

   ```bash
   # id 为 wp_event_id 的十进制值
   # 实例号 = (id / 2^22) % 2^10 + 100
   echo "($id / 4194304) % 1024 + 100" | bc
   ```

4. 换了实例号之后，确认没有把同一实例的旧 `.env` / 旧编排配置复用到新实例上

## 仍可能重复的情况

以下情形无法在进程内解决，需要在部署与下游侧处理：

| 情形 | 说明 |
| --- | --- |
| 两个实例配置了相同 `WP_INS_ID` | 持续性重复，只能靠编号纪律 + 监控发现 |
| 镜像/卷快照被复制到第二台机器 | 两份相同的 `.env` 会得到相同实例号 |
| `fork()` 后不 `exec` 的子进程 | 共享父进程的计数器状态（`exec` 会重建，不受影响） |
| 下游列宽不足 | 32 位整数、`double`（超过 2^53 丢精度）、过短的 `VARCHAR` 会让 ID 看起来重复 |

已由引擎规避、无需运维介入的情况：

- **同一秒内崩溃重启**：首次发号的逻辑秒会对齐到"首次调用所在整秒 + 1"，不会重放上一轮的号
- **挂钟回拨**：沿用已用过的逻辑秒继续累加计数器，不会倒退
- **单实例超量**（> 4,194,304 条/秒）：逻辑秒前移一格，不回绕

## 下游列类型

`wp_event_id` 全程 `u64` 并以**字符串**输出，wparse 侧不涉及有符号数。剩下的只是接收端怎么建列：

| 列类型 | 可用范围 |
| --- | --- |
| `VARCHAR` / `String` | 2106-02-07 |
| `BIGINT UNSIGNED` | 2106-02-07 |
| 有符号 `BIGINT` / `Int64` | 2038-01-19（ID ≈ `秒 × 2^32`，秒到 `2^31` 时越过 `i64::MAX`） |
| 32 位整数 | 不可用 |

若列是 `VARCHAR` 且需要按 ID 排序：字典序只在位数相同时成立，ID 今天 19 位、约 2044 年变成 20 位，跨过该边界后排序会错乱。请用 `CAST(... AS BIGINT UNSIGNED)`，或给列定长补零（如 `CHAR(20)`）。

## 验证用例

`wp-examples` 的 `core/instance_id` 覆盖了本文的全部关键行为：

```bash
cd wp-examples/core/instance_id
./run.sh
```

包括默认值、进程环境、`.env`、进程优先于 `.env`、`WP_GLOBAL_TAGS` 共用同一套解析，以及四种非法值必须启动失败。
