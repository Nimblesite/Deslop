---
layout: layouts/blog.njk
title: "Towards 100% Accuracy: Transparent Duplicate Code Detection"
date: 2026-08-21
updated: 2026-08-21
author: Christian Findlay
tags:
  - posts
  - duplicate-code-detection
  - code-quality
  - technical-debt
  - ai-coding-agents
  - transparency
category: engineering
description: "See why Deslop publishes its issue graph, known false positives, false negatives, and the research-backed path toward accurate duplicate code detection."
excerpt: "Deslop's push toward 100% accuracy is public: the issue graph exposes known failures, dependencies, verification work, and the research behind every accuracy decision."
heroImage: "/assets/img/blog/towards-100-percent-accuracy-header.webp"
heroImageWidth: "1600"
heroImageHeight: "900"
heroImageAlt: "A physical accuracy audit on a bright drafting table, with a transparent calibration plate, a red correction circle, a loupe, and precision calipers."
ogImage: "/assets/img/blog/towards-100-percent-accuracy-og.png"
ogImageWidth: "1200"
ogImageHeight: "630"
ogImageAlt: "A physical precision audit using a transparent calibration plate and one clearly marked misalignment."
---

“100% accuracy” is the goal. It is not the current score, and this post is not a victory lap.

For Deslop, the standard has two parts:

> Every reported cluster is a real duplicate, and every real duplicate is reported.

The first part is **precision**: no false positives. The second is **recall**: no false negatives. A duplicate code detection tool that optimizes only one can look impressive while failing its users. Report almost nothing and precision looks safe. Report every vaguely similar block and recall looks broad. Neither result is useful.

The honest path toward 100% is to expose the distance still left to travel.

## Why publish an open issue graph?

<figure>
  <a href="/issues/" aria-label="Open the interactive Deslop issue graph">
    <img src="/assets/img/blog/issue-graph-1600.webp"
         srcset="/assets/img/blog/issue-graph-800.webp 800w, /assets/img/blog/issue-graph-1600.webp 1600w"
         sizes="(max-width: 48rem) calc(100vw - 3rem), 48rem"
         alt="Deslop's issue graph with 103 open issues grouped into eight colour-coded workstreams, connected by blocking, parent–sub-issue, and cross-reference lines."
         width="1600" height="943" loading="eager" decoding="async">
  </a>
  <figcaption>The public issue graph captured on 21 August 2026. Select the image to open the current interactive graph.</figcaption>
</figure>

This is a snapshot of Deslop's [public issue graph](/issues/) on 21 August 2026. At capture time, it contained 103 open issues grouped into eight workstreams. Thirty-three sat in Accuracy, the largest group. The captured dataset recorded 22 accuracy-critical issues, three release blockers, 16 fixes awaiting release verification, 82 connected issues, and 122 explicit relationships. The [live issue data](/assets/data/issues.json) is regenerated from GitHub and may already show a newer count.

Those numbers are not a roadmap performance. They are the open work.

Each node is a GitHub issue. Colour identifies the workstream. Larger nodes have more inbound links. Directed lines show blocking and parent–sub-issue relationships; lighter lines show cross-references. Select a node and the graph exposes its priority, labels, assignee, relative effort, connected issues, and a link to the full public ticket.

The green rings matter most. They mean **fixed on main, awaiting verification**. They do not mean done. An issue stays open until the change is exercised in a real release. That distinction prevents a merged patch from being mistaken for an observed result.

This graph exists because a flat issue list hides systems. A false negative in semantic matching may depend on an embedding fix, a corpus assertion, report-schema work, and release verification. Closing one ticket does not make that chain disappear. The graph shows the chain.

## Code quality needs a public failure ledger

Radical transparency is easy when the list is short. It becomes useful when the list is uncomfortable. Hidden duplicate-code technical debt is still debt; a public failure ledger makes it inspectable.

Deslop publishes known false positives, false negatives, misleading metrics, skipped tests, performance limits, and documentation drift in the same place as features. The [issue planner](/issues/planner/) presents the same source data as priority lanes, a recommended queue, statistics, and an indicative runway. Its effort units express relative sequencing, not dates, deadlines, or promises.

That separation is deliberate:

- The **issue graph** answers: what is connected, and what blocks what?
- The **priority board** answers: what class of work comes first?
- The **queue** answers: what is the recommended order right now?
- The **statistics view** answers: how much open and verification work is visible?
- The **runway** answers: how could the work sequence across streams, without pretending uncertainty is a calendar?

