---
layout: layouts/blog.njk
title: Deslop 如何为重复代码簇排名
date: 2026-04-15
author: Christian Findlay
tags: posts
description: Deslop 按语法树大小与额外可见副本数排列重复代码。仅形状相同的结果没有重复质量。
excerpt: 当前排名使用重复质量，不再乘以旧的字节跨度因子。
heroImage: /assets/img/blog/towards-100-percent-accuracy-header.webp
heroImageWidth: '1600'
heroImageHeight: '900'
heroImageAlt: 绘图桌上的精密测量工具。
ogImage: /assets/img/blog/towards-100-percent-accuracy-og.png
ogImageWidth: '1200'
ogImageHeight: '630'
lang: zh
updated: '2026-09-27'
---

本文已按 **0.35.0** 更新。旧公式包含字节跨度的对数因子；当前排名只使用重复质量。

## 公式

```text
mass = canonical_node_count × max(visible_members − 1, 0)
```

第一个因子衡量参考位置的语法树大小，第二个统计额外可见副本数。复制的代码越多、副本越多，重复质量越大。

克隆按质量降序排列，相同时用稳定 ID 排序。**Same shape, different content** 仅供参考：重复质量为零，没有克隆排名，排在克隆之后。

质量不衡量置信度或提取安全性，也不包含语言、文件历史、字节跨度或相似度乘数。[重复百分比](/zh/docs/accuracy-transparency/)另行衡量被覆盖源代码行的并集。

规范见[按重复质量排名](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/pipeline.md#rank-mass-sum-rank-by-duplicated-mass-only)，报告排序实现位于 [`report_weight.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report_weight.rs)。
