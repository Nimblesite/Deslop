---
layout: layouts/docs.njk
title: Accuracy Transparency — duplication percentages and known issues
description: How Deslop calculates duplication percentages, how the CI gate uses them, and the open GitHub issues that may affect accuracy.
keywords: deslop, accuracy, duplication percentage, duplicated lines, ci gate, false positive, false negative
eleventyNavigation:
  key: Accuracy Transparency
  order: 9
icon: fact_check
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
- `report_hide` and generated-header occurrences do not enter the numerator. Beyond that, no finding is discounted: rank, size, bucket and confidence all carry equal weight. A line whose only evidence is a matching code shape — the kind the report itself labels *"verify before extracting"* — counts exactly like a line in a copy proven identical byte for byte. That is what keeps the number reproducible, and it is also why it can read higher than the duplication you would actually act on ([#344](https://github.com/Nimblesite/Deslop/issues/344), [#355](https://github.com/Nimblesite/Deslop/issues/355)).
- Per-file percentages use the same calculation. Folder percentages sum the files' duplicated and analysed line counts, then divide; they are never averages of file percentages. The engine performs this summing and division itself and carries the folder rows on the report — every percentage on every surface (CLI, LSP, MCP, editors) is computed by this one function in the engine; clients only display the numbers.
- A zero-line corpus reports `0%`. JSON carries the full floating-point value; human-facing reports round it for display.

The implementation is public: [`render_report` selects the visible cluster set](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report.rs#L152-L201), [`compute_repo_metrics` unions the covered lines](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report_metrics.rs#L121-L216), and [`percent` performs the division and clamp](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report_metrics.rs#L278-L320).

## What we are building next

A second, evidence-weighted percentage: the same line sets, with each line priced by the strength of the evidence behind the finding that covers it, so a proven copy weighs more than a shape-only resemblance. It gets its own opt-in CI gate and reports beside the raw figure — which keeps its exact meaning and stays the default, so no existing threshold shifts under you. The design, the weights, and the reasoning behind them are in [`weighted-metrics-plan.md`](https://github.com/Nimblesite/Deslop/blob/main/docs/plans/weighted-metrics-plan.md), tracked in [#344](https://github.com/Nimblesite/Deslop/issues/344).

## How the CI gate works

Set `--fail-over <percent>` or `[threshold] max_duplication_percent` in `.deslop.toml`. The CLI flag overrides the config value; `--no-fail-over` disables the gate for that run.

The gate fails only when the full-precision measured value is **greater than** the ceiling. Equality passes. A breach writes the reports, then exits `3`; without a threshold, duplication alone never fails the run. Thresholds must be finite values from `0` to `100`.

The exact comparison is in [`ThresholdSummary::resolve`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report_metrics.rs#L58-L69), CLI precedence and exit handling are in [`main.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop/src/main.rs#L317-L342), and the [GitHub Action preserves the reports before re-raising the exit code](https://github.com/Nimblesite/Deslop/blob/main/action.yml#L180-L250).

## Known open accuracy risks

Reviewed 13 August 2026. This table covers every open issue that can currently change or misstate a finding, signal, bucket, rank, percentage, CI verdict, or cross-surface result. Some are limited to a language, optional embeddings, a release version or a particular configuration.

| Issues | Possible effect |
| --- | --- |
| [#359](https://github.com/Nimblesite/Deslop/issues/359) | Several engine and VS Code defects can promote unrelated members, suppress real logic, lose a TypeScript clone, misclassify Python assertions, or show a stronger verdict than the evidence supports. |
| [#358](https://github.com/Nimblesite/Deslop/issues/358), [#356](https://github.com/Nimblesite/Deslop/issues/356), [#351](https://github.com/Nimblesite/Deslop/issues/351) | With embeddings enabled, real Python matches can be suppressed, ANN bridges can erase or relabel structural findings, and measured cosine evidence can be discarded. |
| [#357](https://github.com/Nimblesite/Deslop/issues/357) | Duplicate subtrees are all indexed by ANN. This is primarily a scale defect, but its repair must preserve every original pair or it can create false negatives. |
| [#355](https://github.com/Nimblesite/Deslop/issues/355) | A Dart family of one-statement delegating methods can surface as duplication and inflate `duplicated_loc` and `duplication_percent`. |
| [#344](https://github.com/Nimblesite/Deslop/issues/344), [#343](https://github.com/Nimblesite/Deslop/issues/343) | The percentage gives low- and high-confidence visible lines equal weight, while fused confidence can saturate at `1.0`; gates and rankings can therefore overstate weak shape matches. |
| [#342](https://github.com/Nimblesite/Deslop/issues/342) | A repository beneath an ancestor named `dist`, `build`, `target` or another built-in exclude can be scanned as zero files and falsely pass clean. |
| [#339](https://github.com/Nimblesite/Deslop/issues/339), [#336](https://github.com/Nimblesite/Deslop/issues/336), [#286](https://github.com/Nimblesite/Deslop/issues/286) | F# token evidence can depend on byte ranges, data tables can dominate the report, and failed embeddings can leave a recall blind spot. |
| [#301](https://github.com/Nimblesite/Deslop/issues/301) | Identical input can produce different cluster sets and percentages between runs, making a gate near its ceiling flaky. |
| [#298](https://github.com/Nimblesite/Deslop/issues/298), [#292](https://github.com/Nimblesite/Deslop/issues/292) | Generated `out/`, coverage, VS Code test, or Playwright report assets can enter the corpus and inflate or destabilise findings and percentages. |
| [#285](https://github.com/Nimblesite/Deslop/issues/285), [#284](https://github.com/Nimblesite/Deslop/issues/284), [#283](https://github.com/Nimblesite/Deslop/issues/283) | Unrelated TypeScript test scenarios and object-literal tables can be promoted to high-confidence nearly identical code (Type-3). |
| [#103](https://github.com/Nimblesite/Deslop/issues/103), [#79](https://github.com/Nimblesite/Deslop/issues/79), [#71](https://github.com/Nimblesite/Deslop/issues/71) | Python test idioms, already-extracted helper calls, and independent HTTP endpoint tests can be reported as actionable duplication. |
| [#309](https://github.com/Nimblesite/Deslop/issues/309), [#264](https://github.com/Nimblesite/Deslop/issues/264), [#263](https://github.com/Nimblesite/Deslop/issues/263), [#262](https://github.com/Nimblesite/Deslop/issues/262) | `find-similar` can miss unique or tracked code and may reject or omit JavaScript/TypeScript, weakening the pre-write duplicate gate. |
| [#276](https://github.com/Nimblesite/Deslop/issues/276), [#228](https://github.com/Nimblesite/Deslop/issues/228) | Different or stale CLI, LSP, MCP and VS Code results can show conflicting clusters, percentages, rankings, and pass/fail verdicts. |
| [#167](https://github.com/Nimblesite/Deslop/issues/167) | Experimental Dart declarative constructors produce parser error nodes, creating a narrow potential detection gap. |
| [#345](https://github.com/Nimblesite/Deslop/issues/345) | Documentation and code disagree about fused-score admission, ranking order and the embeddings default, which can mislead interpretation of otherwise real figures. |
| [#347](https://github.com/Nimblesite/Deslop/issues/347) | The scheduled real-repository accuracy gate fails before scanning, so accuracy regressions can escape that assurance layer. |

We are actively working to fix every accuracy issue. Until an issue is closed with regression coverage, treat the percentage as an exact measurement of Deslop's current visible findings—not as ground truth about every duplicate in the codebase.
