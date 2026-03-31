# `wp-update` 仓库布局

- 状态: Draft
- 范围: `https://github.com/wp-labs/wp-update.git`

## 背景

`wp-update-core`、`wp-self-update`、`wp-installer` 已经从 `warp-parse` 主功能中拆出，后续需要独立发布并服务多个二进制项目。

这些 crate 需要独立生命周期，但没有必要拆成三个仓库。

## 结论

采用单独仓库 `wp-update`，并在仓库内维持一个 Rust workspace。

仓库内包含三个 crate：

- `wp-update-core`
- `wp-self-update`
- `wp-installer`

不采用三个独立 repo。

## 原因

- 三个 crate 具有明确依赖链：`wp-installer -> wp-self-update -> wp-update-core`
- 接口调整通常需要联动修改与联调
- 发布顺序天然有先后关系，放在同一仓库更容易控制
- 这些能力不再应绑定 `warp-parse` 主仓库生命周期
- 对外复用时，不应要求依赖 `warp-parse` 整个仓库

## 建议目录

```text
wp-update/
├── .github/
├── CHANGELOG.md
├── Cargo.toml
├── README.md
└── crates/
    ├── wp-update-core/
    │   ├── Cargo.toml
    │   ├── README.md
    │   └── src/
    ├── wp-self-update/
    │   ├── Cargo.toml
    │   ├── README.md
    │   └── src/
    └── wp-installer/
        ├── Cargo.toml
        ├── README.md
        └── src/
```

## 根 Cargo.toml 建议

```toml
[workspace]
members = [
    "crates/wp-update-core",
    "crates/wp-self-update",
    "crates/wp-installer",
]
resolver = "2"
```

根清单只承载 workspace 配置，不承载可发布 package。

## 发布策略

发布顺序固定为：

1. `wp-update-core`
2. `wp-self-update`
3. `wp-installer`

各 crate 独立 version，不要求强制同版本。

## 仓库职责

- `wp-update-core`: 通道、manifest、版本比较、通用类型
- `wp-self-update`: 下载、校验、替换、回滚等更新执行逻辑
- `wp-installer`: 首次安装和安装入口封装

`warp-parse` 等业务仓库只消费已发布 crate，不再持有更新实现源码。

## 迁移范围

从 `warp-parse` 迁移以下目录：

- `crates/wp-update-core`
- `crates/wp-self-update`
- `crates/wp-installer`

并从 `warp-parse` 根 workspace 中移除这些成员。

## 迁移后要求

- `warp-parse` 改为依赖发布后的 `wp-self-update`
- manifest 地址不在 crate 内写死，只通过参数或环境变量注入
- 每个 crate 具备自己的 README 和发布说明
- `wp-update` 仓库保留统一 CHANGELOG，用于记录跨 crate 的发布动作

## 参考

- 本地参考项目：`../wp-lang`
- 对应英文版：[../en/wp_update_repo_layout.md](../en/wp_update_repo_layout.md)
