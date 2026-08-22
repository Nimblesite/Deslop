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
