# Release audit — what is still open

Supersedes `regression-audit-v0.32.0.md` (release regressions against `f92300e` = `v0.32.0`) and `performance-branch-review.md` (the `performance` branch closing audit). Everything either document recorded as fixed has been re-measured and dropped; what follows is only what is still open.

**Method.** Every claim below is measured at HEAD, not read off a prior document. Where the two source documents disagreed with the measurement, the measurement wins and the correction is stated inline.

**Measured baseline.** Whole workspace, release profile, all targets: **13 targets, 0 failed**. Every ignored test is a row of the `CURATED_SKIPS` registry in `crates/deslop/tests/skip_policy_contract.rs`, and the registry now holds **28** rows — read straight out of the array, not off an older sweep:

| issue | rows | what is skipped |
|---|---|---|
| #422 | 11 | the cloned corpus repositories (`corpus_repos.rs`) — too large for CI |
| #434 | 4 | the Python noise pins in § 1 |
| #433 | 4 | the three multilingual incremental goldens and `lsh_only_nearmiss_recall` |
| #439 | 3 | the curated-extent contract in `deslop-test-support` |
| #432 | 3 | both operator-drift pins and `report_golden` |
| #369 | 2 | the two embedding pins |
| #426 | 1 | the corpus manifest scope contract |

By crate: 24 in `deslop`, 3 in `deslop-test-support`, 1 in `deslop-lsp`. Every non-corpus row was executed with `-- --ignored` and is red for the reason its skip declares.

## Verdict: not ready for release

One class of defect blocks it: **the Python noise families in § 1 ship in the release binary, publish visible clusters, and move `duplication_percent`.** On `python-issue-71` the scaffolding family ranks **#1 — above the real clone staged in the same run**. Everything else on this page is either pre-existing debt that shipped in v0.32.0 too, or process/gate work that does not change what a user's report says.

---

# 1. Blocking — noise families reach the report (#434, #71, #79, #103)

Four fixtures that asserted **zero clusters** at v0.32.0 now publish. Measured with the release CLI at each pin's own `--min-nodes`, on the checked-in fixture bytes:

| fixture | mn | what publishes | dup% | `duplicated_loc` | `clusters_hidden` |
|---|---|---|---|---|---|
| `python-issue-71-rest-endpoint-shape` | 4 | **one `nearly_identical` family, 4 occurrences, ranked #1** — all in `test_endpoints.py` | 70.18 | 40 | **0** |
| `python-issue-70-test-data-variation` | 8 | one `structural_only` family, 4 occurrences — all in `test_write_file_calls.py` | 57.14 | 28 | **0** |
| `python-issue-72-monkeypatch-setenv` | 4 | one `structural_only` family, 3 occurrences — all in `test_fly_host.py` | 67.39 | 31 | 3 |
| `python-issue-107-chained-dict-assert` | 4 | three `identical` clusters, one same-file pair per test file | 45.83 | 22 | 4 |

The control clone each fixture stages is 16 duplicated lines. Every row above counts the family on top of it, so `--fail-over` trips on scaffolding.

> **Read the "what publishes" column as families, not cluster counts.** Every row except `python-issue-107` is **one** cluster with that many occurrences; the control clone is the other. Re-measured at HEAD with the release CLI: `#71` → 2 clusters / 0 hidden / 70.2%, `#70` → 2 / 0 / 57.1%, `#72` → 2 / 3 / 67.4%, `#107` → 4 / 4 / 45.8%, `#69` → 0 / 5 / 0.0%. The figures below are confirmed, not restated.

**Root cause of the `#70` / `#71` half, measured.** `[CLONE-NOISE-LITERAL-VARIATION-CALLS]` is the filter that should suppress these, and it never fires because `cluster_filters/calls.rs::every_covered_statement_has_call` returns `false`: each `test_delete_*` body ends in `assert resp.status_code == 204`, a statement carrying **no call**, so the whole-function occurrence fails the covered-statement precondition before any literal comparison happens. That precondition exists for a real reason — `rename_needs_an_anchor` pins it, and a call-free statement beside a varying call *can* be authored work worth extracting — so this is a spec arbitration, not a threshold to loosen: an assertion on the value the varying call returned is part of the idiom, while an authored computation is not, and the filter cannot currently tell them apart.

