---
layout: layouts/blog.njk
title: "Duplicate Code Detection: Accuracy Changes Since 0.34.0"
date: 2026-09-27
author: Christian Findlay
tags:
  - posts
  - duplicate-code-detection
  - code-quality
  - technical-debt
category: engineering
description: "Deslop's next release targets false matches, missed copies and incomplete clone ranges, with clearer comparisons and duplication metrics."
excerpt: "Finding a pair is only part of accuracy. Deslop also needs to report the complete copied code, reject unrelated lookalikes and count the right lines."
heroImage: "/assets/img/blog/towards-100-percent-accuracy-header.webp"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "A loupe and precision measuring tools on a drafting table."
ogImage: "/assets/img/blog/towards-100-percent-accuracy-og.png"
ogImageWidth: "1200"
ogImageHeight: "630"
---

A duplicate code detector earns its place in code review by finding copies worth inspecting. False positives waste review time. False negatives leave the same bug waiting to be fixed in another location. Incomplete findings can identify a copy while hiding most of it.

The changes on Deslop's `main` branch since **0.34.0** target all three problems. They are preparation for the next packaged release, not a claim that the existing 0.34.0 download includes them. [Releases](/releases/) lists the available packages.

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
