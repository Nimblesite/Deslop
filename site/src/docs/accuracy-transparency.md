---
layout: layouts/docs.njk
title: Accuracy Transparency — duplication percentages and known issues
description: How Deslop calculates duplication percentages, how the CI gate uses them, and the open GitHub issues that may affect accuracy.
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

## What we are building next

An evidence-weighted percentage is planned beside the raw figure, with its own opt-in CI gate. Proven copies will weigh more than shape-only resemblance without changing existing thresholds. The design is in [`weighted-metrics-plan.md`](https://github.com/Nimblesite/Deslop/blob/main/docs/plans/weighted-metrics-plan.md), tracked in [#344](https://github.com/Nimblesite/Deslop/issues/344).

## How the CI gate works

Set `--fail-over <percent>` or `[threshold] max_duplication_percent` in `.deslop.toml`. The CLI flag overrides the config value; `--no-fail-over` disables the gate for that run.

The gate fails only when the full-precision measured value is **greater than** the ceiling. Equality passes. A breach writes the reports, then exits `3`; without a threshold, duplication alone never fails the run. Thresholds must be finite values from `0` to `100`.

The exact comparison is in [`ThresholdSummary::resolve`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report_metrics.rs), CLI precedence and exit handling are in [`main.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop/src/main.rs), and the [GitHub Action preserves the reports before re-raising the exit code](https://github.com/Nimblesite/Deslop/blob/main/action.yml).

## Known open accuracy risks

Reviewed 21 August 2026. The [Issue overview](/issues/overview/) is rebuilt from open GitHub issues and is the current exhaustive index. This table groups the active risks that most directly affect results; issues marked `fixed-on-main` remain open in the overview until release verification.

| Risk | Active issues | Possible effect |
| --- | --- | --- |
| Mechanical percentage | [#385](https://github.com/Nimblesite/Deslop/issues/385), [#344](https://github.com/Nimblesite/Deslop/issues/344) | Generated code can lower the raw percentage, while weak and strong visible findings still receive equal weight. |
| False positives and overstated evidence | [#362](https://github.com/Nimblesite/Deslop/issues/362), [#365](https://github.com/Nimblesite/Deslop/issues/365), [#359](https://github.com/Nimblesite/Deslop/issues/359), [#409](https://github.com/Nimblesite/Deslop/issues/409), [#417](https://github.com/Nimblesite/Deslop/issues/417), [#421](https://github.com/Nimblesite/Deslop/issues/421), [#389](https://github.com/Nimblesite/Deslop/issues/389), [#103](https://github.com/Nimblesite/Deslop/issues/103), [#79](https://github.com/Nimblesite/Deslop/issues/79), [#71](https://github.com/Nimblesite/Deslop/issues/71), [#283](https://github.com/Nimblesite/Deslop/issues/283), [#284](https://github.com/Nimblesite/Deslop/issues/284), [#285](https://github.com/Nimblesite/Deslop/issues/285) | Unrelated declarations, tests, tables, calls, or sub-line fragments can be promoted into clusters or stronger buckets. |
| False negatives | [#373](https://github.com/Nimblesite/Deslop/issues/373), [#367](https://github.com/Nimblesite/Deslop/issues/367), [#369](https://github.com/Nimblesite/Deslop/issues/369), [#387](https://github.com/Nimblesite/Deslop/issues/387), [#407](https://github.com/Nimblesite/Deslop/issues/407), [#410](https://github.com/Nimblesite/Deslop/issues/410), [#356](https://github.com/Nimblesite/Deslop/issues/356), [#264](https://github.com/Nimblesite/Deslop/issues/264), [#309](https://github.com/Nimblesite/Deslop/issues/309) | Renamed, near-miss, semantic, embedded, or snippet-query matches can disappear. |
| Stale or conflicting state | [#380](https://github.com/Nimblesite/Deslop/issues/380), [#276](https://github.com/Nimblesite/Deslop/issues/276), [#228](https://github.com/Nimblesite/Deslop/issues/228), [#292](https://github.com/Nimblesite/Deslop/issues/292), [#262](https://github.com/Nimblesite/Deslop/issues/262) | Cache partitions, bundled engine versions, transient assets, or editor state can make surfaces disagree. |
| Corpus and assurance gaps | [#401](https://github.com/Nimblesite/Deslop/issues/401), [#415](https://github.com/Nimblesite/Deslop/issues/415), [#412](https://github.com/Nimblesite/Deslop/issues/412), [#366](https://github.com/Nimblesite/Deslop/issues/366), [#298](https://github.com/Nimblesite/Deslop/issues/298), [#167](https://github.com/Nimblesite/Deslop/issues/167) | Unsound or skipped checks, generated assets, parser errors, and synthetic embedding tests can hide regressions. |

Treat the percentage as an exact measurement of the report's current visible findings, not as ground truth about every duplicate in the codebase.
