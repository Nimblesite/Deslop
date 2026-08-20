# Calculation Violations Audit

Re-audited against commit `e466c656dcdd` on 2026-08-20. The previous audit baseline was `8751e8bfbf35`.

## Result

No previously open violation has been fixed. The current codebase still performs report-derived and percentage calculations in UI clients, in direct conflict with the rule in `CLAUDE.md` that calculations must exist only in core Rust.

The two commits since the previous audit added proposed Rust/wire replacements for several client-derived values, but made no production changes under `clients/vscode` or `clients/jetbrains`. Consequently, every client calculation listed in the previous audit is still present and still used.

The attempted Rust-side remediation also does not compile. `cargo check --workspace` fails with:

- `crates/deslop-core/src/report_render.rs:205`: nonexistent `crate::render::signals::content_evidence_verdict`.
- `crates/deslop-core/src/live/embedding_refresh.rs:100`: missing `percent` in an `EmbeddingProgress` initializer.
- `crates/deslop-core/src/live/session_helpers.rs:53`: missing `percent` in an `EmbeddingProgress` initializer.

The duplicate folder-duplication percentage engine that existed in the VSIX has been removed in the current commit: folder totals and percentages now come from `RepoMetrics.folders`, computed by the canonical Rust `percent()` function in `crates/deslop-core/src/report_metrics.rs`. That specific violation is fixed.

The remaining violations are listed below. Tests, fixtures, build tooling, and CSS declarations are outside this audit; the scope is production VSIX, webview, website JavaScript, and JetBrains UI code.

## Re-audit status

| Finding | Status at `e466c656dcdd` | Evidence |
|---|---|---|
| CALC-001 severity percentile | **OPEN** | Rust wire fields were added, but rank fields are not stamped and both TypeScript formulas remain. |
| CALC-002 signal semantics | **OPEN** | Rust wire fields were added, but the Rust verdict call does not compile and all TypeScript derivations remain. |
| CALC-003 VSIX impact aggregation | **OPEN** | No VSIX production code changed; rank is still derived locally. |
| CALC-004 JetBrains aggregation | **OPEN** | No JetBrains production code changed. |
| CALC-005 client count/metric rewriting | **OPEN** | `occurrence_count` was added and stamped in Rust, but the VSIX still recomputes it and still rewrites projected metrics. |
| CALC-006 progress percentage | **OPEN / BUILD-BREAKING** | The wire field was added, its Rust producers omit it, and the VSIX still computes the percentage. |
| CALC-007 path/language/location derivation | **OPEN** | `ReportCluster.language` was added in Rust, but clients still derive language and paths locally. |
| Folder duplication percentage | **FIXED** | The prior Rust `RepoMetrics.folders` fix remains intact. |

The new wire fields are groundwork, not completed fixes. A violation is only fixed when the engine produces the value, the wire carries it, every client consumes it, and the old client calculation is deleted.

## Hard domain-calculation violations

### CALC-001: Severity percentile is calculated twice in TypeScript

**Status: OPEN.** `ReportCluster.rank` and `rank_band` were added to the wire, but `cluster_to_report` initializes them as `0` and an empty string. No rank-stamping implementation exists in `report_weight.rs`, and both TypeScript percentile engines below remain unchanged.

The same rank-percentile formula is independently implemented in two UI runtimes:

- `clients/vscode/src/severity.ts:66-69` calculates `1 - (rank - 1) / (total - 1)`.
- `clients/vscode/webview-ui/src/store.ts:49-55` repeats that formula inline.
- `clients/vscode/src/types/report.ts:124-129` then classifies the locally calculated percentage into `worst`, `top10`, `mid`, or `faint` using UI-owned thresholds.

This is both a calculation outside Rust and a duplicate percentage calculation engine. The extension host and webview can drift independently, and neither value is supplied by the engine.

### CALC-002: Signal semantics are recalculated and classified in the UI

**Status: OPEN and currently build-breaking.** `ReportSignals.shape`, `ReportCluster.meets_fused_gate`, and `ReportCluster.evidence_verdict` were added to the wire. The clients do not consume them and retain every calculation below. In addition, Rust attempts to populate `evidence_verdict` by calling a function that does not exist, so the workspace does not compile.

- `clients/vscode/src/types/signals.ts:115-116` calculates a shape score with `max(structural, token_jaccard)`.
- `clients/vscode/src/bubble/renderParts.ts:73-74` independently repeats the same shape-score calculation.
- `clients/vscode/src/types/signals.ts:121-141` applies a UI-owned epsilon and compares shape, embedding, fused confidence, and `FUSED_THRESHOLD` to manufacture a semantic verdict.
- `clients/vscode/src/bubble/live.ts:377-405` locally ranks candidates and applies another UI-owned admission rule based on bucket or fused confidence.
- `clients/vscode/src/bubble/renderParts.ts:80-92` quantizes signal values into glyph strength.
- `clients/vscode/webview-ui/src/components/SignalStrip.tsx:61-63` clamps a signal, multiplies it by 100, and rounds it for bar width.

