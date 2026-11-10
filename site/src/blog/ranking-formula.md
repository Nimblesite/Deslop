---
layout: layouts/blog.njk
title: How Deslop ranks duplicate-code clusters by duplicated mass
date: 2026-04-15
updated: 2026-11-11
author: Christian Findlay
tags: posts
description: Deslop ranks duplicate code by syntax-tree size times additional visible copies. Shape-only findings have no duplicated mass and rank after clones.
excerpt: The current ranking uses duplicated mass, without the old byte-span multiplier.
heroImage: /assets/img/blog/ranking-mass.webp
heroImageWidth: '1600'
heroImageHeight: '840'
heroImageAlt: Four stacks of graphite tiles ordered largest first, with a crimson top tile on the tallest stack.
ogImage: /assets/img/blog/ranking-mass-og.jpg
ogImageWidth: '1200'
ogImageHeight: '630'
---

Updated for **0.35.0**. The earlier ranking multiplied by a logarithmic byte-span term; current ranking uses duplicated mass alone.

A duplicate code report is only useful if the first finding is the one worth your time. A large repository can produce hundreds of clusters. Nobody reviews them all, and no AI agent should load them all into its context. The order of the list decides what gets fixed, so the ranking rule has to be simple enough to check by hand.

## The formula

```text
mass = canonical_node_count × max(visible_members − 1, 0)
```

The first factor measures the syntax-tree size of the reference occurrence. The second counts additional visible copies. More copied code and more copies mean more duplicated mass.

Clones appear in descending mass order, with stable IDs breaking ties. **Same shape, different content** is informational: it has zero duplicated mass, no clone rank, and appears after clones.

### Worked example

| Cluster | Nodes | Visible copies | Extra copies | Mass |
| --- | ---: | ---: | ---: | ---: |
| A | 120 | 2 | 1 | 120 |
| B | 40 | 4 | 3 | 120 |
| C | 200 | 1 | 0 | 0 |

Mass is nodes × extra copies.

A and B tie on mass, so their stable IDs decide the order. Cluster C has one visible copy left — the others are hidden or excluded — so it adds no duplicated mass.

## Why syntax-tree nodes, not lines

Lines of code depend on formatting. The same function can be twelve lines under one style guide and twenty under another, and a formatter run would reorder the whole report. Syntax-tree nodes count the code itself: statements, expressions, calls and operators. Deslop already parses every file with [tree-sitter](/blog/tree-sitter-over-regex/), so the node count of the reference copy is available at no extra cost and does not change when someone reformats a file.

The *reference* copy is the cluster's canonical occurrence. Copies in a cluster can differ slightly, for example a renamed variable or one extra statement, so the formula uses one well-defined copy rather than an average.

## Why "additional" copies

Two copies of a function contain one redundant implementation. Five copies contain four. The formula counts the copies you could remove: if every copy were replaced by a call to one shared function, `visible_members − 1` is how many would go.

That is also why the count uses *visible* copies. If you mark generated code with `report_hide`, those occurrences stay in the analysis but no longer count towards mass. A cluster where your hand-written code duplicates a generated file still shows up, because it still has a visible copy you wrote. A cluster that only duplicates generated output drops to zero. [Configuration](/docs/configuration/#exclude-vs-report_hide--the-core-idea) explains the difference between excluding and hiding.

## What mass deliberately leaves out

Mass does not measure confidence or extraction safety. It does not include language, file history, byte span or a similarity multiplier. [Duplication percentage](/docs/accuracy-transparency/) separately measures the union of covered source lines.

Each of those exclusions is deliberate:

- **No confidence multiplier.** Similarity belongs to a specific pair of copies, not to a whole cluster. A cluster's rank should not depend on which pair happened to be scored. Use `--compare` or **Compare To Canonical** in VS Code to inspect any two copies directly.
- **No category weighting.** An identical copy and a nearly identical copy of the same size rank the same. The category tells you how much the copies differ; mass tells you how much code is duplicated.
- **No byte span.** The old logarithmic byte-span term rewarded long identifiers and string literals. Two clusters with the same structure could swap places because one used longer names. Removing it made the order depend on code structure alone.
- **No file history or churn.** Recency and change frequency are useful signals, but they need git history and would make the same snapshot rank differently in different clones of the repository.

A ranking you can recompute from the report alone is one you can trust in code review. Every input to the formula is in the JSON output.

## Reading mass in a report

Every cluster in the JSON report carries its category, its occurrences, its mass and its rank. Mass is printed as a whole number, so two reports from the same code can be compared line by line. The text and HTML reports show the same figure next to each cluster, and the VS Code panel lists clusters in the same order. If a cluster's position surprises you, multiply its node count by its extra copies and compare the result with the clusters above it. No hidden factor is involved.

## Mass is not the duplication percentage

The two figures answer different questions:

| Figure | Question | Unit |
| --- | --- | --- |
| Mass | Which cluster should I look at first? | Syntax-tree nodes × extra copies |
| Duplication percentage | How much of the analysed code is covered by reported clones? | Lines, overlapping ranges counted once |

The percentage is `100 × duplicated_loc / analysed_loc`, where duplicated lines are the per-file union of visible clone ranges. It is the number a [CI threshold](/docs/configuration/#threshold--the-ci-gate) checks. Mass only orders the list. A repository can have a low percentage with one very high-mass cluster, and that cluster is still the right place to start.

## Ranking for humans and AI agents

The same order appears everywhere: the CLI's text and HTML reports, the VS Code cluster panel, and the MCP server's `duplicates` tool. For an AI agent this is what keeps the context small. The agent asks for a short summary, takes the top cluster, decides whether to extract, reuse or accept it, and asks again. Because the order is deterministic, two runs over the same code give the same list. That keeps diffs between reports readable and stops an agent from reacting to noise.

## What to do with the top finding

A high-mass cluster is a strong candidate for a refactoring, not an instruction to make one. Check whether the copies really change together. Large copied blocks that share tests and callers usually belong in one function. Copies that are expected to diverge, such as test fixtures or code migrations, can be hidden with configuration so they stop occupying the top of the list. Either way, the decision is yours.

After the change, the report refreshes. If the copies were merged, the cluster disappears and the next one moves up. If some copies remain, the cluster's mass falls with each removed copy, so progress shows up in the ranking as well as in the duplication percentage.

The contract is [Rank by duplicated mass](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/pipeline.md#rank-mass-sum-rank-by-duplicated-mass-only); report ordering is implemented in [`report_weight.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report_weight.rs).
