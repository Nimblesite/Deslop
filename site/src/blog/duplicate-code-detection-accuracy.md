---
layout: layouts/blog.njk
title: "Deslop 0.35.0: Accuracy in Duplicate Code Detection"
date: 2026-09-27
updated: 2026-11-11
author: Christian Findlay
tags:
  - posts
  - duplicate-code-detection
  - code-quality
  - technical-debt
category: engineering
description: "Deslop 0.35.0 improves duplicate code detection: clearer comparisons, code duplication metrics, and fixes for false matches, missed copies and partial ranges."
excerpt: "Finding a pair is only part of accuracy. Deslop also needs to report the complete copied code, reject unrelated lookalikes and count the right lines."
heroImage: "/assets/img/blog/accuracy-complete-shapes.webp"
heroImageWidth: "1600"
heroImageHeight: "840"
heroImageAlt: "Matching graphite contours beside a differently shaped offcut, illustrating complete structural comparison."
ogImage: "/assets/img/blog/accuracy-complete-shapes-og.jpg"
ogImageWidth: "1200"
ogImageHeight: "630"
---

A duplicate code detector earns its place in code review by finding copies worth inspecting. False positives waste review time. False negatives leave the same bug waiting to be fixed in another location. Incomplete findings can identify a copy while hiding most of it.

Deslop **0.35.0** addresses these problems through clone classification, complete-range recovery and clearer comparison evidence. [Download the release](/releases/).

## Why accuracy decides whether a detector gets used

A duplicate code detector produces a list of claims: *these ranges are copies of each other*. Each kind of error costs something different.

- **A false positive** sends a reviewer, or an AI agent, to merge code that should stay separate. Merge two functions that only look alike and you couple code that was never meant to change together.
- **A false negative** leaves a copy unreported. The next bug fix lands in one place, and the other copy keeps the bug.
- **An incomplete range** reports the right copy but shows only part of it. A refactor based on that range extracts half a function and leaves the rest duplicated.

Teams stop reading a report when it is noisy, and AI agents act on whatever they are given. Both need findings that are correct when they arrive. That is why this release concentrates on finding the right copies and reporting their full extent, rather than adding features.

## Find the whole copy

One defect could split a long, exact copy into short overlapping findings. Recognizing that two files share code was insufficient: a reviewer needed the complete copied range.

The [range recovery change](https://github.com/Nimblesite/Deslop/pull/570) joins overlapping exact findings when the complete source ranges match byte for byte and form valid consecutive syntax-tree siblings. If a valid join is impossible, copied windows must remain visible instead of being discarded. Its CLI test checks the entire copied text, both file paths, exactly two occurrences, the category and the rank.

Other [recall changes](https://github.com/Nimblesite/Deslop/pull/567) recover copied setup inside edited methods and copies within one file. The boundaries matter: reporting shared setup must not drag unrelated neighbouring methods into the finding.

## Separate renamed copies from unrelated lookalikes

Renaming variables or changing test messages does not necessarily change the work being copied. Those cases belong under **Nearly identical code**, alongside small edits. Larger edits with substantial copied work belong under **Similar code**.

Matching layout alone is weaker evidence. **Same shape, different content** is informational and contributes nothing to clone counts, duplicated mass or duplication percentages. A repeated control-flow skeleton is not enough to declare technical debt.

The content checks distinguish systematic renaming from changed operations. Renaming a collaborator can preserve a copy; asking that collaborator to perform a different operation needs different evidence. **Identical code** requires source-text agreement under the whitespace rules, rather than merely equal normalized trees. The [category guide](/docs/how-it-works/#read-the-categories) explains how these labels relate to Type I, II, III and IV clones.

## Make each comparison inspectable

Similarity measurements now belong to the two locations actually being compared. A large cluster no longer presents one selected pair's score as confidence for the whole group.

In VS Code, **Compare To Canonical** compares with the reference copy. **Select for Compare** and **Compare with Selected** let you choose another pair. The CLI's new `--compare` returns the engine's verdict for two explicit source ranges. Mass orders the work to review; it does not certify that extraction is safe.

Build artifacts are also excluded by recognized names and line shape, and generated-file banners are read from parsed comments. These checks keep the analysis focused on source code without treating a banner-like string literal as a generated-file declaration.

## Measure accuracy beyond a pair count

The clone registers contain **CLEARLY IN** pairs that must be found and **CLEARLY OUT** pairs that must not be reported as clones. Uncertain examples are recorded without forcing a verdict. Comparisons use pinned repository revisions and account for older category names: an old shape-only result cannot satisfy a known-clone assertion.

We also inspect **clone-range coverage**: how much of the judged copied ranges the findings cover. A detector that finds only a small fragment of every copy can look good on pair correctness while still missing code users need to see.

A repository's duplication percentage answers a separate question: what fraction of analysed lines is covered by reported clones? It is not an accuracy score. Overlapping ranges count once, and shape-only findings count zero. [Accuracy Transparency](/docs/accuracy-transparency/) documents the calculation and the limits of corpus measurements.

The release work makes specific defects testable and comparisons easier to inspect. That is the evidence to ask of a static code analysis tool: which false matches were rejected, which missed copies were recovered, and whether the report shows the code a reviewer actually needs.

## Check a finding yourself

You do not have to take a cluster on trust. Every occurrence in the JSON report carries exact byte offsets, and the CLI can recompute the verdict for any two of them:

```text
deslop . --compare "src/orders.ts:1204:1980" --compare "src/invoices.ts:310:1086"
```

The command scans the project, recomputes the evidence for that pair, and prints a JSON verdict. Offsets are bytes, the end is exclusive, and each range has to match an occurrence in the report. In VS Code the same check runs from the cluster panel, with the two ranges side by side in the diff editor.

If a verdict looks wrong, that is the most useful bug report Deslop can receive: two paths, two ranges and the version number. Confirmed false positives and false negatives become fixtures in the test suite, so the same mistake fails the build if it comes back. The [public issue graph](/issues/) shows which accuracy work is open, and [Towards 100% Accuracy](/blog/towards-100-percent-accuracy/) explains why those failures are published.

## Upgrading to 0.35.0

Category names changed in this release. Reports now use **Identical code**, **Nearly identical code**, **Similar code**, **Same behavior, different code** and **Same shape, different content**. If your scripts read the JSON report, check the [category guide](/docs/how-it-works/#read-the-categories) before upgrading. Ranking now uses duplicated mass alone, so cluster order can change even where the findings did not; [the ranking post](/blog/ranking-formula/) explains the formula. Duplication percentages exclude shape-only findings, so a repository with many repeated control-flow skeletons may report a lower figure than before. A lower figure here reflects a more accurate count, not deleted duplicates.