The underlying method is also public. The report is generated from GitHub metadata, explicit relationships, cross-references, and documented keyword rules, with no AI enrichment. You can inspect the [open GitHub issues](https://github.com/Nimblesite/Deslop/issues), the [generated JSON](/assets/data/issues.json), and the [generator source](https://github.com/Nimblesite/Deslop/blob/main/scripts/issues/generate_issue_report.py).

## Accuracy is not the duplication percentage

One of the easiest ways to mislead people is to give two different ideas the same label.

Deslop's repository duplication percentage is exact arithmetic over the visible findings in one report. It measures the union of reported duplicated lines divided by analysed lines. It is **not** a statistical estimate of detector accuracy, and it does not prove perfect precision or recall. The full formula, exclusions, rounding behaviour, and CI threshold semantics are published in [Accuracy Transparency](/docs/accuracy-transparency/).

That distinction has practical consequences. If a false positive survives the detector, the percentage can be exactly calculated and still describe the codebase incorrectly. If a real duplicate is missed, the arithmetic can still be exact while the numerator is incomplete. Transparent maths is necessary; validated detection is the harder problem.

## How the goal connects to code clone detection research

The accuracy push did not start from a blank page. Deslop combines several established lines of code clone detection research, each aimed at a different part of the precision–recall problem.

- **Type-1** — copied code with layout or comment changes. Deslop reports it as **Identical code**.
- **Type-2** — the same structure with identifiers renamed or literals changed. Also **Identical code**.
- **Type-3** — statements inserted, removed, or altered. Reported as **Nearly identical code**.
- **Type-4** — similar behaviour expressed through different syntax or structure. Reported as **Same behavior, different code**.

The bold names are what a Deslop report prints; the types are the research vocabulary the rest of this section uses.

[Baxter and colleagues' AST research](https://leodemoura.github.io/files/ICSM98.pdf) showed why parsed program structure can find exact and near-miss clones that line comparison misses. Deslop follows that foundation with tree-sitter syntax trees, identifier and literal normalization, and bottom-up Merkle fingerprints.

[SourcererCC](https://arxiv.org/abs/1512.06448) established a scalable token-based path for near-miss clone detection. Deslop adapts that direction to normalized AST-kind sequences, then uses MinHash locality-sensitive hashing to find **Nearly identical code** [Type-3] candidates without comparing every subtree with every other subtree.

For semantic similarity, [SSCD](https://onlinelibrary.wiley.com/doi/full/10.1002/spe.3355) provides the relevant BERT-plus-nearest-neighbour precedent. Deslop's optional embedding layer uses an HNSW index to widen recall toward **Same behavior, different code** [Type-4] clones. Structural, token, and embedding evidence are fused, clustered, ranked, and rendered through one engine. The implementation map and primary sources are collected in [Research Background](/docs/research-background/).

Each extra layer can recover real duplicates. Each can also admit convincing nonsense. Normalization can erase a meaningful difference. A threshold can hide a real near-miss. An embedding can place two unrelated functions close together. Transitive clustering can join findings that should remain separate. Ranking can bury a correct cluster so deeply that it might as well be absent.

The research provides methods. It does not grant accuracy by association.

## How Deslop measures progress toward 100%

Accuracy work has to turn examples into assertions.

Deslop's fixture tests pin the expected cluster, occurrence count, file paths, evidence bucket, and ranking order. Its [real-repository corpus gate](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/corpus.md) adds human-curated ground truth against pinned public repository commits:

- `must_find` entries fail when a verified duplicate is missed.
- `must_find_type2` entries check renamed clones and their visible evidence.
- `must_not_cluster` entries fail when verified non-duplicates are reported together.
- known failures remain tied to open issues instead of being silently re-baselined.

Every confirmed false positive or false negative should become a regression case. A failing assertion is evidence. A green test without a meaningful oracle is only green paint.

This is also why AI coding agents matter to the accuracy goal. Agents can produce code faster than a reviewer can build repository-wide memory. Deslop's live LSP server keeps the workspace report current, while its MCP server lets an agent call `find-similar` before adding another implementation. Prevention is useful only if the answer is trustworthy, so the same precision and recall standard applies to the editor, CLI, MCP tools, reports, and metrics.

## What “towards” means

[Research on AST-based clone detection](https://leodemoura.github.io/files/ICSM98.pdf) notes that semantic equivalence between arbitrary program fragments cannot be decided in general. “Towards 100%” therefore describes a direction and an operating rule, not a universal proof:

1. Publish the goal without claiming it has been reached.
2. Publish the known counterexamples.
3. Convert each counterexample into a durable test.
4. Keep fixes open until release verification.
5. Expose the calculations, evidence, dependencies, and remaining work.

The [issue graph](/issues/) is the visible ledger for that rule. The [research background](/docs/research-background/) explains where the detection methods come from. The [accuracy documentation](/docs/accuracy-transparency/) explains what the reported numbers do—and do not—mean.

If Deslop ever earns the right to say “100%” about a bounded benchmark, the evidence will be public. Until then, so is the gap.
