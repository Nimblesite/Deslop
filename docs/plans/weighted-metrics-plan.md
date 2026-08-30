# Evidence-weighted duplication metric — plan

Implements [pipeline.md §METRICS-REPO-WEIGHTED](../specs/pipeline.md#metrics-repo-weighted) and [§EXIT-CODES-WEIGHTED](../specs/pipeline.md#exit-codes-weighted); config surface in [exclusion.md](../specs/exclusion.md). The spec is normative — this plan is sequencing, touch-list, and test contract only.

**Problem.** `metrics.duplication_percent` counts every visible line at equal weight, so a `structural_only` family — evidence the tool itself labels "unverified" — moves the CI gate exactly like byte-proven copy-paste. This is the open metrics row of gh **#344** (Gap 3 of the fused rollout), and gh **#355** is a measured instance: a Dart delegating-method family alone producing `duplication_percent = 13.71`.

## Decisions (settled — in the spec, do not reopen)

Two metrics on the wire ([METRICS-REPO-WEIGHTED]): the mechanical percentage untouched and still the default gate; the weighted companion priced by bucket × category constants from the spec's default table, per-line max-weight-wins (invariant: weighted ≤ mechanical), never confidence-scaled ([RANK-MASS-SUM] is the same rule), two independent gates with one kill switch ([EXIT-CODES-WEIGHTED]). Weighting prices honest labels; misrouted clusters stay separate accuracy bugs.

## Work items, in order

1. **Config** — `crates/deslop-core/src/config.rs`: parse `[metrics.bucket_weights]` / `[metrics.category_weights]`, validate finite `[0.0, 1.0]` (`ConfigThreshold`-style error naming the path; `0.0` legal, unlike `[ranking]`), resolve into a `MetricWeights` carried beside `RankingPolicy`.
2. **Wire model** — `docs/models/live-ipc.td`: `WeightedMetrics { duplicated_loc: Float, duplication_percent: Float, threshold: ThresholdSummary, bucket_weights, category_weights }` on `RepoMetrics`; `weighted_duplicated_loc` / `weighted_duplication_percent` on `FileMetric`. Regenerate; never hand-edit generated code.
3. **Computation** — `crates/deslop-core/src/report_metrics.rs`: in the existing `fold_cluster_lines` projection, carry each line's max effective weight alongside the `BTreeSet<u64>` union (same visible set, same hidden/literal-family exclusions — one projection, two aggregations, so the metrics cannot drift apart).
4. **Gate** — `ThresholdSummary::resolve` reused for the weighted ceiling; `render_report` fills `metrics.weighted.threshold`; `crates/deslop/src/main.rs` adds `--fail-over-weighted`, extends `--no-fail-over` to both, maps either breach to exit `3`.
5. **Renderers** — text/HTML header carries both figures per [METRICS-REPO]; JSON canonical. VSIX Duplication panel headline ([vsix.md §VSIX-METRICS-PANEL](../specs/vsix.md#vsix-metrics-panel)) and the metrics webview show the weighted figure beside the mechanical one; per-folder rollups sum weighted numerators.
6. **Docs shipped to agents** — `REPORTING-CONTEXT.md` (`schema_doc`) gains the weighted fields and gate; fold into the #345 drift sweep. Update `site/src/docs/accuracy-transparency.md` from "specified, tracked in #344" to the shipped formula.

## Test contract (write first; watch each fail)

Coarse E2E over fixture repos, asserting rendered reports — never internals:

- **Mixed-evidence fixture**: one verbatim cross-file clone + one cross-file `structural_only` sibling family + one data table. Assert cluster set, buckets, occurrence counts and paths; assert exact `duplicated_loc`, `duplication_percent`, `weighted_duplicated_loc`, `weighted_duplication_percent` (hand-computed from the fixture's line counts and the default table); assert weighted < mechanical, and the echoed weight table on the wire.
- **All-identical fixture**: weighted == mechanical at full `f64` precision.
- **No-`[metrics]`-section invariance**: mechanical fields byte-identical across runs with and without a `[metrics]` section; with `structural_only = 0.0` the weighted numerator drops by exactly the family's line count and mechanical is unchanged.
- **Overlap fixture**: a line covered by both an `identical` and a `structural_only` cluster counts `1.0` weighted — max, not `1.15`.
- **Gate matrix**: mechanical-only breach, weighted-only breach, both, neither → exit codes `3/3/3/0`; equality passes both; `--no-fail-over` suppresses both; invalid weight → exit `2` naming the path.
- **Config rejection**: `NaN`, `-0.1`, `1.5` each rejected.
- **#355 fixture**: once its family is correctly hidden, both numerators exclude it; until then the weighted figure prices it at `0.15` — assert the current exact values so any drift is loud.
- **Determinism**: two runs, identical weighted figures (extends the #301 corpus checks).

Coverage thresholds ratchet in `coverage-thresholds.json` as usual.

## Related issues

- **#344** — primary tracker (metrics/gate row of the confidence rollout). This plan closes that row only; the other #344 surfaces stay in [fused-score-followups.md](fused-score-followups.md).
- **#343** — fixed prerequisite: bucket labels now sit on bounded, content-gated evidence, which is what makes bucket-keyed weights meaningful.
- **#355** — measured inflation instance; mitigated at 0.15 by this plan, actually fixed by its own routing repair.
- **#336** — data-table precedent; `category_weights.data` default mirrors its ranking outcome.
- **#345** — doc-drift sweep; item 6 rides with it (`schema_doc`, site page).
- **#283/#284/#285** — routing promotions that weighting deliberately does not paper over (decision 6).
- **#347** — the corpus gate should record both figures per repo once it runs, so the weighted metric gets real-repository baselines from day one.
