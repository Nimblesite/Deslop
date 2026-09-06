---
layout: layouts/blog.njk
title: Why the ranking formula is the entire product
date: 2026-04-15
author: Christian Findlay
tags: posts
description: Deslop ranks duplicate-code clusters by clone_node_count × (cluster_size − 1) × log2(1 + spanned_bytes). The worst offender is always line one. Here's why the formula is not configurable.
excerpt: Deslop ranks clusters by clone_node_count × (cluster_size − 1) × log2(1 + spanned_bytes). Every decision in the tool flows from that one line. Here's why it's not configurable.
heroImage: "/assets/img/blog/ranking-formula-header.png"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "Header image showing Deslop's fixed ranking formula and a worst-offender-first report."
---

A duplicate-detection tool that reports clusters without ranking them is a search engine that returns results in insertion order. You can tell the user "there are 142 clusters," and you have just transferred the problem from the tool to the human. Line one of the report is the only line that matters on the first look. Everything else in Deslop exists to make line one correct.

## The formula

```
weight = clone_node_count × (cluster_size − 1) × log2(1 + spanned_bytes)
```

Implemented in [`crates/deslop-core/src/cluster.rs::rank_weight`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/cluster.rs). Three factors, all multiplicative, with one logarithmic damper.

**`clone_node_count`** — the AST node count of the duplicated fragment. A five-node getter is not interesting. A fifty-node method with nested control flow is. Node count is the closest proxy we have to "how much effort was duplicated."

**`cluster_size − 1`** — the number of *additional* members beyond the first. Two copies counts as one duplicate pair. Five copies counts as four. A singleton cluster scores zero by construction, which is the mathematically honest version of "one occurrence isn't a duplicate."

**`log2(1 + spanned_bytes)`** — payoff scale, in bytes, dampened by `log2`. The byte total tracks how much code an extraction would actually move; the logarithm prevents a single 5000-line vendored file from dominating four genuine 50-line method copies. Bytes (not lines) are the source of truth because Deslop addresses occurrences by `[byte_start, byte_end)` everywhere — line numbers are render-time only.

Multiplying the three gives a number that is dimensionally sensible (effort × repetition × blast radius) and monotonic in every argument. Doubling the node count doubles the weight; doubling the cluster size more than doubles it — the boost is biggest for small clusters (going from two copies to four triples the `size − 1` term) and settles toward an exact doubling as clusters grow; doubling the bytes adds one to the log term.

## What the formula deliberately excludes

- **Language weight.** An identical-code C# duplicate and an identical-code Rust duplicate score identically if their nodes × (size − 1) × log spans match. Language preferences belong in configuration, not the ranking.
- **Signal weight.** The ranking does not multiply by `embedding_cos` or `structural`. Those signals gate whether a cluster exists at all (the fused threshold sits at 0.85 in [`pair.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/pair.rs)). Once accepted, every cluster is ranked on the same scale.
- **File age / churn.** Tempting, and wrong. Old stable duplication is still duplication. Adding a churn factor would hide long-standing problems that the team has learned to live with — which is precisely the kind of problem Deslop should surface.
- **User-configurable weights.** Non-negotiable. If every team tuned their own weights, cross-repo comparison would be meaningless, and "weight = 2184" in a blog post would communicate nothing.

## The consequence of that choice

Because the ranking is a single fixed formula, two things become true:

1. **Every report is comparable.** The worst cluster in your repo can be directly compared to the worst cluster in someone else's repo. Numbers mean the same thing everywhere.
2. **Every bug in the ranking is a user-visible bug.** If I change the formula in a minor version, every CI pipeline that gates on a score threshold breaks silently. So the formula is load-bearing, and changes go through the same review bar as the JSON schema.

## What changes, what doesn't

Signals evolve. The embedding model will change. The LSH bands will be retuned. Clone-type definitions may pick up a fifth category for ML-generated near-misses. All of that is downstream of ranking.

The ranking formula is the one surface we commit to keeping stable. It is what makes Deslop a tool you can trust — rather than a search engine that returns 142 clusters in insertion order.
