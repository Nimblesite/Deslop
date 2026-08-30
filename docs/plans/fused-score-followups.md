# Fused admission — remaining work

**What this file is.** The open work on fused pair admission, content routing and ranking. One status per item, once: spec ids for the rules, pins for the assertions. Nothing here restates a spec; work owned elsewhere is listed and not repeated.

**Owned elsewhere. Do not restate it here.**

| Work | Owner |
|---|---|
| The two `#[ignore]`d tests (#369 ×2), the token-signature root cause (#367), corroboration floors (#365), the mock embedder (#366), `MIN_COSINE` recall (#407), the role gate (#358) | [`embedding-accuracy-plan.md`](embedding-accuracy-plan.md) |
| Curated ground truth, negative corpus assertions, and the #331 / #336 / #339 / #347 / #401 close-outs | [`corpus-assertion.md`](corpus-assertion.md) |
| The metrics/gate row of #344 | [`weighted-metrics-plan.md`](weighted-metrics-plan.md) |
| Driving repo duplication under the 16.4 pin | gh #397, ledger in `.deslop.toml` |
| Unhardcoding the compiled tuning levers and recording their provenance (`embedding_top_k`, `type4_embedding_floor`, `low_structural_type4_ceiling`, `low_structural_type4_weight`, `proven_identical_token_floor`, `literal_table_*`) | [`unhardcode-tuning-plan.md`](unhardcode-tuning-plan.md) |

A candidate-route problem belongs here only when two runs produce the same final occurrence set and disagree on its admission, routing or rank.

## The one measure

Every reported cluster is a real duplicate, and every real duplicate is reported. Order this backlog by how much each item moves that number.

## The contract

`fused` is the pair admission score — the strongest single axis, decided pair by pair against a configurable bar ([FUSED-THRESHOLD](specs/fused.md)); it never refers to the whole cluster ([FUSED-SCOPE](specs/fused.md)). A cluster renders its bucket (the verdict), the elected pair's measured axes ([FUSED-CLUSTER-SIGNALS]), and its content evidence ([FUSED-CONTENT-GATE]) — never a fused number, and never a confidence-scaled weight ([RANK-MASS-SUM](specs/pipeline.md)). The rendered-confidence world — `signals.fused`, the three bands, `meets_fused_gate`, and the `fused_golden_bands.rs` / `fused_golden_invariants.rs` suites that cited the old contract by name — is deleted by the first backlog item; the surviving pins are the pair-level ones (`pair_admission_bounded_max.rs`, `issue_343_sum_clamp_saturation`).

## Landed — settled on this branch

One line each: what → spec → pin.

- **#373** polymorphic gate no longer hides consistently-renamed Type-2 clones — subject bodies compare as normalised kind streams ([CLONE-NOISE-POLYMORPHIC-CONTRACT]); dual-direction pin `polymorphic_gate_hides_rename_clone.rs`.
- **#410** certified renames carry no doubt — the asymptotic mass weight stops discounting a rename the anchor mass already vouches for, `rename_consistency` reads 1.0 ([FUSED-CONTENT-GATE]); pin `assert_certified_rename_reaches_act_now` re-points to routing in the rollout; [REPAIR-RENAME-LITERAL-ECHO] monotonicity survives.
- **#458 shape half** — a cluster renders one admitted pair's own shape axes, `signal_source` names it ([FUSED-CLUSTER-SIGNALS]); `pair_consistent_signals.rs`, `verbatim_family_survives_stranger.rs`.
- **Ranking weight is summed duplicated mass, never confidence-scaled** — [RANK-MASS-SUM] owns the formula; ties break by cluster id; `rank_mass.rs`.
- **Token-bridge welds, containers, regions** — [PIPELINE-CLUSTER-ELECT] + [PIPELINE-CLUSTER-ELECT-CONTAINER]; `csharp_merged_clone_families.rs`, `rank_structural_only_policy`, eleven unit tests.
- **Operators survive normalisation as their own tokens** — [PIPELINE-NORMALIZE-AST-OPERATOR]; six-language golden re-blessed (ids/node counts only), alignment cap 512→768, `SEMANTIC_EPOCH` 3.
- **Assertion instruments hardened** — #415 `fused_score_bounds.rs` fails on empty/missing signals; #398 `ReportFixture` one `FileId` per path (`report_fixture_file_identity.rs`); #435 callsite-interest anchor; #412 substring skips replaced with declared `#[ignore]`s under [TEST-SELECTION-SKIP] — `make test` runs the whole workspace unfiltered.
- **#440 / #426** — VSIX extension coverage floor 87.6% (`vsix.extension_threshold`); corpus manifests curated — `flutter`/`fsharp` `expect_files_min`/`expect_clusters`, curated Type-2 entries carry a `min_nodes` extent floor (gh #439), `corpus_manifest_contract` unskipped.

---

# Backlog

Every item is unpinned unless a test is named. **Write the failing fixture first and watch it fail** — the assertion is worth more than the fix.

## Engine accuracy

- [ ] **#389** — one physical duplication published twice: the C# `LedgerAlpha`/`LedgerBeta` method clone and its signature-line view disagree on the leading `public` modifier by 7 bytes. Decide range convention versus predicate tolerance; assert exactly one `identical` cluster at `--min-nodes 8` on `incremental-multilang`.
- [ ] **#421** — a sub-line fragment published as a cluster (`python-issue-69-abstract-method` at `--min-nodes 4`); tighten to an empty visible surface.
- [ ] **#362** — two unrelated const-declaration files must not rank first. Writable as a two-file fixture today.
- [ ] **#71 / #103 / #285**, **#79**, **#283 / #284** — one negative fixture each, asserting the family stays hidden while a real clone in the same run stays visible; the pin pattern exists (`python_issue_69_abstract_method` et al.). Recheck the language-agnostic data category before treating #283/#284 as open detector defects.
- [ ] **#458 content half** — `agreement` and `rename_consistency` must be the same elected pair's own values ([FUSED-CLUSTER-SIGNALS], [FUSED-CONTENT-GATE] §2). Both cluster means are quarantined with `panic!`; the per-pair machinery (`pair_agreement`, `pair_rename_consistency`) is retained. Repair: elect the shape axes' pair and render its own values — never a sum (ratios in `[0,1]`; sum is right for *mass*, not agreement). Red pin: `a_byte_identical_pairs_content_evidence_is_never_diluted_by_the_cluster`. Noted: a byte-identical pair reads `rename_consistency = 0.5556` alone — the pair value's anchor-mass weight, unchanged by the repair.
- [ ] **#432** — discount operator disagreement in the blend so `+`/`-` drift cannot reach the top band or outrank a byte-identical pair; then re-bless `report_golden` once.
- [ ] **#433** — make the frontier-leaf population identical on cold, warm and mixed passes; then re-bless the three goldens once.
- [ ] **#443** — distinguish "no authored content measured" from agreement `1.0`.
- [ ] **#431** — stop overwriting measured `token_jaccard` for clusters the Merkle argument does not cover.
- [ ] **#356** — ANN bridges must not mutate structural components before measurement (`embedding_route_invariance` green, issue open).
- [ ] **Retire cluster fused (code rollout)** — remove `signals.fused` from the wire and every surface, `meets_fused_gate`, the content-gate multiply (`buckets/gate.rs` `apply_content_gate`, including `content_confidence = max(A, discount × R)` and `RENAME_CONSISTENCY_DISCOUNT`), the fused tie-break in `report_weight.rs`, the band constants (`ACT_NOW_FUSED` / `REUSE_FUSED`), and the evidence-line fused; delete `fused_golden_bands.rs` / `fused_golden_invariants.rs`; re-point the certified-rename and history-determinism pins from rendered fused to routing outcomes (bucket + support) without weakening an assertion; update the code comments citing the old contract.

## Ranking and provenance

- [ ] **#363** — the confidence multiplier is settled ([RANK-MASS-SUM]); still open: `log2(1 + spanned_loc)` over lines (`pipeline.md`) versus `spanned_bytes` (`cluster.rs`), and whether the visible re-rank keeps a log term. Change whichever side is not the truth.
- [x] **7c — provenance relabelled in [FUSED-TUNING-LEVERS]** — `embedding_min_cosine` / `content_gate.support_floor` are **Derived** (SSCD tabulates `0/0.95`; SourcererCC's 0.7 is overlap, not Jaccard); `fused_threshold`'s cite corrected.
- [ ] Sweep `embedding_top_k = 5` against a corpus with large clone classes (every surveyed system ties topN to class size).
- [x] **7e — stated in [FUSED-STRATEGY-BOUNDED-MAX]** — the max runs over uncalibrated axes, the most generous axis wins by construction, and `fused_threshold` at 0.85 pays the precision bill.
- [ ] Inline literal tables in `buckets.rs` / `report_render.rs` need provenance; code comments must match the spec's derived labels — spec half landed ([FUSED-TUNING-LEVERS]).
- [x] **Gate-vs-scale settled** — content gates routing (bucket) and nothing else ([FUSED-CONTENT-GATE]); the rendered-confidence multiply and the question died with the cluster-fused rollout.

## Gates and coverage

- [ ] **#422 / #166** — bring the corpus suite inside a PR gate's resources. `flutter`'s ceilings re-derived (295 s / 7947 MB against 9000 MB) and its `memory` known-failure cleared; `fsharp/memory` remains; `max_peak_rss_mb: 9000` still sits above a standard runner.
- [ ] Re-read every "suite is green" claim against the unfiltered gate; drop `ollama.rs` from `rust.ignore_filename_regex` if the crate holds its floor without it.

## Reporting language — [PRINCIPLES-REPORT-NOT-DICTATE]

- [ ] Bucket sentences in `buckets.rs` (the single source) per the [CLONE-BUCKETS](../specs/taxonomy.md#clone-buckets) table; `clone_category.rs`'s *"Extract the duplicated logic into a shared function."* and `report_boilerplate.rs`'s *"Consider…"* / *"Review only if…"* go with them.
- [ ] Delete the TypeScript copies of Rust strings — `types/report.ts`, `severity.ts`, `bubble/renderParts.ts`, `bubble/live.ts`, `types/signals.ts` each restate a sentence `buckets.rs` owns, breaking the one-rendering rule.
- [ ] Rename `act-now` — 243 occurrences across 63 files (`ACT_NOW_FUSED`, `ACT_NOW_BUCKETS`, `isActNow`, spec prose) — to the bucket it denotes; the band constants themselves die with the rollout.
- [ ] `action_sentence` → `evidence_sentence` in the buckets sextuple; the wire names `action_hints` / `recommendation` stay (renaming breaks agent prompts for no accuracy gain).

## Public documentation and repository policy

- [ ] **#345** — `REPORTING-CONTEXT.md` (`schema_doc`) and the site accuracy page still describe obsolete CLI defaults and the obsolete ranking formula.
- [ ] 37 committed source files exceed the 500-line rule (largest `deslop-mcp/tests/cli.rs` at 2,902) — split them or gate the rule.

## Blocked elsewhere — do not start these here

- [ ] The two remaining embedding `#[ignore]`s (`lsp_embedding_determinism`, `issue_343_sum_clamp_saturation`, #369) wait on [`embedding-accuracy-plan.md`](embedding-accuracy-plan.md) §1. No further ignore may be added to that suite.
- [ ] The corpus close-outs #331 / #336 / #339 / #347 / #401 and a strict `make test-corpus` on the release candidate wait on [`corpus-assertion.md`](corpus-assertion.md) Part A.

## Skipped while the follow-ups are in flight — gh #432 / #433 / #434

Twelve accuracy tests remain red against in-flight work; #434's four are regressions since v0.32.0 and block the release, owned by [`../release-audit.md`](../release-audit.md) §2. Each is `#[ignore]`d under [TEST-SELECTION-SKIP] citing its issue; each ends by deleting the attribute **and** its `CURATED_SKIPS` row — assertions untouched. Eleven ended: the #435 pair (callsite anchor), `history_determinism` (re-pinned at `0.9 × shape`), the `state_file_and_ipc` trio (live-cache seed), `rank_structural_only_policy` ×2 (container election), `refactor_merge_refusals`, `ts_issue_284_produce_then_assert`, `embedding_route_invariance` (#356), and #434's hidden-group summary.

- **#432** — `operator_drift_is_not_duplication` ×2, `report_golden` (ids/node counts stale after [PIPELINE-CLUSTER-ELECT-CONTAINER]). Ends when the blend discounts operator disagreement; re-bless `report_golden` once, last.
- **#433** — `incremental_multilang_golden` ×3, `lsh_only_nearmiss_recall`. Ends when the frontier-leaf population is identical cold and warm.
- **#434** — the four Python noise pins. Do not restate them here; see the release audit §2.

---

# Ledger

Kept only for the fused repair IDs cited from tests and specifications.

| ID | What it fixed | Held by |
|---|---|---|
| `[REPAIR-RENAME-ANCHOR-MASS]` (#405) | A maximal Type-2 rename below the literal-anchor floor priced to `0.0588` — reported as coincidence. Replaced a four-literal cliff with smoothly weighted Baker-corroborated anchor mass | `type2_rename_anchor_floor.rs`, `fused_golden_bands.rs`, `js_language_features.rs`, `js_ts_clone_buckets.rs`, `common/signals.rs`, `taxonomy.md` |
| `[REPAIR-SUBSUME-CONTENT-FIRST]` (#367, #408) | Measured content before destructive cross-cluster subsumption and made the survivor election read it: a demoted view never deletes a credible one, a demoted encloser yields only to verbatim-proven nesting that carries statement mass, and between credible views enclosure stands | `cross_cluster_collapse.rs`, `type3_enclosing_method.rs`, `cluster/subsume/election.rs`, `[PIPELINE-CLUSTER-SUBSUME]` in `pipeline.md` |
| `[REPAIR-RENAME-LITERAL-ECHO]` (#409) | Counted a literal renamed alongside its symbol as consistent rename evidence instead of disproof, so a more complete rename can never score below a less complete one | `rename_literal_monotonicity.rs`, `js_language_features.rs`, `content/rename.rs`, `[FUSED-CONTENT-GATE]` in `fused.md` |
