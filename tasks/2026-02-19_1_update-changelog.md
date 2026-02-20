# 背景
文件名：2026-02-19_1_update-changelog.md
创建于：2026-02-19 10:57:38
创建者：zuowenjian
主分支：alpha
任务分支：alpha
Yolo模式：Ask

# 任务描述
更新 CHANGELOG 同步 wp-motor 变更

方案B：继续 alpha 序列，同步最新变更
- 将 Unreleased 内容移到 [0.18.0-alpha]
- 创建新的 Unreleased 描述 v1.17.1-alpha 到 v1.17.4-alpha 的变更
- 更新 wp-motor 到最新版本（v1.17.4-alpha）

# 项目概览
warp-parse 是一个高性能流数据处理系统，依赖 wp-motor 作为核心引擎。

当前版本状态：
- warp-parse Cargo.toml 版本：0.18.0
- warp-parse 最新标签：v0.18.0-alpha (2026-02-12)
- warp-parse 当前 wp-motor 依赖：v1.17.1-alpha
- wp-motor 最新版本：v1.17.4-alpha

⚠️ 警告：永远不要修改此部分 ⚠️
RIPER-5 协议规则摘要：
- 模式转换需要明确信号：ENTER RESEARCH/INNOVATE/PLAN/EXECUTE/REVIEW MODE
- EXECUTE 模式必须 100% 忠实遵循计划
- REVIEW 模式必须标记即使最小的偏差
- 未经明确许可不能在模式之间转换
- 每个响应开头必须声明当前模式
⚠️ 警告：永远不要修改此部分 ⚠️

# 分析

## 当前 CHANGELOG 状态

### CHANGELOG.md (中文版)
- [Unreleased] 部分描述 wp-motor v1.17.0-alpha 的变更
- 最新正式版本记录：[0.17.1] - 2026-02-09
- 缺少 v0.18.0-alpha 的正式记录

### CHANGELOG.en.md (英文版)
- 内容与中文版对应
- 同样缺少 v0.18.0-alpha 的正式记录

## wp-motor 版本对照

| 版本 | 日期 | 主要变更 |
|-----|------|---------|
| v1.17.0-alpha | 2026-02-12 | OML Match OR 条件语法、OML NLP 函数、可配置 NLP 词典 |
| v1.17.1-alpha | 2026-02-12 | `[semantic]` 配置控制 NLP 加载 |
| v1.17.2-alpha | 2026-02-13 | `kv`/`kvarr` key 解析支持括号类字符 |
| v1.17.3-alpha | 2026-02-17 | sink-level batch buffer，移除 AsyncFileSink BufWriter |
| v1.17.4-alpha | 2026-02-18 | `batch_size` 配置到 sink groups |

## 不一致之处
1. warp-parse Cargo.toml 中 wp-motor 依赖是 v1.17.1-alpha，但 CHANGELOG 的 Unreleased 只描述了 v1.17.0-alpha
2. wp-motor 已有 4 个新版本（v1.17.1 到 v1.17.4），需要同步到 warp-parse

# 提议的解决方案

## 需要修改的文件
1. `CHANGELOG.md` - 中文版 CHANGELOG
2. `CHANGELOG.en.md` - 英文版 CHANGELOG
3. `Cargo.toml` - 更新 wp-motor 依赖版本

## 变更内容

### 1. CHANGELOG.md 变更

将现有 [Unreleased] 内容移到 [0.18.0-alpha]，添加新的 [Unreleased] 部分：

```markdown
## [Unreleased]

### Added
- **Sinks/Buffer**: 新增 sink 级别批量缓冲，支持配置 `batch_size` 参数
  - 小数据包（< batch_size）进入待处理缓冲区，定期或缓冲区满时刷新
  - 大数据包（>= batch_size）自动绕过待处理缓冲区，减少开销（零拷贝直通路径）
  - 新增 `flush()` 公共 API 用于手动刷新缓冲区
- **Sinks/Config**: 新增 `batch_timeout_ms` 配置到 sink group（默认 300ms），控制定期缓冲刷新间隔

### Changed
- **Sinks/File**: 移除 `AsyncFileSink` 的 `BufWriter` 和 `proc_cnt` 定期刷新，直接写入 `tokio::fs::File`；上层批量组装使用户态缓冲变得多余

### Fixed
- **wp-oml**: 修复 parser 和 test 模块中的 llvm-cov 警告

## [0.18.0-alpha] - 2026-02-12

### Added
- **OML Match**: 新增 OR 条件语法 `cond1 | cond2 | ...`，用于匹配表达式
  - 支持单源和多源匹配
  - 兼容值匹配和函数匹配
- **OML NLP**: 新增 `extract_main_word` 和 `extract_subject_object` 管道函数，用于中文文本分析
- **OML NLP**: 新增可配置 NLP 词典系统，支持通过 `NLP_DICT_CONFIG` 环境变量自定义词典
- **Engine Config**: 新增 `[semantic]` 配置节，控制 NLP 语义词典加载（默认 `enabled = false`，禁用时节省约 20MB 内存）

### Changed
- **OML Match**: 多源匹配现在支持任意数量的源字段（不再限制为 2/3/4 个）
- **Documentation**: 更新 OML 文档（中英文），涵盖 match OR 语法和多源支持
```

