---
layout: layouts/blog.njk
title: How Deslop ranks duplicate-code clusters
date: 2026-04-15
author: Christian Findlay
tags: posts
description: Deslop ranks duplicate-code clusters by clone_node_count × (cluster_size − 1) × log2(1 + spanned_bytes). Here is what each factor measures.
excerpt: Deslop ranks clusters by fragment size, additional copies, and spanned bytes so higher-impact findings appear first.
heroImage: "/assets/img/blog/ranking-formula-header.webp"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "Header image showing Deslop's fixed ranking formula and a worst-offender-first report."
ogImage: "/assets/img/blog/ranking-formula-og.jpg"
ogImageWidth: "1200"
ogImageHeight: "630"
---

An unranked list of duplicate clusters leaves the user to decide where to begin. Deslop assigns each cluster an impact weight and sorts the report from highest to lowest.

## The formula

```
weight = clone_node_count × (cluster_size − 1) × log2(1 + spanned_bytes)
```

Implemented in [`crates/deslop-core/src/cluster.rs::rank_weight`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/cluster.rs). Three factors, all multiplicative, with one logarithmic damper.

**`clone_node_count`** — the AST node count of the duplicated fragment. Higher node counts represent larger structural fragments.

**`cluster_size − 1`** — the number of *additional* members beyond the first. Two copies counts as one duplicate pair. Five copies counts as four. A singleton cluster scores zero by construction, which is the mathematically honest version of "one occurrence isn't a duplicate."

**`log2(1 + spanned_bytes)`** — the byte span, dampened by `log2` so very large ranges do not dominate the ranking. Deslop addresses occurrences by `[byte_start, byte_end)`; line numbers are derived for display.

The score increases with fragment size, additional copies, and byte span. The logarithm makes span grow more slowly than the other two factors.

## What the formula deliberately excludes

- **Language weight.** An identical-code C# duplicate and an identical-code Rust duplicate score identically if their nodes × (size − 1) × log spans match. Language preferences belong in configuration, not the ranking.
- **Signal weight.** The ranking does not multiply by `embedding_cos` or `structural`. Those signals gate whether a cluster exists at all (the fused threshold sits at 0.85 in [`pair.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/pair.rs)). Once accepted, every cluster is ranked on the same scale.
- **File age / churn.** File history is not part of the formula. Ranking uses the current report's structural measurements.
