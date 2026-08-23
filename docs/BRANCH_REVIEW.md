# Branch fix plan

Only current, actionable work is listed here. Complete the TODOs in priority order without weakening existing accuracy, ranking, determinism, or corpus assertions.

## P0 — Stop empty authored evidence from scoring as perfect agreement

- [ ] Introduce an explicit unmeasured/empty state for authored-content agreement instead of returning `1.0` from `content/frontier.rs` and `content.rs` when no authored evidence was measured.
- [ ] Ensure empty or unmeasured evidence cannot satisfy the content-support gate or increase fused confidence in `buckets/gate.rs`.
- [ ] Stop deserializing a missing legacy agreement field as measured `1.0`; preserve legacy replay behavior without claiming evidence that is absent.
- [ ] Add black-box report assertions for bucket, fused score, agreement, occurrence set, and ranking. Keep byte-identical and real-rename controls in the same fixture.

Done when empty authored evidence is visibly distinct from perfect agreement at every routing and rendering surface, and cannot promote a cluster.

## P0 — Bound Flutter scan time and memory

The old 55-million-pair LSH fan-out and per-measurement logging are fixed, but the pipeline still builds the corpus serially, retains full pair/candidate populations, and performs shared-subtree rescue over a data-dependent population without a hard work budget.

- [ ] Add aggregate timings and cardinalities for corpus construction, LSH output, candidate construction, rescue eligibility, alignment/cache counts, and peak retained populations.
- [ ] Put explicit accuracy-preserving bounds on retained LSH/candidate populations and shared-subtree rescue work; avoid holding redundant full vectors simultaneously.
- [ ] Optimize or parallelize the serial corpus path only after the retained work is bounded.
- [ ] Re-run the pinned cold Flutter scan with controlled binary and corpus provenance.
- [ ] Prove the run finishes within 10 minutes and 7,168 MiB while preserving every curated `must_find`, precision, scope, bucket, range, ranking, and determinism assertion.

Done when the controlled Flutter run produces a report inside both budgets with no accuracy-contract changes.

## P0 — Make operator disagreement visible to the content signals

Two operator-drift tests are red on this branch and pin a real false positive. `crates/deslop/tests/fixtures/operator-drift/ledger_credit.py` and `ledger_debit.py` differ in exactly one token (`scaled + floor` vs `scaled - floor`) — they compute different answers — yet publish as `id=3c351ea8ee5cb48d bucket=nearly_identical fused=0.9477 agreement=0.9565 rename_consistency=1.0000 token_jaccard=1.0000 structural=0.9907`, clearing `ACT_NOW_FUSED` and outranking the byte-identical control in the same corpus.

Two independent mechanisms produce it. The second is on `main` and is filed there as [#431](https://github.com/Nimblesite/Deslop/issues/431) — the content gate overwrites a measured `token_jaccard` with `1.0` across the whole `structural >= 0.99` band, on a Merkle-identity argument that only holds at exactly `1.0`. Nothing below is a substitute for that fix; both must land for the pair to stop publishing.

The first is branch-only, because `main` emits no operator leaves at all and has no `Population` enum to extend:

- [ ] `content/rename.rs::pair_rename_consistency` measures only the identifier and literal populations, so a position where the two members carry different `__op__` leaves is invisible to it and it returns `1.0000` — a claim of perfect renaming for a pair that is not a rename.
- [ ] Give operator leaves a `Population` of their own so a disagreeing operator lowers rename consistency rather than being skipped, and confirm the content-support gate and `content_confidence` both see the reduction.
- [ ] Keep `each_family_member_normalises_to_its_own_operator_leaves` asserting exact ordered multisets; the leaves are already correct — the defect is downstream of normalization, in what the content signals choose to read.
- [ ] Turn `an_operator_only_difference_never_reaches_the_act_now_line` and `the_real_clone_outranks_every_operator_family` green without relaxing either assertion, and keep all four families and the published control in the fixture.

Done when a one-operator difference cannot reach an act-now bucket and the byte-identical control ranks first.

## P1 — Split the oversized operator-normalization module

`crates/deslop-core/src/lang/shared.rs` is 863 lines, and `operator_field_cases()` remains far above the 20-line function limit.

- [ ] Move operator classification and its grammar contract data into focused modules.
- [ ] Reduce every resulting file below 500 lines and every function below 20 lines.
- [ ] Preserve the language-keyed unfielded-operator rules, registry-driven cross-language collision checks, exact operator-leaf assertions, and the published operator-drift control.
- [ ] Do not duplicate normalization logic or weaken assertions during the split.

Done when the size limits pass and all operator normalization/report tests remain unchanged and green.

## Final acceptance

- [ ] Run `make ci` after the three TODOs are complete.
- [ ] Fix any current failure at root cause; do not preserve or chase superseded failures from the old CI timeline.
- [ ] Require a fully green run before merge.