These are report interpretations and classifications, not merely verbatim rendering of engine output.

### CALC-003: Cluster, file, folder, and language impact is aggregated in the VSIX

**Status: OPEN.** No VSIX production source changed after the previous audit. The new wire `rank` field is initialized to zero rather than stamped after sorting, and the VSIX still derives rank and aggregate weights locally.

- `clients/vscode/src/tree/grouping.ts:50-56` derives global ranks from array position.
- `clients/vscode/src/tree/grouping.ts:76-80`, `114-119`, and `200-205` locally order clusters and groups by weight or byte offset.
- `clients/vscode/src/tree/grouping.ts:132-141` calculates per-file maximum and summed cluster weights.
- `clients/vscode/src/tree/folder.ts:57-66` recursively recalculates maximum and summed descendant weights for folders.
- `clients/vscode/src/tree/providers.ts:300-313` calculates the maximum weight for language groups.
- `clients/vscode/src/tree/sort.ts:33-40` owns the max-weight/sum-weight ranking formula.
- `clients/vscode/src/tree/pathTree.ts:45-46` recursively calculates descendant file counts.
- `clients/vscode/src/tree/rollup.ts:57` re-sorts engine-produced folder/file percentages in TypeScript.

The values and ordering shown as “worst offenders” are therefore not entirely engine-owned.

### CALC-004: The JetBrains UI independently aggregates and ranks report data

**Status: OPEN.** No JetBrains production source changed after the previous audit.

- `clients/jetbrains/deslop-shared/src/main/kotlin/com/nimblesite/deslop/jetbrains/DeslopOffenderGrouping.kt:123-140` groups clusters, sums member weights, counts members, and sorts groups and clusters by calculated impact.
- `clients/jetbrains/deslop-shared/src/main/kotlin/com/nimblesite/deslop/jetbrains/DeslopOffenderGrouping.kt:25-38` derives language and folder groups from occurrence paths.

This is a second non-Rust offender-ranking/grouping engine and overlaps conceptually with the VSIX calculations in CALC-003.

### CALC-005: The VSIX rewrites report counts and metrics

**Status: OPEN.** Rust now stamps a new `occurrence_count` field, but the VSIX still calls its existing `occurrenceCount()` and `occurrenceTotal()` calculations. The dirty-file projection and local metric rewriting also remain unchanged.

- `clients/vscode/src/types/report.ts:56-61` calculates an occurrence count using the maximum of wire totals, cluster size, and visible occurrence length.
- `clients/vscode/src/reportStore.ts:345-365` removes dirty occurrences, recalculates cluster size, and replaces `metrics.clusters_total` in a client-side projected report.
- `clients/vscode/src/reportStore.ts:384-390` independently repeats the occurrence-total calculation.
- `clients/vscode/src/reportStore.ts:229-238` merges report deltas and locally restores weight order.

The UI is not rendering the report verbatim; it derives replacement report facts. The duplicated occurrence-total logic can also drift between general rendering and dirty-file projection.

### CALC-006: Embedding progress percentage is calculated in the VSIX

**Status: OPEN and currently build-breaking.** `EmbeddingProgress.percent` was added to the wire, but both Rust producers fail to initialize it. The existing TypeScript formula remains active and does not read the new field.

`clients/vscode/src/tree/providers.ts:421-435` calculates `floor(done / total * 100)` from progress counts. This is an explicit percentage engine outside Rust.

### CALC-007: Paths, languages, and editor locations are derived in UI clients

**Status: OPEN.** Rust now populates `ReportCluster.language`, but the VSIX and JetBrains grouping code still derive language from file extensions. All path and editor-location calculations below remain unchanged.

- `clients/vscode/src/locations.ts:77-82` clamps byte offsets and calculates one-based line/column display coordinates.
- `clients/vscode/src/types/languages.ts:35-36` derives language from a path extension.
- `clients/vscode/src/tree/language.ts:20-43` groups clusters by that derived language.
- `clients/vscode/src/pathUtils.ts:14-16`, `clients/vscode/src/tree/paths.ts:21-25`, and `clients/vscode/src/tree/pathTree.ts:49-50` calculate path segments and tree structure.
- `clients/jetbrains/deslop-shared/src/main/kotlin/com/nimblesite/deslop/jetbrains/DeslopOffenderGrouping.kt:177-195` independently derives folder, base-name, and extension values from paths.
- `clients/jetbrains/deslop-shared/src/main/kotlin/com/nimblesite/deslop/jetbrains/DeslopOffendersTreePanel.kt:141-145` converts the engine line to a zero-based editor line.

These are presentation adapters, but they still violate the literal “ZERO calculations outside core Rust” rule.

## Percentage and numeric transformations in presentation code

**Status: OPEN.** None of these production UI files changed after the previous audit.

The following calculations do not change the underlying engine metric, but they round, scale, clamp, or format numbers in UI code and are therefore violations under the literal rule:

- `clients/vscode/src/commands/statusBar.ts:72`: rounds duplication percentage to one decimal.
- `clients/vscode/src/tree/nodes.ts:277`: rounds metric percentage to one decimal.
- `clients/vscode/src/tree/threshold.ts:33`: rounds threshold percentage to one decimal.
- `clients/vscode/webview-ui/src/components/MetricHeading.tsx:19`: rounds a percentage to one decimal.
- `clients/vscode/webview-ui/src/duplication/main.tsx:72`: rounds folder/file percentage to one decimal.
- `clients/vscode/src/types/signals.ts:68-70`: rounds signal values to two decimals.
- `clients/vscode/src/clusterDocument.ts:74-77`, `clients/vscode/src/commands/treeMenus.ts:156,214,229`, `clients/vscode/src/tree/nodes.ts:112,167,172,240,245,265`, `clients/vscode/webview-ui/src/cluster/main.tsx:198,459`, and `clients/vscode/webview-ui/src/report/main.tsx:232`: locally round weights or signals.
- `clients/vscode/src/tree/providers.ts:368-369,428-429`: locally formats counts.

If Rust is intended to own even display precision, the wire needs engine-rendered display values; full-precision numeric wire values necessarily require the client to format them.

## UI/control calculations caught by the literal rule

**Status: OPEN.** None of these production UI files changed after the previous audit.

These are not duplication-domain engines, but the current wording makes them hard violations too:

- Rank and occurrence numbering: `clients/vscode/src/clusterDocument.ts:88`, `clients/vscode/src/commands/register.ts:157`, `clients/vscode/src/commands/treeMenus.ts:227-232`, `clients/vscode/src/tree/nodes.ts:142`, and `clients/vscode/webview-ui/src/cluster/main.tsx:35,49,467`.
- Circular navigation and index clamping: `clients/vscode/src/commands/register.ts:276`, `clients/vscode/webview-ui/src/cluster/main.tsx:419-438`, and `clients/jetbrains/deslop-shared/src/main/kotlin/com/nimblesite/deslop/jetbrains/DeslopOffendersTreePanel.kt:83-89`.
- Byte/range clamping and edit offsets: `clients/vscode/src/commands/register.ts:519`, `clients/vscode/src/commands/treeMenus.ts:266`, and `clients/vscode/src/bubble/live.ts:188,464`.
- State counters and bounded caches: `clients/vscode/src/bubble/live.ts:176,236`, `clients/vscode/src/notifications.ts:83,138`, and `clients/vscode/src/reportStore.ts:193,222,265,318-319`.
- Spinner state: `clients/vscode/src/tree/providers.ts:93,111-121`.
- Layout and alternating rows: `clients/vscode/webview-ui/src/duplication/main.tsx:41,77`, `clients/vscode/webview-ui/src/cluster/main.tsx:239`, and `clients/vscode/webview-ui/src/report/main.tsx:176,185`.
- Process-priority clamping: `clients/vscode/src/extension.ts:399`.
- String/path indexing and sentinel arithmetic: `clients/vscode/src/binary.ts:292`, `clients/vscode/src/clusterSelection.ts:37-47`, `clients/vscode/src/pathUtils.ts:15-16`, and `clients/vscode/src/tree/paths.ts:25`.

The rule needs an explicit scope if these ordinary client mechanics are meant to be allowed. As written, “ZERO calculations outside core Rust” prohibits them.

## Confirmed fixed in this round

Before commit `8751e8bfbf35`, `clients/vscode/src/tree/rollup.ts` summed folder LOC and calculated `(duplicatedLoc / analysedLoc) * 100` in TypeScript. Both the sidebar and duplication webview consumed that duplicate engine.

The current code instead:

- computes repo, file, diff, and folder percentages through `percent()` in `crates/deslop-core/src/report_metrics.rs:407-425`;
- computes folder rows in `crates/deslop-core/src/report_metrics.rs:176-210`;
- carries those rows on `RepoMetrics.folders` in `docs/models/live-ipc.td:143-154`; and
- only nests the transmitted rows in `clients/vscode/src/tree/rollup.ts`.

The current pipeline specification and wire model agree on this design. No current duplicate folder-percentage formula remains in the VSIX.

## Other UI surfaces

No arithmetic was found in production JavaScript under `site/src` or in `site/eleventy.config.js`. The JetBrains violations are CALC-004 and the path/editor conversions in CALC-007.

## Verification performed for this update

- Compared `8751e8bfbf35..e466c656dcdd`: no production VSIX, webview, or JetBrains source changed.
- Re-read every previously identified calculation site; the formulas remain present.
- Inspected the new wire and Rust fields to determine whether they are populated and consumed.
- Ran `cargo check --workspace`; it failed with the three compilation errors recorded above.

## Bottom line

The folder percentage duplication remains fixed, but **none of CALC-001 through CALC-007 is fixed**. Severity percentile, signal interpretation, impact aggregation/ranking, occurrence/count projection, progress percentage, and path/language/location derivation are still calculated outside core Rust. Severity percentile and shape-score calculation still have duplicate TypeScript implementations. The new engine fields are not yet migrated into the clients, rank fields are not stamped, progress producers omit their new field, and the current Rust workspace does not compile.