**Correction to the standing framing.** `docs/plans/fused-score-followups.md` attributes all four to `[CLONE-NOISE-VERBATIM-SUBGROUP]` republishing "the intra-file byte-identical core" of a suppressed family. The measurement says these are **two different defects**:

- On `python-issue-70` and `python-issue-71`, `clusters_hidden == 0`. Nothing was suppressed and then republished — **the noise filter never fired at all**, and what publishes is not byte-identical (`structural_only` / `nearly_identical`). The verbatim-subgroup rule does not explain these two.
- On `python-issue-72` and `python-issue-107`, a suppression decision *was* taken (3 and 4 hidden) and same-file cores publish alongside it. That is the verbatim-subgroup escape as described.

A fifth pin, `polymorphic_gate_hides_rename_clone`, was a third thing again: `python-issue-69-abstract-method` publishes nothing (0 clusters, 5 hidden), and the failure was the **wording** — the summary blamed "your .deslop.toml config" in a scan root with no such file. **That one is now fixed and the pin runs in `make test`:** the summary names Deslop's own filters, so its `#[ignore]` and `CURATED_SKIPS` row are gone.

**Which of these are regressions from the release commit — measured against `f92300e`, not inferred.** The four pins are not equivalent, and only two of them are regressions:

| pin | fixture at `f92300e` | test at `f92300e` | verdict |
|---|---|---|---|
| `python-issue-70` | **did not exist** | did not exist | new pin on this branch — not a regression |
| `python-issue-71` | **did not exist** | did not exist | new pin on this branch — not a regression |
| `python-issue-72` | present, **byte-identical to HEAD** | green, not `#[ignore]`d, asserted `cluster_count == 0` | **regression** |
| `python-issue-107` | present, **byte-identical to HEAD** | green, not `#[ignore]`d, asserted `clusters == 0` and `duplicated_loc == 0` | **regression** |

The two regressions were reproduced with the release CLI on the *baseline* file set — only the files that existed at `f92300e`, with the control clones this branch added left out — so nothing but the detector changed:

- **`#72`**: `test_fly_host.py` alone. `files_analysed 1`, and the stage ledger shows **every noise filter at `fired=0`**, including `literal_calls`. One `structural_only` cluster publishes with three occurrences (lines 1–5, 8–12, 15–19) — the three `monkeypatch.setenv` scaffolding functions. `v0.32.0` suppressed this family; HEAD does not fire on it at all. The cause is the same `every_covered_statement_has_call` precondition measured above: each body ends `explicit_host_id = "fly-1"` and `assert explicit_host_id == "fly-1"`, two statements carrying no call.
- **`#107`**: the three pytest modules alone. The ledger reads `structural_family_split 15 → 15`, then `noise_verbatim_split` **15 → 22** — the split is the only stage that adds clusters — and three survive ranking as `identical`. Each is a *pair of adjacent one-line assertions inside a single function*, e.g. `assert data["model_config"]["provider"] == "openai"` with `assert data["model_config"]["model"] == "gpt-4o"`. Those two lines are not byte-identical, so a pass documented as grouping "by the exact source bytes" is publishing them under a bucket that claims byte-identity.

