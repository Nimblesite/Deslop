---
layout: layouts/docs.njk
title: VS Code — Inspect and Compare Duplicate Code
description: Read clone categories and mass, compare canonical or arbitrary occurrences, and group Deslop findings in VS Code.
eleventyNavigation:
  key: VS Code
  order: 5
icon: account_tree
docsGroup: guides
---

# VS Code Cluster Panel

The panel shows a group of code occurrences and lets you inspect the actual copies. This guide covers [0.35.0](/releases/).

## Open and compare

- Click a **file link** to open and select its code range. There is no separate Open button.
- **Compare To Canonical** opens the group's reference occurrence on the left and the clicked occurrence on the right. The canonical row is labelled and cannot compare with itself.
- For another pair, choose **Select for Compare** on the left-hand occurrence, then **Compare with Selected** on the other. **Clear Selection** cancels the choice. Selection clears after comparison or when you change clusters.
- **Copy Context For AI** supplies the finding's context for an agent.

Canonical means the reference used for navigation, not the best implementation. The diff title identifies both ranges and the engine's verdict for that pair, including exact-byte or indentation-only matches. Two ranges can be in the same file.

## Read the finding

The **clone kind** distinguishes Identical code, Nearly identical code, Similar code, optional Same behavior, different code, and informational Same shape, different content. [How It Works](/docs/how-it-works/#read-the-categories) explains the Type I–IV mapping.

**Mass** is copied syntax-tree size multiplied by additional visible copies. **Rank** orders clones by mass, largest first. Neither is a confidence score. **Occurrence count** is the total number of locations, which can exceed the rows displayed in a large group. **Cluster ID** identifies the finding across surfaces.

**Severity** is the configured diagnostic level for the kind, not its mass rank. Identical and Nearly identical default to warning; Similar and Same behavior default to information. Shape-only has no diagnostic by default and never counts as duplication. Colour identifies kind.

Structural, token, content and embedding measurements describe a pair. They appear in an explicit comparison, not as a score for the entire cluster.

## Use the sidebar

**Top Offenders** groups findings by **Clone Category**, **Folder**, **Language**, **File** or **No Grouping**. Clones stay in descending mass order. One grouping menu replaces separate language and sorting controls.

**Duplication** shows workspace, folder and file percentages from the same engine calculation used in reports. **Session** shows analysis state and embedding settings. The views refresh with the live analysis.

Help buttons open formatted explanations on hover or keyboard focus. **Escape** dismisses help; technical details stay collapsed until needed.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `j` / `k` | Focus the next / previous occurrence. |
| `n` / `p` | Move to the next / previous cluster. |
| `Enter` | Open the focused occurrence. |
| `?` | Toggle keyboard help. |
