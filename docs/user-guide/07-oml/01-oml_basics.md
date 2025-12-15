# OML 语言基础

本文档介绍 OML (Object Modeling Language) 的基础语法和核心概念。

## 什么是 OML

OML 是一种对象构建语言，用于对解析后的数据进行组装与聚合。

## 读取模式（read 与 take）

OML 在读取输入记录时提供两种语义明确的模式：

- read（非破坏性）
  - 多次读取同一键仍可读到值；
  - 先从目标记录 dst 查找，未命中再从输入 src 克隆值；
  - 典型用途：同一字段被多个目标复用，或不希望“消费”源字段。

- take（破坏性）
  - 读取后会将该键从 src 中移除；后续再次 take 同名键将取不到；
  - 典型用途：一次性消费字段、避免被后续逻辑重复使用。


示例：

```oml
name : read_take_diff
---
# 假设输入 src: A1=hello, B1=world

# 非破坏性读取：可重复
X1 = read(A1) ;   # X1 <- hello
X2 = read(A1) ;   # X2 <- hello（仍可读到）

# 破坏性读取：取走后 src 移除
Y1 = take(B1) ;   # Y1 <- world（同时从 src 移除 B1）
Y2 = take(B1) ;   # 取不到（B1 已不在 src）
```
