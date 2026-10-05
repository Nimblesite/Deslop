---
layout: layouts/blog.njk
title: How Deslop ranks duplicate-code clusters
date: 2026-04-15
author: Christian Findlay
tags: posts
description: Deslop ranks duplicate code by syntax-tree size and additional visible copies. Shape-only findings have no duplicated mass.
excerpt: The current ranking uses duplicated mass, without the old byte-span multiplier.
heroImage: /assets/img/blog/towards-100-percent-accuracy-header.webp
heroImageWidth: '1600'
heroImageHeight: '900'
heroImageAlt: Precision tools on a drafting table.
ogImage: /assets/img/blog/towards-100-percent-accuracy-og.png
ogImageWidth: '1200'
ogImageHeight: '630'
updated: '2026-09-27'
---

Updated for **0.35.0**. The earlier ranking multiplied by a logarithmic byte-span term; current ranking uses duplicated mass alone.

## The formula

```text
mass = canonical_node_count × max(visible_members − 1, 0)
```

The first factor measures the syntax-tree size of the reference occurrence. The second counts additional visible copies. More copied code and more copies mean more duplicated mass.

Clones appear in descending mass order, with stable IDs breaking ties. **Same shape, different content** is informational: it has zero duplicated mass, no clone rank, and appears after clones.

Mass does not measure confidence or extraction safety. It does not include language, file history, byte span or a similarity multiplier. [Duplication percentage](/docs/accuracy-transparency/) separately measures the union of covered source lines.

The contract is [Rank by duplicated mass](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/pipeline.md#rank-mass-sum-rank-by-duplicated-mass-only); report ordering is implemented in [`report_weight.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report_weight.rs).
