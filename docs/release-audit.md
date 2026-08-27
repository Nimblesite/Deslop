# Release audit — what blocks the release

Scoped to **regressions since `f92300e` (v0.32.0)**: behaviour that is worse at HEAD than in the shipped release, plus anything that stops the gate running at all. Measured on the `accuracy-ordered-overlap-bound` branch at HEAD (`5a0999e`) with `target/release/deslop`.

Everything else that was on this page — the fusion-arithmetic departures, the parked engine defects, the gate and coverage work, the documentation drift — is not a regression and now lives in [`plans/fused-score-followups.md`](plans/fused-score-followups.md). It is not restated here.

**Verdict: not ready. One blocker remains — the #434 noise families (§ 2). The gate compile (§ 1) is fixed and verified.**

## 1. The gate does not compile — introduced on this branch (fixed)

`crates/deslop-core/Cargo.toml` declares two bench targets, `cluster_signals` and `shared_subtree_alignment`, without `required-features = ["benchmark"]`. Both sources import `cluster::benchmark` / `overlap::benchmark`, which are `#[cfg(feature = "benchmark")]`. The gate runs `--all-targets --features deslop-core/live,deslop-lsp/profiling`, and `deslop-lsp/profiling` pulls in `fxprof-processed-profile` and `pprof` only — it does not enable `deslop-core/benchmark`. So cargo builds the benches without the feature and dies.

Reproduced directly: `cargo check -p deslop-core --benches` fails `E0432`, "could not find `benchmark` in `cluster`" and "in `overlap`", with rustc naming both gated modules as configured out. `make test` and `make lint` both fail to compile at HEAD.

**Fix — applied by `release-gate` over TMC, verified here:** `required-features = ["benchmark"]` now sits on both `[[bench]]` sections. `scripts/benchmarks/cluster-signals.mjs` and `shared-subtree-alignment.mjs` already pass `--features benchmark` themselves, so nothing else moved. The exact gate command now compiles every target clean, and the full suite with the fix runs 1210 passed, 0 failed — this was the branch's only self-inflicted damage.