**Where the two regressions came from.** Neither was introduced by the branch that is currently in review. `verbatim_subgroup.rs` did not exist at `f92300e` at all, and the `every_covered_statement_has_call` precondition in `calls.rs` was added after it; both arrived on `main` in `b235c1a5` (PR #424, "Fused-score accuracy follow-ups: verbatim subgroups…") and `c3ce7882` (the corpus-scale performance pass). `git diff origin/main...HEAD` touches neither file, so the current branch neither causes these two nor can close them by being held back. They are `main`'s to fix, tracked as gh #434.

Both pins are `#[ignore]`d with a `[SKIP-UNFINISHED]` reason naming gh #434 and `[CLONE-NOISE-VERBATIM-SUBGROUP]`, and both are registered in `CURATED_SKIPS`. Run with `-- --ignored` they fail, for the real reason, on the release CLI. That is what the skip registry is for: a shipped defect stays visible, counted, and attributable to an open issue instead of quietly passing. The bargain it records is unfinished work — deleting the pin, or weakening its assertions to make it pass, would be the thing the policy exists to prevent.

`[CLONE-NOISE-VERBATIM-SUBGROUP]` exists to close a real false negative (a proven copy vanishing because one shape-compatible stranger joined its cluster). Both directions are real defects; the spec arbitration was never resolved, and the pins were skipped instead of restated. Whichever way it lands, `duplicated_loc` must not count a family the report suppresses.

# 2. Accuracy defects, not blocking (all pre-existing or in-flight)

## Operator-only drift reaches the act-now tier and outranks the real clone — #432

`operator_drift_is_not_duplication` ×2, red. Measured: `ledger_credit.py` / `ledger_debit.py` differ only in `+` versus `-`, and render `nearly_identical`, `structural=0.9907`, `token_jaccard=1.0000`, `fused=0.9477`, weight `101.400`. They rank **first**, ahead of the corpus's one genuine `identical` pair at `fused=1.0000`. A `find-similar` consumer is told to write one where the other is meant.

v0.32.0 was worse here — operators collapsed to a shared placeholder, so `+` and `-` hashed identically. This is debt made visible by `[PIPELINE-NORMALIZE-AST-OPERATOR]`, not debt introduced.

## Mixed passes measure different content evidence than cold — #433

`lsh_only_nearmiss_recall`, red at `[PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]`. On identical corpus bytes the mixed pass and the cold pass diverge in `clusters`:

| signal | mixed | cold |
|---|---|---|
| `agreement` | 0.3333 | 0.3590 |
| `rename_consistency` | 0.5608 | 0.5833 |

`fused` survives at 0.85 here only because the shape term dominates; the rendered `evidence_verdict` already differs ("share 0.33 of their content" vs "0.36"), and a cluster whose content term is the max would move bucket between two runs of the same code.

**Correction.** The regression audit recorded this as "not reproducible through the CLI, specific to the LSP path". That was measured cold-vs-fully-warm only. It reproduces through the CLI on the **mixed** pass.

## The committed goldens are stale — #432 / #433

`report_golden` ×1 and `incremental_multilang_golden` ×3, red. `report_golden`'s drift is `canonical_node_count` 60 → 68 and consequently new cluster ids — the `[PIPELINE-CLUSTER-ELECT-CONTAINER]` election changing what a cluster is made of. These are stale, not wrong. Bless **once**, last, after #432 and #433 land.

The same staleness reaches the Flutter validation: `[PERF-FLUTTER-TODO-ACCURACY]` records report hash `2562e181…` as the accepted deterministic output, recorded before the container election. That hash no longer describes HEAD.

## Pre-existing engine defects with open issues

- **#443** — `content/frontier.rs::positional_agreement` returns `1.0` when nothing was measured, so "no authored content to disagree on" is indistinguishable from byte-proven agreement. Untouched by any branch this cycle.
- **#431** — `buckets/gate.rs` overwrites the measured `token_jaccard` with `1.0` for `NearlyIdentical` clusters at `structural >= STRUCTURAL_SATURATION_FLOOR` (0.99). The Merkle argument it rests on does not cover every cluster routed there.
- **#389** — one physical duplication published twice (method view + signature-line view); containment fails by 7 bytes on the leading `public` modifier.
- **#421** — a sub-line fragment published as a cluster (two dict entries on one line).
- **#362** — two unrelated const-declaration files produce the repository's largest ranked finding.
- **#356** — embeddings-on ANN bridges mutate structural components before measurement. `embedding_route_invariance` went green under the container election and is unskipped; the issue itself is not closed.
- **#369** — `issue_343_sum_clamp_saturation` routes `nearly_identical` where the pin demands `same_behavior`; `lsp_embedding_determinism` red with it. Both wait on `embedding-accuracy-plan.md` §1.

## False positives with no negative fixture

**#71 / #103 / #285**, **#79**, **#283 / #284** — each needs one fixture asserting the family stays hidden **while a real clone in the same run stays visible**. #71, #79 and #103 already have fixtures; what they lack is a passing engine (§ 1).

# 3. Gates and coverage

- **The VS Code extension host now has a line-coverage floor (#440).** It never did before, and the ~6,300 lines of `clients/vscode` changed this cycle went in under no gate at all. The host writes no V8 profile for extension code, which is real — but that made the coverage unmeasurable only through V8, not unmeasurable. The counters are now compiled into the modules and dumped from inside the host, measuring **87.6%** across all 43 compiled modules, enforced as `vsix.extension_threshold`. No Testing API migration was needed. See [vsix.md §VSIX-TESTING-COVERAGE](specs/vsix.md#vsix-testing-coverage).
- **The corpus gate has never run in CI (#422, blocked on #166).** All 11 repository checks are `#[ignore]`d — minutes of wall time and >13 GB peak per repository.
- **`corpus/flutter.json` sets `max_peak_rss_mb: 9000`**, above a standard GitHub Actions runner. The manifest is now correctly the single source of truth for the figure, and its rationale says so — but the rationale no longer states the reasoning that used to bound it (the shipped Action must not be OOM-killed). `flutter/memory` and `fsharp/memory` are `known-failures` entries under #166, so no gate moved.
- **#426** — `corpus_manifest_contract` is red: `flutter` has no `expect_files_min`, so a scan that analysed zero files would satisfy every cluster assertion in the manifest (#342's failure mode).
- **Release evidence** — the candidate packaged Action has never been validated through the download/install/execute path users receive. A conditional `diff-gate` job reporting a skip is not evidence.

# 4. Documentation and bookkeeping drift

These are measured disagreements between code and the documents that describe it. Under the repo's own rule (code, specs and tests must agree) each is a defect, not a tidy-up.

- **`skip_policy_contract.rs`'s own module doc miscounts its registry.** It says "twenty-two gh #432–#435 entries" and "three embedding entries". Measured: **12** entries for #432–#434 (#435 has none left) and **2** embedding entries.
- **`fused-score-followups.md` says "Ten accuracy tests remain red"** in the in-flight section. Measured: **12**.
- **`fused-score-followups.md` says "27 files exceed the 500-line rule"**. Measured: **32**, largest `deslop-mcp/tests/cli.rs` (2,891), `deslop-core/tests/live.rs` (1,473), `clients/vscode/src/test/unit/tree.topOffenders.unit.test.ts` (1,185). Split them or gate the rule.
- **#345 / #363** — `REPORTING-CONTEXT.md` and the site accuracy page still describe obsolete CLI defaults and an obsolete ranking formula. `fusion.md` and `pipeline.md` were re-read and agree with the code; these two have not been.
- **Eight source files cite `docs/performance-branch-review.md` by path** for the finding each test pins. Retiring that document means repointing them (see the index at the foot of this page).

---

# Checklist

## Blocks the release

- [ ] **#434 — decide the `[CLONE-NOISE-VERBATIM-SUBGROUP]` arbitration** (cross-file-hidden versus verbatim-published) and write it into `docs/specs/noise.md`.
- [ ] **#434 — fix `python-issue-70` and `python-issue-71`, where the noise filter records no suppression at all** (`clusters_hidden == 0`) and the family publishes `structural_only` ×4 / `nearly_identical` ×4-ranked-first. This is not the verbatim-subgroup escape and needs its own root cause.
- [ ] **#434 — fix the verbatim-subgroup escape on `python-issue-72` and `python-issue-107`**, where suppression is counted and same-file cores publish anyway.
- [ ] **#434 — `duplicated_loc` must not count a family the report suppresses**, whichever way the arbitration lands.
- [x] **#434 — the hidden-group summary names Deslop's own filters** in a scan root with no `.deslop.toml`. `hidden_group_summary_names_the_hider_not_the_users_config` is un-skipped and green in `make test`.
- [ ] **Restate the two remaining #434 pins (`#70`, `#71`) against the decided spec and delete their `CURATED_SKIPS` rows.**

## Accuracy — after the release

- [ ] **#432** — discount operator disagreement in the confidence blend so `+`/`-` drift cannot reach an act-now bucket or outrank a byte-identical pair.
- [ ] **#433** — make the frontier-leaf population identical on the cold, warm and mixed paths.
- [ ] **#432 / #433** — re-bless `report_golden` and `incremental_multilang_golden` once, last, and review the diff.
- [ ] **`[PERF-FLUTTER-TODO-ACCURACY]`** — re-run the Flutter validation and re-record the accepted report hash; `2562e181…` predates `[PIPELINE-CLUSTER-ELECT-CONTAINER]`.
- [ ] **#443** — distinguish "no authored content measured" from agreement `1.0`.
- [ ] **#431** — stop overwriting measured `token_jaccard` for clusters the Merkle argument does not cover.
- [ ] **#389** — decide range convention versus predicate tolerance; assert exactly one `identical` cluster for the C# pair on `incremental-multilang` at `--min-nodes 8`.
- [ ] **#421** — stop publishing sub-line fragments; tighten `python_issue_69_abstract_method` to an empty visible surface.
- [ ] **#362** — two unrelated const-declaration files must not rank first.
- [ ] **#356** — ANN bridges must not mutate structural components before measurement.
- [ ] **#369** — `issue_343_sum_clamp_saturation` and `lsp_embedding_determinism`, via `embedding-accuracy-plan.md` §1. No further `#[ignore]` may be added to that suite.
- [ ] **#283 / #284**, **#285** — one negative fixture each, asserting the family stays hidden while a real clone in the same run stays visible.

## Gates

- [ ] **#440** — migrate the extension-host suites to the Testing API and restore a line-coverage floor for `out/**`.
- [ ] **#426** — curate `expect_files_min` and `expect_clusters` for `flutter` and `fsharp`; unskip `corpus_manifest_contract`.
- [ ] **#422 / #166** — bring the corpus suite inside a PR gate's resources; re-derive `corpus/flutter.json`'s `max_peak_rss_mb` from a fresh measured scan and state the runner bound in the rationale.
- [ ] Validate the candidate packaged Action end to end through the user-facing download/install/execute path.
- [ ] Run `make test-corpus` strictly (ignoring `known-failures.json`) on the release candidate.

## Documentation

- [ ] Correct `skip_policy_contract.rs`'s module doc: 12 entries for #432–#434, 2 embedding entries, no #435 entries.
- [ ] Correct `fused-score-followups.md`: "Ten accuracy tests remain red" → 12; "27 files exceed the 500-line rule" → 32.
- [ ] Correct `fused-score-followups.md` §#434 — it describes one defect where there are two, and `clusters_hidden == 0` on two of the four fixtures.
- [ ] **#345 / #363** — re-read `REPORTING-CONTEXT.md` and the site accuracy page against the shipped defaults and ranking formula.
- [ ] Repoint the eight `docs/performance-branch-review.md` citations at this document.
- [ ] Split or gate the 32 files over 500 lines.

---

# Retired findings — citation index

The `performance` branch closing audit is retired; every finding it raised is resolved and pinned. The titles are kept here only because tests cite them by name.

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

Also closed out on that branch and not repeated above: the Flutter manifest/plan figure contradiction (the manifest is now the sole source), the signature-arena I/O error swallowing (module deleted, `SignatureLookup` seam kept), the internal Rust API breaks (accepted — every crate is `0.0.0-dev` and ships only inside the VSIX), and the oversized modules the branch itself introduced.
