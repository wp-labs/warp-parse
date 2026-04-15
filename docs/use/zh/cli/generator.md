# `wpgen` 生成器使用指南

## 适用范围

本文面向需要准备测试数据、样本回放数据、联调用输入的使用者，说明 `wpgen` 的主要能力和常见用法。

## 命令定位

`wpgen` 主要用于两类事情：

- 初始化和检查生成器配置
- 基于规则或样本生成测试数据

## 查看帮助

```bash
wpgen --help
```

## 初始化生成器配置

初始化 `conf/wpgen.toml`：

```bash
wpgen conf init --work-root .
```

检查生成器配置：

```bash
wpgen conf check --work-root .
```

建议顺序：

1. 先执行 `wpgen conf init`
2. 按工程需要修改 `conf/wpgen.toml`
3. 再执行 `wpgen conf check`

## 基于规则生成数据

```bash
wpgen rule \
  --work-root . \
  -c wpgen.toml \
  -n 10000 \
  -s 5000 \
  -p
```

适用场景：

- 验证某套 WPL 规则在目标字段分布下的表现
- 为 sink、压测、联调准备稳定输入
- 在没有真实日志时快速构造测试流量

## 基于样本生成数据

```bash
wpgen sample \
  --work-root . \
  -c wpgen.toml \
  -n 5000 \
  -p
```

适用场景：

- 手头已有原始样本文件
- 想在样本基础上做扩增和重复回放

## 常用参数

- `--work-root`：工作根目录
- `--wpl`：临时覆盖 WPL 目录
- `-c, --conf-name`：生成器配置文件名
- `-n`：覆盖总行数
- `-s`：覆盖生成速度
- `-p, --print_stat`：打印统计
- `--stat`：设置统计周期

## 清理已生成数据

根据 `wpgen` 配置清理已生成输出：

```bash
wpgen data clean --work-root . -c wpgen.toml
```

这一步适合在重复联调前清空旧输出，避免新旧样本混杂。

## 典型流程

1. `wpgen conf init --work-root .`
2. 修改 `conf/wpgen.toml`
3. `wpgen conf check --work-root .`
4. 执行 `wpgen rule` 或 `wpgen sample`
5. 用 `wparse batch` 或 `wproj` 相关命令验证结果

## 相关文档

- 功能与 CLI 使用指南: [index.md](index.md)
- `wparse` 运行时使用指南: [runtime.md](runtime.md)
- 项目工具使用指南: [project.md](project.md)
