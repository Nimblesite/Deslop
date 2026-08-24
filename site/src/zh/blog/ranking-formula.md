---
layout: layouts/blog.njk
title: Deslop 如何为重复代码簇排名
date: 2026-04-15
author: Christian Findlay
tags: posts
description: Deslop 通过 clone_node_count × (cluster_size − 1) × log2(1 + spanned_bytes) 为重复代码簇排名。本文说明每个因子的含义。
excerpt: Deslop 按片段大小、额外副本数和字节跨度为簇排名，让影响更大的发现排在前面。
heroImage: "/assets/img/blog/ranking-formula-header.webp"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "展示 Deslop 固定排名公式以及最严重者优先报告的头图。"
ogImage: "/assets/img/blog/ranking-formula-og.jpg"
ogImageWidth: "1200"
ogImageHeight: "630"
lang: zh
---

未排序的重复簇列表仍会让用户自行判断该从哪里开始。Deslop 为每个簇计算影响权重，并按权重从高到低排列报告。

## 公式

```
weight = clone_node_count × (cluster_size − 1) × log2(1 + spanned_bytes)
```

实现于 [`crates/deslop-core/src/cluster.rs::rank_weight`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/cluster.rs)。三个因子，全部相乘，外加一个对数阻尼器。

**`clone_node_count`** —— 重复片段的 AST 节点数。节点越多，结构片段越大。

**`cluster_size − 1`** —— 第一个成员之外的*额外*成员数。两份副本算作一对重复。五份副本算作四对。单元素的簇按构造方式得分为零，这是"只出现一次不算重复"这句话在数学上诚实的版本。

**`log2(1 + spanned_bytes)`** —— 经过 `log2` 阻尼的字节跨度，因此特别大的范围不会主导排名。Deslop 使用 `[byte_start, byte_end)` 定位出现位置；行号仅用于显示。

片段越大、额外副本越多、字节跨度越长，分数越高。对数让字节跨度的增长慢于另外两个因子。

## 公式刻意排除了什么

- **语言权重。** 一个完全相同的 C# 重复和一个完全相同的 Rust 重复，如果它们的 节点数 × (size − 1) × log 跨度 相匹配，得分就完全相同。语言偏好属于配置，而非排名。
- **信号权重。** 排名不会乘以 `embedding_cos` 或 `structural`。这些信号决定一个簇是否存在（融合阈值在 [`pair.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/pair.rs) 中设为 0.85）。一旦被接受，每个簇都在同一标尺上排名。
- **文件年龄 / 变更频率。** 公式不使用文件历史，只使用当前报告中的结构性度量。
