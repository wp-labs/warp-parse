# 自动更新设计

- 状态: Draft
- 范围: `wproj self update`

## 背景

Warp Parse 同时发布多个二进制。更新机制需要在不把替换风险带入主运行链路的前提下，以可控、可审计的方式统一升级整套工具。

## 目标

- 为所有二进制提供统一更新入口
- 支持手动检查和手动更新
- 后续支持自动检查，并为自动应用预留能力
- 做好版本和哈希校验
- 安装失败时可回滚

## 非目标

- 首版不做增量补丁
- 首版不做 GUI
- 首版不引入复杂多清单兼容层

## 职责划分

- `wproj`: 唯一执行更新动作的入口
- `wparse`: 后续可提示有新版本，但不负责替换二进制
- `wpgen` 与 `wprescue`: 不承载更新流程

## CLI 形态

规划命令：

```bash
wproj self status
wproj self check
wproj self update
wproj self rollback
wproj self auto enable|disable|set
```

## 本地状态

建议目录：

```text
~/.warp_parse/update/
```

建议文件：

- `policy.toml`
- `state.json`
- `lock`
- `backups/`

## 通道模型

更新通道必须与发布分支严格对齐：

- `stable` <- `main`
- `beta` <- `beta`
- `alpha` <- `alpha`

跨通道升级必须是显式操作。

## Manifest 模型

客户端读取：

```text
updates/<channel>/manifest.json
```

清单至少应包含：

- version
- channel
- 发布时间等元数据
- 平台资产
- sha256 校验值

## 更新流程

1. 读取当前版本
2. 拉取目标 channel 的 manifest
3. 比较版本
4. 下载资产并校验哈希
5. 解压到临时目录
6. 获取更新锁
7. 创建备份
8. 原子替换二进制
9. 运行健康检查
10. 成功落状态，失败则回滚

## 安全约束

- 严格执行 channel 隔离
- 只允许可信来源下载
- 使用文件锁避免并发更新
- 保留回滚备份
- 记录可审计的状态和失败原因

## 包管理器兼容

若检测到安装来自系统包管理器：

- `check` 仍可使用
- `update` 默认拒绝直接替换
- 如有 `--force`，必须显式启用

## MVP 范围

首批实现：

- `status`、`check`、`update`、`rollback`
- manifest 拉取
- 哈希校验
- 全量安装
- 备份与回滚
- 本地策略和状态持久化

后续实现：

- 完整 `auto` 策略
- 运行时版本可用提示
- 更细粒度错误码和可观测性

## 验收标准

- 更新成功后所有二进制版本一致
- 替换失败会自动回滚
- 并发更新不会破坏安装
- channel 映射在流程中被一致执行

## 对应英文版

- [../en/self_update_design.md](../en/self_update_design.md)
