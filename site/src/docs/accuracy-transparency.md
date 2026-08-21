---
layout: layouts/docs.njk
title: Accuracy Transparency — how duplication percentage is calculated
description: How Deslop calculates duplication percentages and how the CI gate uses the measured value.
keywords: deslop, accuracy, duplication percentage, duplicated lines, ci gate, false positive, false negative
eleventyNavigation:
  key: Accuracy Transparency
  order: 9
icon: fact_check
docsGroup: trust
---

# Accuracy Transparency

Deslop's percentage is exact arithmetic over the findings in the current report. It is not a statistical estimate, and it is not a claim that detection has perfect precision or recall.

It is a raw coverage figure: it measures how much of the codebase the detector currently flags, taking every surviving finding at face value. Nothing in it is discounted for how strong the evidence behind a finding is.

## How the percentage is calculated

```text
duplication_percent = clamp(100 × duplicated_loc / analysed_loc, 0, 100)
```

- `analysed_loc` is the number of physical lines in every analysed source file. Blank and comment lines count; an empty file contributes zero. Files excluded before analysis contribute nothing.
- `duplicated_loc` is the per-file union of physical line numbers touched by non-hidden occurrences in clusters that survive the report filters. A line covered by overlapping occurrences or clusters is counted once.
- `report_hide` and generated-header occurrences do not enter the numerator. Generated files still remain in `analysed_loc`, so they can lower the percentage ([#385](https://github.com/Nimblesite/Deslop/issues/385)). Beyond that, no finding is discounted: rank, size, bucket and confidence all carry equal weight. A line whose only evidence is a matching code shape — the kind the report itself labels *"verify before extracting"* — counts exactly like a line in a copy proven identical byte for byte ([#344](https://github.com/Nimblesite/Deslop/issues/344)).
- Per-file percentages use the same calculation. Folder percentages sum the files' duplicated and analysed line counts, then divide; they are never averages of file percentages. The engine performs this summing and division itself and carries the folder rows on the report — every percentage on every surface (CLI, LSP, MCP, editors) is computed by this one function in the engine; clients only display the numbers.
- A zero-line corpus reports `0%`. JSON carries the full floating-point value; human-facing reports round it for display.

The implementation is public: [`render_report` selects the visible cluster set](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report.rs), [`compute_repo_metrics` unions the covered lines](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report_metrics.rs), and the same file's `percent` function performs the division and clamp.

## How the CI gate works

Set `--fail-over <percent>` or `[threshold] max_duplication_percent` in `.deslop.toml`. The CLI flag overrides the config value; `--no-fail-over` disables the gate for that run.

The gate fails only when the full-precision measured value is **greater than** the ceiling. Equality passes. A breach writes the reports, then exits `3`; without a threshold, duplication alone never fails the run. Thresholds must be finite values from `0` to `100`.

The exact comparison is in [`ThresholdSummary::resolve`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report_metrics.rs), CLI precedence and exit handling are in [`main.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop/src/main.rs), and the [GitHub Action preserves the reports before re-raising the exit code](https://github.com/Nimblesite/Deslop/blob/main/action.yml).

Treat the percentage as an exact measurement of the report's current visible findings, not as ground truth about every duplicate in the codebase.
