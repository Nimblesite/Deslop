# Weighted duplication metrics — implementation plan

Implement [METRICS-REPO-WEIGHTED](../specs/pipeline.md#metrics-repo-weighted) and [EXIT-CODES-WEIGHTED](../specs/pipeline.md#exit-codes-weighted). The normative formulas, defaults, and invariants live in `pipeline.md`; the configuration surface lives in [exclusion.md](../specs/exclusion.md). This file owns the wholesale replacement scope and final acceptance contract only.

## Execution rule — one destructive cutover

Land this as one indivisible replacement. Change the Rust calculation, configuration, CLI, canonical typeDiagram model, generated wire types, every renderer, every client, tests, and documentation in the same hit. Do not introduce compatibility fields, optional legacy paths, adapters, dual calculations, temporary fallbacks, or independently landable phases. Delete any superseded metric plumbing immediately; the repository may fail to compile and every affected test may remain red during the cutover. Restore a compiling build and green final-contract tests only after every producer and consumer has moved to the finished model.

## Contract

The existing `duplication_percent` remains the mechanical, industry-comparable percentage and the default CI gate. The new weighted percentage uses the same visible clusters, line projection, denominator, hidden-occurrence exclusion, and literal-family exclusion; it changes only the contribution assigned to each covered line.

For each duplicated line, Rust computes the maximum `bucket_weight × category_weight` among covering clusters, then sums those line weights. It never reads pair `fused`, elected-pair axes, content support, or any other confidence value. With every configured weight in `[0,1]`, `0 ≤ weighted_duplication_percent ≤ duplication_percent ≤ 100`; all-one weights make the figures exactly equal.

Bucket weights price the engine’s final evidence class. They do not change candidate admission, clustering, routing, visibility, ranking, or the mechanical metric. A misrouted cluster remains a detector defect and must not be hidden by changing a metric weight.

## Wholesale replacement scope

- **Configuration:** parse `[metrics.bucket_weights]` and `[metrics.category_weights]` in `crates/deslop-core/src/config.rs`; default every omitted key from [METRICS-REPO-WEIGHTED](../specs/pipeline.md#metrics-repo-weighted), accept finite values in `[0,1]`, and reject invalid values with exit `2` naming the full path. Carry the resolved table in a dedicated `MetricWeights`, separate from `RankingPolicy`.
- **One Rust projection:** replace the existing metrics fold in `crates/deslop-core/src/report_metrics.rs` with one projection that records the mechanical line union and each line’s maximum effective weight. Derive repository, file, and folder numerators from it; every percentage uses the core Rust percentage function.
- **Canonical wire model:** replace the metrics shapes in `docs/models/live-ipc.td` with the complete final model, including `WeightedMetrics { duplicated_loc, duplication_percent, threshold, bucket_weights, category_weights }` and weighted `FileMetric` fields, then regenerate every Rust and TypeScript consumer. Never patch generated files or carry old and new shapes together.
- **CLI gate:** add `--fail-over-weighted` and replace threshold resolution with the complete two-gate result. Either strictly-greater breach exits `3`, equality passes, and `--no-fail-over` disables both gates.
- **All renderers and clients:** update text, HTML, JSON, VSIX metrics surfaces, per-file rows, and folder rows in the same change. They consume Rust-authored fields verbatim; no TypeScript calculation or legacy field fallback remains.
- **All documentation and tests:** replace obsolete fixtures and descriptions with the final two-metric contract in the same cutover. Update `REPORTING-CONTEXT.md`, regenerate `schema_doc`, and update the accuracy-transparency site page before the cutover is considered complete.

## Acceptance tests

- **Mixed evidence:** one `identical` cluster, one `structural_only` cluster, and one `data` cluster; assert exact cluster IDs, buckets, categories, occurrence counts, paths, line sets, both numerators, both percentages, and the echoed resolved weights.
- **All-one weights:** weighted and mechanical repository, file, and folder values are identical at full `f64` precision.
- **Mechanical invariance:** adding or changing `[metrics]` cannot alter any mechanical field or cluster; setting `structural_only = 0` removes exactly that family’s unique lines from the weighted numerator.
- **Overlapping coverage:** a line covered by `identical` and `structural_only` clusters weighs `1.0`, not their sum and not their average.
- **Zero denominator:** both percentages are `0` when `analysed_loc = 0`.
- **Gate matrix:** mechanical-only breach, weighted-only breach, both, and neither produce `3/3/3/0`; equality passes; `--no-fail-over` suppresses both.
- **Invalid configuration:** `NaN`, infinity, negative values, and values above `1` fail with exit `2` and the offending path.
- **Determinism:** repeated runs produce byte-identical weighted fields and weight tables.
- **gh #355:** assert its current routed bucket and exact weighted contribution; do not use weighting as the routing fix.

## Completion

The plan is complete when all acceptance tests pass through rendered reports, every surface shows the same engine-authored figures, the mechanical report is byte-identical without weighted configuration, generated models are current, and the coverage thresholds have ratcheted without exclusions.

Trackers: gh #344 owns delivery; gh #355 is the measured inflation case; gh #336 supplies the data-category precedent; gh #345 owns the documentation drift sweep; gh #347 adds corpus baselines for both percentages.