That count excludes the 28 rows of `CURATED_SKIPS` in `crates/deslop/tests/skip_policy_contract.rs`. Twelve of them are accuracy tests that are red for real reasons under `-- --ignored`: `operator_drift_is_not_duplication` ×2 and `report_golden` (#432), `incremental_multilang_golden` ×3 and `lsh_only_nearmiss_recall` (#433), the four #434 pins below, and `lsp_embedding_determinism` / `issue_343_sum_clamp_saturation` (#369). Read "1210 passed" next to that number, never on its own.

## 2. Four Python noise families publish that v0.32.0 suppressed — #434

Re-measured at HEAD with the release CLI, at each pin's own `--min-nodes`, on the checked-in fixture bytes:

| fixture | what publishes | dup% | `duplicated_loc` | hidden |
|---|---|---|---|---|
| `python-issue-71` | one `nearly_identical` family ×4, ranked **#1** above the real clone staged in the same run (weight 108 vs 44) | 70.18 | 40 | 0 |
| `python-issue-70` | one `structural_only` family ×4 | 57.14 | 28 | 0 |
| `python-issue-72` | one `structural_only` trio ×3, all intra-file in `test_fly_host.py`, `hidden=false` — its 15 lines are counted in `duplicated_loc` (31 of 46) | 67.39 | 31 | 3 |
| `python-issue-107` | three `identical` clusters, one same-file pair per file | 45.83 | 22 | 4 |

Each fixture also stages a real 16-line clone as a control. Every row counts the noise family on top of it, so `--fail-over` trips on scaffolding.

### All four are regressions, not two

The previous revision of this page recorded #70 and #71 as new pins on previously-untested behaviour. That is wrong, and the correction matters because it doubles the blocking surface. Measured against `f92300e`:

- **The fixture bytes are unchanged.** `test_write_file_calls.py` (#70), `test_endpoints.py` (#71) and `test_fly_host.py` (#72) all hash identically to their v0.32.0 contents. What this branch added to those directories is the `control_clone_a.py` / `control_clone_b.py` pair and the test — not the noise family. The input the detector sees is the same input v0.32.0 saw.
- **Both root causes are code that did not exist in v0.32.0.** `cluster_filters/verbatim_subgroup.rs` is absent at `f92300e`. So is `every_covered_statement_has_call` in `cluster_filters/calls.rs` — the file existed, that precondition did not.

So the same bytes now take a code path that v0.32.0 had no way to take, in both halves of the defect. For #72 and #107 this is confirmed from the other direction: their tests were green at `f92300e`, unskipped, asserting zero clusters. For #70 and #71 it is an inference from unchanged input plus new blocking code — running the v0.32.0 binary would need a checkout, which the repository rules forbid, so nobody has measured it directly. Treat them as regressions and let a measurement demote them, not the other way round.

### Two defects, not one

- **#70 / #71 — the filter never fires** (`hidden == 0`). `[CLONE-NOISE-LITERAL-VARIATION-CALLS]` should suppress these. `every_covered_statement_has_call` returns false first, because each body ends in a call-free `assert`. Fixing it is a spec arbitration — an assertion on the value the varying call returned is part of the idiom, an authored computation is not, and the filter cannot currently tell them apart. It is not a threshold to loosen. Note the constraint: `rename_needs_an_anchor` pins the precondition that makes it return false, so this fix can turn a green test red and has to move both together.
- **#72 / #107 — the verbatim-subgroup escape.** Suppression is counted (3 and 4 hidden) and the same-file cores publish anyway. On #107 the published pairs are not even byte-identical, though the pass documents grouping "by the exact source bytes".

Whichever way the arbitration lands: **`duplicated_loc` must not count a family the report suppresses.** That is independent of the arbitration and can land first.

This branch neither caused nor can close these — `b235c1a5` (#424) and `c3ce7882` brought them to `main`. It is the release vehicle, so it ships them. At this HEAD the branch touches both files mechanically (`Rc`→`Arc`, sharding) with the decision logic unchanged.

Already fixed and green: the #69 hidden-group summary wording, un-skipped, `CURATED_SKIPS` row deleted.

## Checked and cleared — not blockers

Recorded so neither gets re-raised against this release.

- **#458 — rendered cluster signals average over pairs the detector never admitted.** Confirmed at HEAD: two byte-identical TypeScript files render `identical` at `structural / token_jaccard / fused = 1.0 / 1.0 / 1.0` when scanned alone, and `nearly_identical` at `0.9982 / 0.8313 / 0.7953` when the same pair sits inside a six-member cluster. Byte proof loses its bucket to averaging. Real, and critical — but `cluster/signals.rs` is present at `f92300e` and the mean predates it, so it is not a regression. Tracked in the plan.
- **#459 — "adding one duplicated file deletes existing findings" does not reproduce.** Filed against `ts-mixed-band` on the claim that adding a byte-identical `ledger_a_copy.ts` drops the report from 2 clusters / 5 files / 100% to 1 cluster / 2 files / 11.11%. Re-measured cold, warm, and across `--min-nodes` 8/12/20/30/40: adding the copy *increases* coverage every time — 5 files → 6, clusters 2 → 2 or 3, duplication stays 100%, `duplicated_loc` rises 15 → 18, and the cold and warm-cache reports agree exactly. The original figures came from a contaminated scratch directory. The issue is wrong and needs a correction comment.

## To finish this release

- [x] **Gate the two benches** — `required-features = ["benchmark"]` on both `[[bench]]` sections (§1). Fixed by `release-gate` over TMC; verified here against the exact gate command.
- [x] **#434 — `duplicated_loc` must not count a suppressed family.** No separate fix exists: metrics already fold only visible clusters, pinned green by `metric_excludes_hidden_clusters` (re-run here, passing). This item collapses into the two fix items below — what § 2 still shows counting is the *published* trio, which goes hidden only when #72 lands.
- [ ] **#434 — decide the `[CLONE-NOISE-VERBATIM-SUBGROUP]` arbitration** (cross-file-hidden versus verbatim-published) and write it into `docs/specs/noise.md`.
- [ ] **#434 — fix #70 / #71**, where the filter records no suppression at all. Move `rename_needs_an_anchor` with it or explain why it still holds.
- [ ] **#434 — fix #72 / #107**, where suppression is counted and same-file cores publish anyway. Blocked on the arbitration.
- [ ] **Restate all four #434 pins** against the decided spec and delete their `CURATED_SKIPS` rows.
- [ ] **Correct #459 on GitHub** — the measurement does not hold; post the cold/warm/min-nodes sweep. Do not close it; that is the issue author's call.
- [ ] **Re-bless the stale goldens once, last** — `report_golden` (#432) and `incremental_multilang_golden` ×3 (#433) are stale, not wrong, and `[PERF-FLUTTER-TODO-ACCURACY]`'s recorded hash `2562e181…` predates `[PIPELINE-CLUSTER-ELECT-CONTAINER]`. Blocked on #432 and #433 landing.
- [ ] **Run the full gate on the candidate.** A strict `make test-corpus` cannot pass yet — `flutter/memory` and `fsharp/memory` are `corpus/known-failures.json` entries under #166, `corpus/flutter.json` sets `max_peak_rss_mb: 9000` above a standard runner, and #426 keeps `corpus_manifest_contract` red. Either land those first or state plainly that the release ships without a strict corpus run.
- [ ] **Validate the candidate packaged Action** through the download/install/execute path users receive. A conditional `diff-gate` job reporting a skip is not evidence.

## Retired findings — citation index

Eight source files cite this table by title; it stays.

| finding | pinned by |
|---|---|
| streamed LSH construction | `lsh/banding.rs` |
| admission parity | `pair/gate_parity_tests.rs` |
| first-seen pair deduplication drops stronger evidence | `pair/candidates/builder.rs`, `tests/pair_evidence_merge.rs` |
| parallel rescue | `overlap/rescue.rs::shard_equivalence_tests` |
| mixed-size overlap fallback | `overlap/tests.rs` |
| segmented-store remove/upsert logic has no changed test | `pipeline/session/store.rs` |
| Large parallel paths lack black-box parity coverage | `pipeline/corpus/tests.rs` |
| Removed signature-construction performance assertions | `pipeline/signatures/tests/canary.rs` |