### 2. CHANGELOG.en.md 变更

同步英文版内容：

```markdown
## [Unreleased]

### Added
- **Sinks/Buffer**: Add sink-level batch buffer with configurable `batch_size` parameter
  - Small packages (< batch_size) enter pending buffer, flushed periodically or when buffer is full
  - Large packages (>= batch_size) automatically bypass pending buffer for reduced overhead (zero-copy direct path)
  - New `flush()` public API for manual buffer flush
- **Sinks/Config**: Add `batch_timeout_ms` configuration to sink group (default 300ms), controls periodic buffer flush interval

### Changed
- **Sinks/File**: Remove `BufWriter` and `proc_cnt` periodic flush from `AsyncFileSink`, write directly to `tokio::fs::File`; upstream batch assembly makes userspace buffering redundant

### Fixed
- **wp-oml**: Fix llvm-cov warnings in parser and test modules

## [0.18.0-alpha] - 2026-02-12

### Added
- **OML Match**: Add OR condition syntax `cond1 | cond2 | ...` for match expressions
  - Supports single-source and multi-source match
  - Compatible with both value matching and function matching
- **OML NLP**: Add `extract_main_word` and `extract_subject_object` pipe functions for Chinese text analysis
- **OML NLP**: Add configurable NLP dictionary system, support custom dictionary via `NLP_DICT_CONFIG` environment variable
- **Engine Config**: Add `[semantic]` section in `wparse.toml` to control NLP semantic dictionary loading (`enabled = false` by default, saves ~20MB memory when disabled)

### Changed
- **OML Match**: Multi-source match now supports any number of source fields (no longer limited to 2/3/4)
- **Documentation**: Update OML documentation (Chinese and English) for match OR syntax and multi-source support
```

### 3. Cargo.toml 变更

更新 wp-motor 依赖版本从 v1.17.1-alpha 到 v1.17.4-alpha：

```toml
# 从
wp-engine     = {                            git = "https://github.com/wp-labs/wp-motor", tag = "v1.17.1-alpha" }
wp-config     = { package = "wp-config",     git = "https://github.com/wp-labs/wp-motor", tag = "v1.17.1-alpha" }
wp-lang       = { package = "wp-lang",       git = "https://github.com/wp-labs/wp-motor", tag = "v1.17.1-alpha" }
wp_knowledge  = { package = "wp-knowledge",  git = "https://github.com/wp-labs/wp-motor", tag = "v1.17.1-alpha" }
wp-cli-core   = { package = "wp-cli-core",   git = "https://github.com/wp-labs/wp-motor", tag = "v1.17.1-alpha" }
wp-proj       = { package = "wp-proj",       git = "https://github.com/wp-labs/wp-motor", tag = "v1.17.1-alpha" }

# 改为
wp-engine     = {                            git = "https://github.com/wp-labs/wp-motor", tag = "v1.17.4-alpha" }
wp-config     = { package = "wp-config",     git = "https://github.com/wp-labs/wp-motor", tag = "v1.17.4-alpha" }
wp-lang       = { package = "wp-lang",       git = "https://github.com/wp-labs/wp-motor", tag = "v1.17.4-alpha" }
wp_knowledge  = { package = "wp-knowledge",  git = "https://github.com/wp-labs/wp-motor", tag = "v1.17.4-alpha" }
wp-cli-core   = { package = "wp-cli-core",   git = "https://github.com/wp-labs/wp-motor", tag = "v1.17.4-alpha" }
wp-proj       = { package = "wp-proj",       git = "https://github.com/wp-labs/wp-motor", tag = "v1.17.4-alpha" }
```

# 当前执行步骤：等待进入 PLAN 模式

# 任务进度
- 2026-02-19 10:57:38 - 创建任务文件，完成研究分析

# 最终审查
（待完成后填写）