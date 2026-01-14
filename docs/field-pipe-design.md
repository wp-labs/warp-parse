# Field Pipe 设计方案

本文档描述 WPL 管道中“字段集合操作”与“单字段操作”分离后的整体方案，旨在支撑如下写法：

- `json | take(key) | base64_decode`
- `json (@key | base64_decode)` （沿用现有 `@key` 语义）
- `(chars, chars) | last() | base64_decode`

核心思想是通过新的运行时抽象限制不同类型的 pipe 能力，使字段索引、语义与性能优化更稳定。

## 目标

1. **单字段接口**：`base64_decode`、`json_unescape` 等函数仅操作某个字段引用，不再访问整个 `Vec<DataField>`。
2. **字段集操作明确化**：`take(key)`、`last()` 以及 Group pipe 作为字段集操作器，负责选择/转换字段集合并决定哪个字段进入后续单字段函数。
3. **索引一致性**：`FieldIndex` 始终记录“首个同名字段”，即使多次构建或重复字段存在，语义一致。
4. **无需扩展语法**：复用当前 DSL，尤其是 `@key`，通过 AST 映射到新的选择器实现。

## 关键抽象

### 执行上下文

`PipeExecutor` 内部持有 `&mut Vec<DataField>`、可选的“活动字段”索引以及 `FieldIndex` 缓存。外部不直接访问这些状态，而是通过三个 trait 协同：

- `FieldSelector`：只负责在集合中定位字段；
- `FieldSetPipe`：对集合进行增删改；
- `FieldPipe`：基于当前活动字段执行逻辑。

执行器在 selector 成功后记录活动字段下标，并在集合被改写时清空该下标及索引。

### `FieldSelector`

```rust
pub trait FieldSelector {
    fn select<'a>(&self, fields: &'a mut Vec<DataField>) -> Option<&'a mut DataField>;
}
```

- `take(name)`、`last()`、`@key` 等 selector 在集合上查找字段并返回 `Option<&mut DataField>`；找不到时返回 `None`。
- Selector 不改变集合结构，只负责确定“下一个 FieldPipe 要处理的字段”。

### `FieldSetPipe`

```rust
pub trait FieldSetPipe {
    fn process(&self, fields: &mut Vec<DataField>) -> WResult<()>;
}
```

- Group、排序、批量新增字段、字符串解析等依旧实现此 trait。
- 执行器在调用后负责检测集合是否被修改并决定是否重建 `FieldIndex`，可通过 pipe 返回自定义标志或比较前后 `len`/版本号等方式实现。

### `FieldPipe`

```rust
pub trait FieldPipe {
    fn process(&self, field: Option<&mut DataField>) -> WResult<()>;
}
```

- `field == None` 表示 selector 未命中或缺失，典型实现应返回 `"<pipe> | no active field"`。
- 如果需要“可选”语义，也可以在 `None` 时直接 `Ok(())`，由 DSL 控制链式效果。

## Pipe 执行流程

1. `PipeExecutor` 初始化内部状态（字段集、活动字段、索引）。
2. 若命中 `FieldSetPipe`：
   - 调用 `process` 直接操作字段集。
   - 如果结构被改写（由 pipe 通知或执行器检测），重建索引并清空活动字段。
3. 若命中 selector：
   - 调用 `select`，记录返回的字段引用（或其索引）。
4. 若命中 `FieldPipe`：
   - 将步骤 3 记录的引用作为 `Option<&mut DataField>` 传入 `process`。
   - 未设置活动字段则抛出 `"<pipe> | no active field"`。
5. Group pipe (`WplEvalGroup`) 作为 `FieldPipe` 运行：默认会运行 last()

## AST 与语义映射

- `WplFun` 拆分为两类：
  - `FieldSelector`（`take`, `last`, 现有 `@key` 节点）：编译为 selector，实现 `FieldSelector` trait。
  - `FieldTransform`（各种 Fun、`base64_decode` 等）：实现 `FieldPipe`。
- `@key` 解析阶段直接转换为 `FieldSelector::Take(key)`，无需 DSL 语法改动。
- 现有 `WplPipe::Group` 继续保留，只是执行路径改到 `FieldSetPipe`。

## 错误处理

- `FieldPipe` 在 `None` 时：`fail.context(ctx_desc("<pipe> | no active field"))`。
- `take(key)` 未找到字段：沿用 `"<pipe> | not exists"`。
- `last()` 在空集合：同样返回 `not exists` 错误。
- 允许拓展“可选”语义：特定 selector 可在未命中时返回 `None` 而非报错，由下游决定是否继续。

## 索引策略

- `FieldIndex::build` 使用 `HashMap::entry(...).or_insert(idx)`，确保同名字段永远指向首个出现位置。
- 只有当管道中存在 `FieldPipe` 时才构建索引。
- 每当字段集被 `FieldSetPipe` 改写时重建索引并清空活动字段。

## 示例执行

```
json | take(key) | base64_decode

- take(key) -> selector，定位字段。
- base64_decode -> FieldPipe，对 active 字段执行。
```


```
(chars, chars) | last() | base64_decode

- 初始字段集：两个 chars。
- last() -> selector
- base64_decode -> 仅处理最后一个字段。
```

```
(chars, chars) |  base64_decode

- 初始字段集：两个 chars。
- 自动执行 last()
- base64_decode -> 仅处理最后一个字段。
```

## 实施步骤

1. 引入 `FieldSelector`、`FieldSetPipe`、`FieldPipe` 三个 trait，并让 `PipeExecutor` 管理活动字段状态与索引。
2. 迁移现有 Fun：
   - 选择器类（新建 `take/last`、`@key`）实现 `FieldSelector`。
   - 原有 Fun 实现 `FieldPipe`（多数代码可复用）。
3. Group pipe 等集合操作实现 `FieldSetPipe`，在结构变化后通知执行器重建索引。
4. 补充测试：重复字段、`last()`、的嵌套 group、`base64_decode` 组合等。
5. 更新用户文档，说明“字段函数需要在前面显式选择字段”。

该方案确保了“基于单字段的管道处理器”具备清晰接口，同时维持现有 DSL 习惯与兼容性。
