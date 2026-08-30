# Fused pair admission and evidence reporting — implementation plan

This plan closes the gap between the shipped cluster-confidence model and the pair-scoped contract in [fused.md](../specs/fused.md). It owns pair admission, elected-pair evidence, content routing, and removal of cluster-level `fused`. Metrics, embedding-specific recall, corpus curation, and tuning configuration remain in their dedicated plans.

## Governing contract

- `fused` is only the pre-rescue pair admission score `f_admit(p) = clamp(max(H,J,E),0,1)` ([FUSED-SCOPE](../specs/fused.md#fused-scope), [FUSED-STRATEGY-BOUNDED-MAX](../specs/fused.md#fused-strategy-bounded-max)). `H` is exact Merkle evidence; graded structural overlap `S` is not substituted into this score.
- Shared-subtree rescue is a separate cross-file compound gate. It may admit a below-threshold pair but never changes that pair’s fused value ([FUSED-SHARED-SUBTREE](../specs/fused.md#fused-shared-subtree)).
- A cluster elects one admitted pair by `q = max(S,J,E)` with corpus-order tie-breaking, then renders that pair’s `S`, `J`, `E`, `A`, and `R` with `signal_source` ([FUSED-CLUSTER-SIGNALS](../specs/fused.md#fused-cluster-signals), [FUSED-CONTENT-GATE](../specs/fused.md#fused-content-gate)). It renders no fused score.
- Content evidence selects the bucket; it never scales ranking weight. Ranking is `canonical_nodes × (visible_members − 1) × category_multiplier × structural_only_multiplier`, sorted by weight descending and cluster id ascending ([RANK-MASS-SUM](../specs/pipeline.md#rank-mass-sum)).
- Thresholds are configuration-backed operating points with the provenance recorded in [FUSED-TUNING-LEVERS](../specs/fused.md#fused-tuning-levers). No repair may widen a threshold to pass a fixture.

## Execution rule — rip out and replace in one hit

Delete the shipped cluster-confidence model and every dependency on it at the start of one uninterrupted cutover. Do not scaffold the new contract beside the old one, preserve a compatibility field, keep an adapter, stage schema versions, land preparatory tests, or maintain compilation between edits. The engine, wire model, generated types, CLI, LSP, MCP, renderers, VSIX, tests, fixtures, and documentation all move together. A broken build and broadly red suite are expected intermediate states; no effort is spent making an intermediate state work. The only valid endpoint is the complete final contract below, compiling and passing its full-strength assertions.

## Wholesale replacement scope

**Delete the old public contract wholesale.**

- [ ] Immediately remove cluster `signals.fused`, `meets_fused_gate`, fused bands, `ACT_NOW_FUSED`, `REUSE_FUSED`, the content-confidence multiply, `RENAME_CONSISTENCY_DISCOUNT`, the ranking tie-break, fused history fields, and every renderer or client branch that consumes them.
- [ ] Delete the old cluster-confidence tests and fixtures as part of the same cutover; replace their meaningful accuracy assertions with final-contract assertions over pair admission, occurrences, bucket, elected axes, content support, weight, and order. Do not keep obsolete assertions green through a compatibility layer.
- [ ] Replace the canonical typeDiagram model immediately and regenerate only the final Rust and TypeScript shapes. No dual schema or deprecated field survives.

**Install the pair-scoped public contract across every surface.**

- [ ] JSON, text, HTML, LSP, MCP, and VSIX expose the elected pair’s measured axes and source with no cluster fused score, fused band, or fused gate.
- [ ] Exact `H`, token `J`, or embedding `E` may clear the pair-specific admission bar; rescue requires every configured corroboration and leaves `f_admit` unchanged.
- [ ] A cluster whose strongest `S`, `J`, and `E` belong to different pairs must render one real elected pair rather than per-axis maxima.
- [ ] Equal-mass clusters sort by cluster id regardless of their old cluster confidence.

**Replace elected-pair content evidence — gh #458.**

- [ ] Replace the quarantined cluster means for `agreement` and `rename_consistency` with `pair_agreement` and `pair_rename_consistency` measured on the same elected pair as `S`, `J`, and `E`.
- [ ] Carry that pair through `signal_source`; never elect content evidence separately and never average across closure members.
- [ ] The final `a_byte_identical_pairs_content_evidence_is_never_diluted_by_the_cluster` assertion must cover the elected pair, occurrence paths, five rendered axes, bucket, and routing support; no intermediate test-only change is landed.

**Remove every remaining cluster-fused implementation site.**

- [ ] Remove `signals.fused`, `meets_fused_gate`, fused bands, `ACT_NOW_FUSED`, `REUSE_FUSED`, and every renderer or client branch that derives a cluster verdict from them.
- [ ] Delete the content-confidence multiply in `buckets/gate.rs`, including `content_confidence = max(A, discount × R)` and the retired `RENAME_CONSISTENCY_DISCOUNT`; routing reads `support = max(A,R)` directly.
- [ ] Remove the `signals.fused` tie-break from `report_weight.rs`; sort equal mass by cluster id.
- [ ] Update `docs/models/live-ipc.td` and regenerate Rust and TypeScript wire models. Do not hand-edit generated files.
- [ ] Delete `fused_golden_bands.rs` and `fused_golden_invariants.rs` during the cutover and replace their meaningful assertions against the final observables before the one change is complete. Pair-level bounds remain covered by `pair_admission_bounded_max.rs` and `issue_343_sum_clamp_saturation.rs`.
- [ ] Remove fused bars, labels, tooltips, help topics, history fields, and local thresholds from every UI. Surfaces render the bucket, elected evidence, and engine-authored explanation.

**Replace ranking with the one governing formula.**

- [ ] Remove every remaining `log2(1 + spanned_loc)`, `spanned_bytes`, or confidence factor from final rendered weight calculations and stale documentation; [RANK-MASS-SUM](../specs/pipeline.md#rank-mass-sum) is the only formula.
- [ ] Assert visible occurrence count, canonical node count, both policy multipliers, exact weight, and `weight desc → id asc` ordering in E2E output.
- [ ] Keep data and structural-only multipliers as explicit policy, not evidence confidence.

**Replace the remaining defective detector and routing behavior in the same change.**

- [ ] **#389:** publish the `LedgerAlpha`/`LedgerBeta` physical duplication once, with one range convention and one `identical` cluster at `--min-nodes 8`.
- [ ] **#421:** suppress the sub-line `python-issue-69-abstract-method` fragment while retaining a real clone in the same run.
- [ ] **#362, #71, #79, #103, #283, #284, #285:** add one negative fixture per family, asserting exact hidden noise and exact retained duplicates.
- [ ] **#432:** operator disagreement must reduce content support enough that `+`/`-` drift cannot take a stronger bucket than a byte-identical pair.
- [ ] **#433:** cold, warm, and mixed passes must produce the same frontier-leaf population and report.
- [ ] **#443:** represent “no authored content measured” separately from measured agreement `1.0`.
- [ ] **#431:** correct rendered token Jaccard only when every member shares one Merkle hash; graded structural saturation is not equality.
- [ ] **#356:** embeddings may add candidate pairs but must not mutate the occurrence set or evidence of an existing structural component through ANN bridge topology.

**Replace reporting language and documentation in the same change.**

- [ ] Make `buckets.rs` the single source for evidence sentences; delete TypeScript copies and rename `action_sentence` to `evidence_sentence` without changing agent-facing action-hint wire fields.
- [ ] Remove the `act-now` vocabulary in favor of explicit buckets.
- [ ] Update `REPORTING-CONTEXT.md`, the site accuracy page, and all examples so `fused` appears only as a pair-admission concept.
- [ ] Replace obsolete code comments and test names that describe cluster fused confidence; retain historical issue references only where they explain a surviving assertion.

## Issue provenance (moved from fused.md)

The spec is written issue-free; the issues that pinned its defect-provenance levers and its design decisions live here. Each row records what the issue governs, so the spec's `**Defect**` / `**Derived**` labels remain checkable claims.

| Issue | Governs |
| --- | --- |
| #104 | Verbatim guard: a verbatim pair among lookalikes (share 2/3) must stay visible |
| #197 | Structural-only family: acceptance criterion (`token_jaccard = 0.00`, `embedding_cos = 0.00`); in-file REST settings family (0.72–0.80) stays demoted |
| #232 | Token-signal correction: a same-Merkle-hash cluster's normalised k-gram sets are equal by construction, so the rendered `token_jaccard` is corrected to 1.0 |
| #286 | Provider-owned input budget: the upstream fixed cap dropped 14,723 of 175,160 subtrees |
| #301 | Election tie-break: earliest pair in corpus order |
| #331 | flutter/flutter shape-echo cluster (`structural = 0.62`, `token_jaccard = 0.98`): the token layer echoing shape, not reporting content |
| #336 | Shape-match saturation on the normalised representation says nothing about content |
| #339 | Same-file collapse can remove every admitted endpoint → no source pair, all-zero axes |
| #341 | Content-gate floors (`support_floor`, `promote_floor`, `literal_table_min_fraction`, `literal_table_min_literals`, `verbatim_member_share_floor`) |
| #343 | Sum-arm removal; the per-pair mean dilution of a byte-identical pair to `structural = 0.36` |
| #346 | Rename-evidence cliff → half-saturation mass weight; `rename_consistency_discount` |
| #356 | `structural_only_max_support` read as a support floor → gate verdict turned on whether the embedding pass ran |
| #368 | `max_endpoint_node_ratio`; `saturating_token_floor` |
| #372 | Byte-identical snippets share one vector → cosine exactly 1.0 |
| #408 | Shared-subtree overlap replaces the literal `0.0`; rescue floors (`shared_subtree_min_overlap`, `shared_subtree_min_jaccard`, `shared_subtree_min_node_count`) |
| #409 | Literal echo ([REPAIR-RENAME-LITERAL-ECHO]): a substituted literal whose bytes transform exactly by one bijection substitution corroborates the rename |
| #410 | Certification: a contradiction-free rename carries no doubt left for the mass term to price |
| #431 | Token correction scoped by digest equality, never by a `structural` reading |
| #458 | Elected-pair evidence and no cluster-level fused: a byte-identical pair renders `1.0/1.0` and keeps its bucket; mass outranks confidence at ranking |

## Owned elsewhere

| Work | Owner |
|---|---|
| Token-signature recall, embedding candidates, mock/real embedding parity, ANN determinism, #356, #357, #358, #365, #366, #367, #369, #407 | [embedding-accuracy-plan.md](embedding-accuracy-plan.md) |
| Weighted duplication percentage and weighted CI gate | [weighted-metrics-plan.md](weighted-metrics-plan.md) |
| Curated ground truth, negative corpus assertions, and release corpus close-outs | [corpus-assertion.md](corpus-assertion.md) |
| Configuration and provenance of compiled tuning levers | [unhardcode-tuning-plan.md](unhardcode-tuning-plan.md) |
| Corpus resource ceilings and ignored release tests | [corpus-assertion.md](corpus-assertion.md) and the release audit |

## Checklist

Work in progress; each box flips only when the change compiles and its assertions pass.

- [x] Delete `ReportSignals.fused` and `ReportCluster.meets_fused_gate` from `docs/models/live-ipc.td`; rename `agreement`/`rename_consistency` to `pair_agreement`/`pair_rename_consistency`; regenerate Rust + TypeScript wire models.
- [x] Generator metadata documents the elected pair (no Mean/Pooled copy).
- [x] VSIX: fused help topic, fused bar, fused tooltip, fused signals line, fused-gate bubble admission removed; strip renders shape / embedding / `pair_agreement`.
- [x] VSIX fixtures: `bucketMeetsFusedGate` deleted; `bucketSignals` carries elected-pair axes.
- [x] VSIX suites rewritten against the final wire shape; a schema test asserts `fused`/`meets_fused_gate` are absent from the generated types.
- [ ] TypeScript compile + unit suites green.
- [ ] VSIX build + webview/UI smoke against the new wire.
- [ ] Site + docs: `fused` appears only as a pair-admission concept.
- [x] Engine owners' slices reviewed on TMC (gate.rs content-confidence multiply, report_weight tie-break, render/signals verdicts, restamp).
- [x] Core Rust: `buckets/gate.rs` content-confidence multiply and `RENAME_CONSISTENCY_DISCOUNT` deleted; routing reads `support = max(pair_agreement, pair_rename_consistency)` directly.
- [x] Core Rust: `report_weight.rs` fused tie-break deleted; equal mass sorts by cluster id ascending.
- [x] Core Rust: `cluster.rs` weight is the one mass formula; `log2(1 + spanned)` and refactor-potential node discount deleted.
- [x] Core Rust: `report_restamp.rs` no longer stamps `meets_fused_gate`; `render/signals.rs` verdict engine reads pair evidence, no fused column.
- [x] Rust E2E: golden bands/invariants migrated to final contract (bucket + pair evidence + ranking); `fused_score_bounds` deleted; `issue_343` asserts no wire fused + real bucket.

## Completion

This plan is complete when no public cluster model or surface contains `fused`, every rendered evidence value comes from one named admitted pair, rescue remains separate from the admission score, final ranking uses duplicated mass with an id-only tie-break, all affected tests assert the new observables without weakened coverage, and the generated wire models and public documentation agree with [fused.md](../specs/fused.md).
