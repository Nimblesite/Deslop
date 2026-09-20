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

- **The sidebar (left)** stacks three views. **Top Offenders** is the worst-first ranked list of every clone cluster in the workspace — each row shows the cluster id, the cluster's **clone kind** (*Identical code*, *Nearly identical code*, *Similar code*, *Same behavior, different code*, or *Same shape, different content*) with a colour and icon of its own, and the cluster's mass — and expands to its occurrences; cluster `#1` is the single highest-impact offender, always one click away. **Duplication** drills the tree workspace → folder → file with a duplication percentage on every node — the same repo-wide number a [CI gate](/docs/configuration/#exit-codes) fails on. **Session** shows the running server: the embedding-model picker (the semantic *same behavior, different code* pass, off until you choose a model), the cache size, the file count, and the live analysis State.
- **The editor (centre)** is where the LSP draws the finding inline. The duplicated span is underlined in the cluster's kind colour as you type, and a message names the kind, the **canonical** occurrence and the copy count — *"Identical code × 3"* — with **View cluster** and **Copy for AI** actions (the AI-ready context block, available on every Deslop surface).
- **The Compare diff (right)** is VS Code's native side-by-side editor, opened by **Compare** on an occurrence row. It shows the canonical occurrence on the left and the clicked occurrence on the right so you can inspect exactly those two ranges before extracting a shared helper.

Everything here is reactive. Edit the code and the tree, the percentages, the inline warning, and the diff all refresh as you type. The same live report backs the MCP tools (`find-similar`, `duplicates`, `cluster-by-id`), so the agent driving your editor sees the duplicate *before* it writes the copy. The rest of this page is a field guide to each label, score, and action in that view.

## Cluster Id

The cluster id is the stable handle for this duplicate-code group. It is derived from the cluster content, so the same clone keeps the same id across refreshes unless the underlying code changes enough to form a different cluster.

Use the id when you need to reference the finding in an issue, an agent prompt, or the MCP `cluster-by-id` flow.

## Clone Kind

The heading is the cluster's clone kind: the weakest relation between the cluster's first occurrence and any other member, exactly as comparing that member against the first would report it. **Identical code** means every copy is byte-for-byte the first one. **Nearly identical code** means every copy is an admitted near-copy and at least one differs. **Same behavior, different code** means at least one copy matched on the embedding pass alone. **Similar code** means substantial copied work remains, but statements or control flow have changed enough that it is no longer a near-copy. **Same shape, different content** means the layout matches and almost none of the content is shared — informational, not a clone, and counted toward no duplication figure. Colour follows the kind everywhere: crimson is identical, amber nearly identical, blue similar, violet same behavior, muted grey same shape.

## Severity

The badge's glyph is the cluster's mass rank band in the current report: `●●` for the worst, `●` for the top tenth, `◐` for the upper half, `○` for the faint tail. Severity is a prioritisation signal, not a verdict, and it never chooses the colour — read the occurrences before you merge or extract anything.

## Rank

The rank badge shows where this cluster sits in the current report. `#1` is the worst offender by duplication impact.

## Mass

Mass is Deslop's duplication impact measure: how much source the cluster's canonical extent covers, weighted by membership. Higher mass means a bigger, more widespread duplication. Use it to decide what to inspect first.

Mass is not a percentage and it is not a CI gate. Use repository duplication percent and thresholds for pass or fail decisions.

## Node Count

The node count is the number of raw clone members combined into the cluster before overlapping same-file members are collapsed. It does not measure fragment length.

## Occurrence Count

Occurrence count is the authoritative number of editor locations after overlapping same-file members are collapsed. It can exceed the rows shown when a large cluster is truncated for display.

## Canonical

The canonical occurrence is the first occurrence of the cluster — a stable anchor for navigation and a stable id input. Compare uses it as the left side of the diff and places the occurrence you clicked on the right.

Canonical does not mean "best" or "source of truth."

## Pair evidence is explicit and two-sided

The panel renders cluster facts: rank, band, mass, and occurrences. Similarity measurements — structural shape, token overlap, embedding similarity, content agreement — describe one pair of locations, not a group, so they appear only in an explicit pair comparison you request by selecting both endpoints. No score is pooled, averaged, or attributed to the cluster.

## Occurrences

Occurrences are the concrete file locations where the clone appears. Each row shows a human editor target, not raw byte offsets.

## Occurrence Location

The occurrence location is the file plus line and column that Open will navigate to. When line and column are unavailable, the panel falls back to the path and tells you the source file could not be read by the extension host.

## Hidden Occurrence

`hidden` means the path matched `report_hide` configuration. Deslop still knows about the range, but hidden rows do not inflate the visible report ranking.

## Open Action

Open moves VS Code to the occurrence and selects the clone range.

## Compare Action

Click **Compare** on any non-canonical occurrence to open VS Code's diff editor in one click. The canonical occurrence appears on the left and the occurrence you clicked appears on the right — exactly the clone bytes, even when both live in the same file. Compare is disabled on the canonical row because that would compare a range with itself. The occurrence's context menu offers **Compare With Canonical** too.

To compare any other two occurrences, tap one row and then another: the first tap picks a row, the second opens the diff with the first on the left. Tap the picked row again to unpick it.

The diff's title names both occurrences and the engine's verdict on exactly that pair: **Identical bytes**, **Differs only by indentation** when indentation is the whole difference, or the pair's clone kind. That line is the fastest check that a finding is real: two copies that differ only by indentation are the same code.

To compare any two occurrences, tap one row and then another. The first tap picks the row; the second opens the diff with the first-tapped range on the left. Tap the picked row again to let it go.

The diff's title names both files and states what the engine found for exactly those two ranges: **Identical bytes**, **Differs only by indentation** when the only difference is how the lines are indented, or the pair's clone kind. That title is the quickest way to confirm a reported copy is real.

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
