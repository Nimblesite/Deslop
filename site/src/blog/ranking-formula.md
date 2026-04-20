---
layout: layouts/blog.njk
title: Why the ranking formula is the entire product
date: 2026-04-15
author: Christian Findlay
tags: posts
excerpt: Deslop ranks clusters by clone_size × clone_count × spanned_LOC. Every decision in the tool flows from that one line. Here's why it's not configurable.
---

A duplicate-detection tool that reports clusters without ranking them is a search engine that returns results in insertion order. You can tell the user "there are 142 clusters," and you have just transferred the problem from the tool to the human. Line one of the report is the only line that matters on the first look. Everything else in Deslop exists to make line one correct.

## The formula

```
score = clone_size_nodes × clone_count × spanned_LOC
```

Three factors, all multiplicative.

**`clone_size_nodes`** — the AST node count of the duplicated fragment. A five-node getter is not interesting. A fifty-node method with nested control flow is. Node count is the closest proxy we have to "how much effort was duplicated."

**`clone_count`** — the number of members in the cluster. Two copies is a pair. Five copies is an epidemic. The formula rewards epidemics because they compound maintenance burden.

**`spanned_LOC`** — the total source lines covered by the cluster. Two fifty-line methods produce a cluster of 100 LOC; extracting them removes 50. The LOC factor makes the score track refactor payoff, not academic similarity.

Multiplying the three gives a number that is dimensionally sensible (effort × repetition × blast radius) and monotonic in every argument. Doubling any factor doubles the score.

## What the formula deliberately excludes

- **Language weight.** A Type-2 C# duplicate and a Type-2 Rust duplicate score identically if their size × count × LOC match. Language preferences belong in configuration, not the ranking.
- **Signal weight.** The ranking does not multiply by `embedding_cos` or `structural`. Those signals gate whether a cluster exists at all. Once accepted, every cluster is ranked on the same scale.
- **File age / churn.** Tempting, and wrong. Old stable duplication is still duplication. Adding a churn factor would hide long-standing problems that the team has learned to live with — which is precisely the kind of problem Deslop should surface.
- **User-configurable weights.** Non-negotiable. If every team tuned their own weights, cross-repo comparison would be meaningless, and "score = 2184" in a blog post would communicate nothing.

## The consequence of that choice

Because the ranking is a single fixed formula, two things become true:

1. **Every report is comparable.** The worst cluster in your repo can be directly compared to the worst cluster in someone else's repo. Numbers mean the same thing everywhere.
2. **Every bug in the ranking is a user-visible bug.** If I change the formula in a minor version, every CI pipeline that gates on a score threshold breaks silently. So the formula is load-bearing, and changes go through the same review bar as the JSON schema.

## What changes, what doesn't

Signals evolve. The embedding model will change. The LSH bands will be retuned. Clone-type definitions may pick up a fifth category for ML-generated near-misses. All of that is downstream of ranking.

The ranking formula is the one surface we commit to keeping stable. It is what makes Deslop a tool you can trust — rather than a search engine that returns 142 clusters in insertion order.
