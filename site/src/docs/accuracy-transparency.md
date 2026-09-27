---
layout: layouts/docs.njk
title: Accuracy Transparency — What Duplication Percentage Measures
description: Understand Deslop duplication percentages, shape-only exclusions, false positives, false negatives and clone-range coverage.
eleventyNavigation:
  key: Accuracy Transparency
  order: 9
icon: fact_check
docsGroup: trust
---

# Accuracy Transparency

Duplication percentage measures the code covered by reported clones. It does not measure how accurate the detector is. This page covers [0.35.0](/releases/).

## How the percentage is calculated

```text
duplication_percent = clamp(100 × duplicated_loc / analysed_loc, 0, 100)
```

- `analysed_loc` counts physical lines in analysed source files, including blank and comment lines. Excluded files contribute nothing; empty files contribute zero.
- `duplicated_loc` counts the per-file union of lines covered by visible clone occurrences. Overlapping ranges count once.
- **Same shape, different content** is informational. It contributes nothing to duplicated lines, files, clone counts, mass or threshold breaches. Its analysed source lines remain in the denominator.
- Occurrences hidden by `report_hide` or generated-file banners do not enter the numerator. Their files remain in the denominator. Use `exclude` to remove a file from analysis entirely.
- Folder percentages divide summed duplicated lines by summed analysed lines. They are not averages of file percentages. The Rust engine computes all percentages; clients display them.
- A zero-line corpus reports `0%`. JSON retains full precision; human-facing reports round for display.

The calculation is in [`report_metrics.rs`](https://github.com/Nimblesite/Deslop/blob/main/crates/deslop-core/src/report_metrics.rs). A surviving false positive can inflate this percentage; a missed clone can reduce it. Correct arithmetic does not establish correct detection.

## How accuracy is checked

The [clone registers](https://github.com/Nimblesite/Deslop/tree/main/corpus/register) record source pairs at pinned repository commits. **CLEARLY IN** means a missed clone is a false negative. **CLEARLY OUT** means reporting the pair as a clone is a false positive. **NOT CLEAR** is recorded but does not assert either verdict.

Version comparisons must account for category changes. An older `structural_only` finding was not a clone: if the register says CLEARLY IN, reporting only that informational match still counts as a miss.

Pair correctness and **clone-range coverage** answer different questions. Correctness asks whether the two known copies were found. Range coverage checks how much of their judged source ranges the reported clone occurrences cover. Detecting a short fragment of a long copy can satisfy the first check while leaving most of the duplication unreported.

The [corpus contract](https://github.com/Nimblesite/Deslop/blob/main/docs/specs/corpus.md) defines the scoring and generated scorecards. Results apply to the judged examples and measured builds; they are not a guarantee of perfect precision or recall across arbitrary repositories.

## How the CI gate works

Set `--fail-over <percent>` or `[threshold] max_duplication_percent`. The CLI flag overrides the config; `--no-fail-over` disables the gate for that run. The value must be finite and between `0` and `100`.

The gate fails only when the full-precision measured percentage is **greater than** the ceiling. Equality passes. A breach writes the reports, then exits `3`. Without a threshold, duplication alone does not fail the run.

With `--diff` and `--only-changed`, the gate uses duplication on the diff's added lines. The whole tree is still analysed so a new copy can match existing code. See [Configuration](/docs/configuration/).
