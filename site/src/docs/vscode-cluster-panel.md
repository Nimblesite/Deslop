---
layout: layouts/docs.njk
title: VS Code Cluster Panel
description: Field guide for the Deslop VS Code cluster detail panel, including cluster ids, ranking, signals, occurrences, and comparison actions.
eleventyNavigation:
  key: VS Code Cluster Panel
  order: 5
icon: account_tree
---

# VS Code Cluster Panel

The cluster panel is the detailed view behind a Deslop duplicate-code finding. It shows one cluster, the reason Deslop grouped the locations, and the editor actions available for inspecting or comparing the copies.

## Cluster Id

The cluster id is the stable handle for this duplicate-code group. It is derived from the cluster content, so the same clone keeps the same id across refreshes unless the underlying code changes enough to form a different cluster.

Use the id when you need to reference the finding in an issue, an agent prompt, or the MCP `cluster-by-id` flow.

## Clone Bucket

The bucket label is the human-readable clone type:

| Bucket | Meaning |
| --- | --- |
| Identical code | The copies are structurally the same after normalization. |
| Nearly identical code | The copies are close, but small differences may matter. |
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

Size is the number of cloned AST members represented by the cluster. A larger size usually means the duplicated unit is a bigger structural fragment.

## Occurrence Count

The occurrence count is the number of editor locations in this cluster. It may be larger than the rows shown when the live wire has truncated a very large cluster.

## Canonical

The canonical occurrence is the first occurrence Deslop uses as the comparison anchor. Compare opens other occurrences against this anchor so the diff has a consistent left and right side.

Canonical does not mean "best" or "source of truth." It is just a stable anchor for navigation and comparison.

## Signals

Signals explain why the locations were grouped. Scores are shown from `0.00` to `1.00`; higher means that signal saw stronger similarity.

## Structural

`structural` measures AST-shape similarity after identifiers and literals are normalized. High structural score catches exact and renamed clones.

## Jaccard

`jaccard` measures normalized token overlap after formatting, comments, and trivia are ignored. High Jaccard score catches near misses that still share most of their text.

## Embedding

`embedding` measures semantic similarity from the selected local embedding model. It can find code that behaves similarly even when the syntax diverges.

Embeddings are off in a fresh live session until a model is selected.

## Fused

`fused` is Deslop's combined clone score. It joins structural, token, and embedding evidence and is the score used to decide whether a pair is reportable.

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
