# Remove evidence-weighted metrics — wholesale deletion plan

[METRICS-REPO-WEIGHTED](../specs/pipeline.md#metrics-repo-weighted-evidence-weighting-is-prohibited) and [EXIT-CODES-WEIGHTED](../specs/pipeline.md#exit-codes-weighted-evidence-weighted-gates-are-prohibited) prohibit evidence-weighted repository figures and gates. Pair evidence cannot be projected onto a closure component or a covered line. This plan deletes the design and every partial implementation in one cutover; it does not preserve a dormant configuration or compatibility wire field.

## Contract

- Use the one repository percentage, formula and eligibility rules in [METRICS-REPO](../specs/pipeline.md#metrics-repo-repo-wide-duplication-metrics).
- Each duplicated line counts once. Pair evidence, pair classification, finding kind, confidence, and severity do not scale it.
- Shape-only is a non-clone and contributes nothing, even when visible or assigned a diagnostic severity. Preserve true clone subsets in mixed groups; count only eligible clone coverage ([CLONE-BUCKETS-STRUCTURAL-ONLY](../specs/taxonomy.md#clone-buckets-structural-only-shape-only-is-not-duplication)). Exclusion of non-clones is required, not evidence weighting.
- The repository has one threshold gate: `--fail-over` or `[threshold] max_duplication_percent`. No weighted gate exists.
- Cluster mass is [RANK-MASS-SUM](../specs/pipeline.md#rank-mass-sum-rank-by-duplicated-mass-only), not a metric percentage and not an input to `duplication_percent`.

## One destructive removal

- [x] Remove the weighted metric, weighted gate, weight tables, and multipliers from the governing specifications.
- [x] Delete `WeightedMetrics`, weighted file/folder fields, resolved weight-table fields, and weighted threshold fields from `docs/models/live-ipc.td`; regenerate every Rust and TypeScript model.
- [x] Delete `[metrics.bucket_weights]`, `[metrics.category_weights]`, validation, defaults, CLI overrides, environment plumbing, and serialization.
- [x] Delete `--fail-over-weighted`, `max_weighted_duplication_percent`, two-gate resolution, weighted verdicts, and weighted exit-code branches.
- [x] Delete weighted numerator calculation, per-line evidence lookup, bucket/category weight lookup, weighted folder rollup, and every helper used only by them.
- [x] Delete weighted text, HTML, JSON, LSP, MCP, VSIX, site, and documentation rendering. Do not leave hidden fields, empty placeholders, deprecated aliases, or no-op switches.
- [ ] Replace tests that assert weighted output with strict negative schema tests and positive mechanical-metric tests; never remove the underlying coverage assertions.

## Proof

- [ ] Black-box CLI tests assert exact `analysed_loc`, `duplicated_loc`, `duplication_percent`, per-file values, folder values, threshold source, breach flag, exit code, and rendered text/HTML.
- [x] Configuration tests reject every retired weighted key and flag with a named invalid-configuration or invalid-argument error (`metrics_weighted_wire_refusal`: four TOML forms and two CLI flags).
- [ ] Generated-model tests prove no weighted field or table exists in Rust or TypeScript.
- [ ] Tests prove changing pair evidence without changing clone eligibility or eligible occurrence coverage cannot change repository metrics; showing or hiding shape-only information and overriding its diagnostic severity also leave metrics unchanged.
- [ ] CLI fixtures pin shape-only-only zero duplication and mixed-corpus true-clone coverage, including exact file/folder/diff percentages, clone counts, paths, ordering and threshold verdicts ([CLONE-KIND-TESTING](../specs/taxonomy.md#clone-kind-testing-required-examples-and-assertions), [METRICS-REPO](../specs/pipeline.md#metrics-repo-repo-wide-duplication-metrics)).
- [ ] Repository-wide searches find no executable weighted-metric type, field, flag, config key, renderer, or compatibility branch.
- [ ] Full CI and installed VSIX verification pass with the one mechanical metric on every surface.

## Completion

This plan is complete when the weighted design is absent from the wire, engine, configuration, CLI, renderers, clients, tests, and installed VSIX, and the one unweighted repository metric is identical on every surface.

## Current verification

The weighted types, calculations, flags and renderers have been removed. `make _ci-analyze` regenerated unchanged wire models at commit `9541eb6c`; the CLI refusal and rendered-surface tests ran in the passing workspace suite recorded in `reports/tests/rust-coverage-baseline.md`. A new black-box regression reproduced silent acceptance of retired TOML fields. Root and threshold tables now reject unknown fields; all three `metrics_weighted_wire_refusal` tests pass, including four retired TOML forms and two CLI flags. Installed VSIX verification and the remaining proof items are still required.

## Legacy compatibility removal — replacement strategy

Legacy code is contamination (AGENTS.md). Every path that existed only to accept an older report, wire, flag, setting or record was hard-deleted on `fix/post-release-review` (`ca176d6e`, `6dc69fd5` and the seed-window fallback commit). The tree is deliberately broken: callers and tests that relied on the deleted paths do not compile or fail. The replacement is **a single current contract with no compatibility branch**: every field the engine writes is required on read, and an input that does not match fails with a named error. Nothing deleted comes back under another name.

### What was deleted, and what replaces it

| Deleted | Replacement |
|---|---|
| Absent `total_includes` handling in the VSIX (`kindCountsTowardTotal`, `decoratableClusters`, `getGroupNodeChildren`, bubble filters, `totalCountsCaption`) | The include set is a required parameter throughout. The "no report loaded" case ([providers.ts:260](../../clients/vscode/src/tree/providers.ts), `webview-ui/src/report/main.tsx:60`) renders nothing instead of passing `undefined` |
| Serde defaults for `total_includes`, `per_kind`, `per_file`, `folders`, `files_skipped`, `routing`, `succeeded_subtrees` | Required fields. `--from-report` refuses a report missing any of them, with one named usage error (exit `2`) that names the missing field and says to re-scan |
| `migrate_legacy_embedding_coverage`, `validate_kind_measurements` | Nothing: the parse refusal above covers both |
| LSP `LEGACY_STARTUP_FLAGS` / `RETIRED_RANKING_FLAG` | The general `unsupported LSP startup flag` error for every unknown flag |
| Recognition of the `[ranking]` section (`reject_retired_ranking`) | `deny_unknown_fields` rejects it like any other unknown section |
| Retired MCP tool names in `KNOWN_NON_CLI_TOOL_NAMES` | The lookalike guard names only the tools the server registers today |
| Corpus scorer reading `bucket` | The scorer reads `kind` only. A report without `kind` fails the scoring run with a named error instead of scoring as "no clone" |
| Path-based parser fallback (`parser_for_occurrence`, `languages_from_report_occurrences`) | `language` becomes required on every `ReportOccurrence` (it stays optional on folder rows) |
| `ReportStore.onDidChange` shim | Every caller subscribes through `effect()` |
| `BackendError::StateFileCorrupt` (name kept for old releases) | A correctly named IPC transport variant; every one of the 23 sites moves to it |
| `ReportChangedNotification.analysed_buffer` | The server stops computing it (`backend_changes.rs`, `scheduler.rs`); acknowledgements ride only on `analysed_buffers` in report answers |
| Corpus record defaults for `load_at_start`, `logical_cores` | Required fields. Records missing them are regenerated by a fresh corpus run, never defaulted |

### Spec changes required (not written here)

- `pipeline.md`: `total_includes` (drop "reports predating the field default to all clone kinds"), `folders` (drop the `#[serde(default)]` sentence), the replay rule at "A saved report with clones but no per-kind measurements…" (replace it with the missing-field refusal), and `embedding_provenance` (`succeeded_subtrees` becomes required).
- `REPORTING-CONTEXT.md`: `language` is required on occurrences, and absent only on folders.
- `corpus.md`: delete the `bucket` paragraphs (the "older engine's name for `kind`" text and the [CORPUS-SCORE] sentence about reading both). State that a report without `kind` fails the run. Add that measurement records must carry `load_at_start` and `logical_cores`.
- `exclusion.md` [EXCLUSION-CONFIG]: `[ranking]` and `--ranking-structural-only` are unknown input with no special message. Delete "the language server refuses its former flag".
- `cli.md` [CLI-SUBCOMMAND-LOOKALIKE]: the guard lists only the registered tools. `--from-report` names the missing-field refusal.
- `vsix.md` [VSIX-STATE-DIRTY]: delete "`deslop/reportChanged` carries `analysed_buffer`". Acknowledgement rides only on report answers.
- `live.md` [LIVE-OPEN-BUFFERS]: the notification carries generation and summary only.
- `mcp.md`: name the IPC transport error variant.
- `docs/models/live-ipc.td`: `ReportOccurrence.language` becomes required, and its Rust/TypeScript models are regenerated.

### Steps

- [x] Hard-delete every legacy compatibility path found by the sweep, then commit and push in the broken state.
- [x] Record the rule in AGENTS.md: legacy code is contamination; delete it on sight and report it.
- [x] Make the spec changes listed above, and cross-reference the spec IDs in code and tests.
- [x] `docs/models/live-ipc.td`: make `ReportOccurrence.language` required. Regenerate both wire models, and confirm `analysed_buffer` and the deleted serde defaults are gone from the generated code.
- [x] Engine: make `--from-report` refuse a report missing a required field, with one named usage error, and add the black-box CLI test for each field (`total_includes`, `per_kind`, `per_file`, `folders`, `files_skipped`, `routing`, `succeeded_subtrees`).
- [x] Convert `replay_preserves_all_clone_kinds_when_the_original_include_field_is_absent` and `replay_requires_kind_measurements_when_kind_limits_are_configured` to assert the refusal. Keep every assertion's strength and delete none of the tests.
- [x] Update replay fixtures and hand-built report JSON in tests to carry every required field.
- [x] LSP: switch `deslop-lsp/tests/app.rs` retired-flag assertions to the general unsupported-flag error.
- [x] Config: switch the `[ranking]` tests (`rank_structural_only_policy.rs`, `issue_190_data_table_demote.rs`) to assert the unknown-field refusal.
- [x] CLI: switch the lookalike-guard tests that named retired tools to assert that those words are treated as plain paths.
- [x] Corpus scorer: fail the run with a named error on a report without `kind`. Test it, then regenerate any committed corpus record missing `load_at_start` / `logical_cores`.
- [x] Parser identity: populate `language` on every occurrence the engine writes, then switch `parser_for_occurrence` and `languages_from_report_occurrences` callers to the required field.
- [x] MCP: add the correctly named IPC transport variant, move all 23 `StateFileCorrupt` sites, and assert the rendered error text in an MCP test.
- [x] LSP: stop computing `analysed_buffer` for notifications (`backend_changes.rs`, `scheduler.rs`), and keep the `analysed_versions` E2E test green.
- [x] VSIX: move `extension.ts` and every test subscriber from `onDidChange` to `effect()`.
- [x] VSIX: handle the no-report case at `providers.ts:260` and `main.tsx:60` without `undefined`. Convert the `legacyMetrics` and no-include-set tests to assert that a report without `total_includes` is refused at parse.
- [x] Re-run the sweep (`legacy`, `older`, `predat`, `retired`, `compat`, `formerly`, `serde(default)` on required fields) and get zero production hits.
- [ ] `make ci` green, coverage at or above `coverage-thresholds.json`, and the corpus comparison (`scripts/compare-versions.sh v0.42.0 HEAD`) shows no accuracy regression.
