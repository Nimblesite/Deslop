---
layout: layouts/blog.njk
title: 为什么排名公式就是整个产品
date: 2026-04-15
author: Christian Findlay
tags: posts
description: Deslop 通过 clone_node_count × (cluster_size − 1) × log2(1 + spanned_bytes) 对重复代码簇进行排名。最严重的重复永远排在第一行。这就是为什么这个公式不可配置。
excerpt: Deslop 通过 clone_node_count × (cluster_size − 1) × log2(1 + spanned_bytes) 对簇进行排名。工具中的每一个决策都源自这一行。这就是为什么它不可配置。
heroImage: "/assets/img/blog/ranking-formula-header.png"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "展示 Deslop 固定排名公式以及最严重者优先报告的头图。"
lang: zh
---

一个报告了各个簇却不对它们排名的重复检测工具，就像一个按插入顺序返回结果的搜索引擎。你可以告诉用户"这里有 142 个簇"，而你只是把问题从工具转移给了人。报告的第一行是初看时唯一重要的一行。Deslop 中其他的一切都是为了让第一行正确。

## 公式

```
weight = clone_node_count × (cluster_size − 1) × log2(1 + spanned_bytes)
```

实现于 [`crates/deslop-core/src/cluster.rs::rank_weight`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/cluster.rs)。三个因子，全部相乘，外加一个对数阻尼器。

**`clone_node_count`** —— 重复片段的 AST 节点数。一个五节点的 getter 没什么意思。一个带嵌套控制流的五十节点方法才有意思。节点数是我们能拿到的对"重复了多少工作量"最接近的代理指标。

**`cluster_size − 1`** —— 第一个成员之外的*额外*成员数。两份副本算作一对重复。五份副本算作四对。单元素的簇按构造方式得分为零，这是"只出现一次不算重复"这句话在数学上诚实的版本。

**`log2(1 + spanned_bytes)`** —— 以字节为单位的收益规模，由 `log2` 阻尼。字节总数反映了一次提取实际会移动多少代码；而对数则防止单个 5000 行的 vendored 文件压倒四份真正的 50 行方法副本。之所以以字节（而非行数）作为事实来源，是因为 Deslop 在所有地方都通过 `[byte_start, byte_end)` 来定位出现位置 —— 行号仅在渲染时才出现。

将这三者相乘，得到的数字在量纲上是合理的（工作量 × 重复 × 影响半径），并且对每个参数都是单调的。节点数翻倍，权重翻倍；簇大小翻倍则权重不止翻倍 —— 这种提升对小簇最为显著（从两份副本增加到四份，会使 `size − 1` 这一项变为三倍），而随着簇增大则逐渐趋向于恰好翻倍；字节数翻倍则会给对数项加一。

## 公式刻意排除了什么

- **语言权重。** 一个完全相同的 C# 重复和一个完全相同的 Rust 重复，如果它们的 节点数 × (size − 1) × log 跨度 相匹配，得分就完全相同。语言偏好属于配置，而非排名。
- **信号权重。** 排名不会乘以 `embedding_cos` 或 `structural`。这些信号决定一个簇是否存在（融合阈值在 [`pair.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/pair.rs) 中设为 0.85）。一旦被接受，每个簇都在同一标尺上排名。
- **文件年龄 / 变更频率。** 诱人，但错误。陈旧而稳定的重复仍然是重复。加入变更频率因子会掩盖那些团队已经学会与之共处的长期问题 —— 而这恰恰是 Deslop 应该揭示的那类问题。
- **用户可配置的权重。** 不容商量。如果每个团队都调自己的权重，跨仓库比较就毫无意义，而博客文章里的"weight = 2184"也就什么都传达不了。

## 这一选择带来的后果

由于排名是一个单一的固定公式，两件事成立：

1. **每份报告都可比较。** 你仓库中最严重的簇可以直接与别人仓库中最严重的簇相比较。数字在任何地方的含义都一样。
2. **排名中的每一个 bug 都是用户可见的 bug。** 如果我在一个次要版本里改了公式，每一条以分数阈值作为门禁的 CI 流水线都会悄无声息地失败。所以这个公式是承重的，其改动要经过与 JSON schema 相同的评审标准。

## 什么会变，什么不会变

信号在演进。嵌入（向量嵌入）模型会变。LSH 的分带会被重新调优。克隆类型的定义可能会新增第五个类别，用于 ML 生成的近似匹配。所有这些都位于排名的下游。

排名公式是我们承诺保持稳定的那一个层面。正是它让 Deslop 成为一个你可以信任的工具 —— 而不是一个按插入顺序返回 142 个簇的搜索引擎。
