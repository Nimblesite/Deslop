---
layout: layouts/docs.njk
title: VS Code — Reading a duplicate-code cluster in the editor
description: Use the Deslop VS Code extension to review Top Offenders, inspect live duplicate warnings and cluster signals, and compare canonical occurrences.
eleventyNavigation:
  key: VS Code
  order: 5
icon: account_tree
docsGroup: guides
---

# VS Code Cluster Panel

The cluster panel is the detailed view behind a Deslop duplicate-code finding. It shows one cluster, the reason Deslop grouped the locations, and the editor actions available for inspecting or comparing the copies.

## What you're looking at

<figure>
  <a href="/assets/img/screenshot.webp">
    <img src="/assets/img/screenshot.webp"
         alt="The Deslop VS Code extension analysing a live workspace: a worst-first Top Offenders tree and a per-directory Duplication breakdown in the sidebar, a live clone warning in the editor naming the canonical copy with Compare, View cluster and Copy for AI actions, and a side-by-side Compare diff against the canonical occurrence."
         width="2560" height="1492" loading="lazy" decoding="async">
  </a>
  <figcaption>The Deslop VS Code extension on a live workspace — the sidebar (left), the live clone warning in the editor (centre), and the Compare diff against the canonical occurrence (right). Every panel refreshes as you type.</figcaption>
</figure>

Three surfaces are visible, and all of them read the same live report:

- **The sidebar (left)** stacks three views. **Top Offenders** is the worst-first ranked list of every clone cluster in the workspace — each row shows the cluster id, a severity dot, and a plain-English bucket ("Identical code", "Nearly identical code"), and expands to its occurrences; cluster `#1` is the single highest-impact offender, always one click away. **Duplication** drills the tree workspace → folder → file with a duplication percentage on every node — the same repo-wide number a [CI gate](/docs/configuration/#exit-codes) fails on. **Session** shows the running server: the embedding-model picker (the semantic *same behavior, different code* pass, off until you choose a model), the cache size, the file count, and the live analysis State.
- **The editor (centre)** is where the LSP draws the finding inline. The duplicated span is underlined as you type, and a message states the bucket and the copy count — *"Identical code × 3 — Safe to extract — every copy is the same."* — then names the **canonical** occurrence used as the comparison anchor. Three actions sit on the finding: **Compare with canonical**, **View cluster**, and **Copy for AI** (the AI-ready context block, available on every Deslop surface).
- **The Compare diff (right)** is VS Code's native side-by-side editor, opened by **Compare with canonical**: this occurrence on the left, the canonical on the right, with matching rows aligned so you can confirm the duplication before extracting a shared helper.

Everything here is reactive. Edit the code and the tree, the percentages, the inline warning, and the diff all refresh as you type. The same live report backs the MCP tools (`find-similar`, `top-offenders`, `cluster-by-id`), so the agent driving your editor sees the duplicate *before* it writes the copy. The rest of this page is a field guide to each label, score, and action in that view.

## Cluster Id

The cluster id is the stable handle for this duplicate-code group. It is derived from the cluster content, so the same clone keeps the same id across refreshes unless the underlying code changes enough to form a different cluster.

Use the id when you need to reference the finding in an issue, an agent prompt, or the MCP `cluster-by-id` flow.

## Clone Bucket

The bucket label is the human-readable clone type:

| Bucket | Meaning |
| --- | --- |
| Identical code | The copies are structurally the same after normalization. |
| Nearly identical code | The copies are close, but small differences may matter. |
| Same shape, different content | The copies share AST shape only — no token or semantic overlap. Sibling boilerplate; demoted in ranking. |
| Loosely similar code | Deslop found weak overlap. Treat it as a hint, not a verdict. |
| Same behavior, different code | The embedding pass found semantic similarity. Review both locations. |

The sentence under the bucket gives the default reading for that bucket. It is guidance, not an automatic refactor instruction.

## AI Match

`AI MATCH` appears when semantic embeddings contributed the decisive signal. This usually means the code looks different but appears to do the same job.

Do not merge a semantic match blindly. Read both occurrences and use Compare before extracting shared code.

## Rank

The rank badge shows where this cluster sits in the current report. `#1` is the worst offender by duplication impact. The color bucket follows the same ranking policy used by diagnostics and the Top Offenders tree.

## Weight

Weight is Deslop's duplication impact score. Higher weight means the duplicated fragment is larger, copied more often, or spans more source. Use it to decide what to inspect first.

Weight is not a percentage and it is not a CI gate. Use repository duplication percent and thresholds for pass or fail decisions.

## Size

Size is the number of raw clone members combined into the cluster before overlapping same-file members are collapsed. It does not measure fragment length.

## Occurrence Count

Occurrence count is the authoritative number of editor locations after overlapping same-file members are collapsed. It can exceed the rows shown when a large cluster is truncated for display.

## Canonical

The canonical occurrence is the first occurrence Deslop uses as the comparison anchor. Compare opens other occurrences against this anchor so the diff has a consistent left and right side.

Canonical does not mean "best" or "source of truth." It is just a stable anchor for navigation and comparison.

## Signals

Signals explain why the locations were grouped. Scores are shown from `0.00` to `1.00`; higher means that signal saw stronger similarity. The first four are confidence; the three under Content Evidence are what Deslop measured inside the matched code.

## Structural

`structural` measures AST-shape similarity after identifiers and literals are normalized. High structural score catches exact and renamed clones.

## Jaccard

`jaccard` measures normalized token overlap after formatting, comments, and trivia are ignored. High Jaccard score catches near misses that still share most of their text.

## Embedding

`embedding` measures semantic similarity from the selected local embedding model. It can find code that behaves similarly even when the syntax diverges.

Embeddings are off in a fresh live session until a model is selected.

## Fused

`fused` is Deslop's combined clone score. It joins structural, token, and embedding evidence and is the score used to decide whether a pair is reportable. A shape match is discounted by the content evidence below, so `fused` can sit far under a perfect structural score.

## Content Evidence

Shape alone cannot tell a renamed copy from unrelated code that happens to share a skeleton. Two clusters can both score `structural 1.00` and `jaccard 1.00` while one is a genuine duplicate and the other is sibling boilerplate — the same `if/else` skeleton around entirely different code. Content Evidence is what Deslop measured inside the match, and it is what discounts the shape score into the fused confidence.

The panel prints a plain-English reading of the two together under the bars, so you do not have to do the arithmetic: it names the shape score, the measured agreement, and the confidence they produced.

## Agreement

`agreement` is how much of the matched content the locations genuinely share, byte for byte. Low agreement under a high shape score means the skeleton lined up but the code inside it did not.

## Rename Consistency

`rename` is whether one consistent identifier renaming explains every difference between the locations. This is what tells a real renamed copy apart from unrelated code that merely shares a shape: a cluster with `agreement 0.10` and `rename 1.00` is the same code with different names, and worth extracting.

## Literal Fraction

`literal` is how much of the match is literal data rather than logic. A match that is mostly literals is a data table, not a function worth extracting.

## Occurrences

Occurrences are the concrete file locations where the clone appears. Each row shows a human editor target, not raw byte offsets.

## Occurrence Location

The occurrence location is the file plus line and column that Open will navigate to. When line and column are unavailable, the panel falls back to the path and tells you the source file could not be read by the extension host.

## Hidden Occurrence

`hidden` means the path matched `report_hide` configuration. Deslop still knows about the range, but hidden rows do not inflate the visible report ranking.

## Open Action

Open moves VS Code to the occurrence and selects the clone range.

## Compare Action

Compare opens VS Code's diff editor with the selected occurrence against the canonical occurrence. It is disabled on the canonical row because comparing the anchor to itself would not show useful information.

## Cluster Navigation

Previous cluster and Next cluster move through the same worst-first list as the Top Offenders view. They update the selected cluster locally inside the webview.

## Keyboard Shortcuts

The panel supports keyboard navigation while focus is inside the webview:

| Shortcut | Action |
| --- | --- |
| `j` / `k` | Move the focused occurrence row. |
| `n` / `p` | Move to the next or previous cluster. |
| `Enter` | Open the focused occurrence. |
| `?` | Toggle detailed keyboard help. |
