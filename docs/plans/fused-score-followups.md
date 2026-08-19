# Fused confidence — what is left

**Scope:** only work that changes fused admission, measured cluster confidence, content gating, bucket
routing, or confidence-aware ranking belongs here. Candidate generation, cache mechanics, watcher state,
CI maintenance, and repository-wide metrics have their own plans. A candidate-route problem belongs here
only when two runs produce the same final occurrence set but assign it different measured confidence.

The shipped contract is `[FUSION-STRATEGY-BOUNDED-MAX]`, `[FUSION-CLUSTER-SIGNALS]` and
`[FUSION-CONTENT-GATE]` in [`fusion.md`](../specs/fusion.md). The real-repository precision gate is planned
separately in [`corpus-assertion.md`](corpus-assertion.md).

## The one measure

Every reported cluster is a real duplicate, and every real duplicate is reported. Order open work by how
much it moves that number.

## The contract

`fused` must **carry information**: the three agent bands in `CLAUDE.md` (`>= 0.85` do not write the
copy, `0.6..0.85` read the canonical occurrence and bias to reuse, `< 0.6` author it) must all be
reachable, and must mean the same thing in every language. `fused_golden_bands.rs` cites this paragraph;
do not weaken it without moving that suite with it.

---

# Open work

## #410 — anchor mass demotes a bijection the engine certifies as total

The only open engine defect in this plan, and unblocked.

`rename_consistency = min(literal_preservation, coverage) * anchor_weight(anchors)`.
[`ts-qualified-type-rename`](../../crates/deslop/tests/fixtures/ts-qualified-type-rename) measures `literal_preservation 1.0` and
`coverage 1.0` — the engine's own terms certify the bijection as **total** — and demotes anyway, purely on
`anchor_weight(8) = 8/(8+4) = 0.6667` against `CONTENT_SUPPORT_FLOOR = 0.7`. It misses by 0.033.

Pinned by `typescript_qualified_type_name_rename_is_token_invariant` against the maximal rename on disk.
**Green as of 2026-08-19** (`typescript_features` 7/7): the whole-function pair survives instead of being
deleted in favour of its byte-identical tail fragment, because content evidence is now attached before
cross-cluster subsumption elects a survivor ([REPAIR-SUBSUME-CONTENT-FIRST]). The mass question below is
therefore open on its own terms, not on a red pin.

#410 was blocked by #409 because #409 changes its only input
(`anchors = preserved_literal_count(literals) + mapping.explained`). That edge is discharged: re-measured
after #409 landed, #410 is **unchanged, as predicted** — the fixture has no substituted literals, so no
echo fires and the anchor set is identical.

**The open question.** Whether `RENAME_EVIDENCE_HALF_MASS` is the wrong shape — a mass term that can never
reach a floor above `n/(n+4)` for small-but-total bijections — or whether a certified-total bijection
should bypass the mass discount entirely.

**Constraints on the fix.** Re-measure against the same precision set #409 was measured against:
`dart_issue_197`, the F# data-table corpus, `type2_rename_anchor_floor`, `fused_golden_bands`.
`CONTENT_SUPPORT_FLOOR` may **not** be lowered to close the 0.033 gap.

## #408 residue — an admission defect, not a gate defect

#408 was filed as a five-language Type-3 recall hole and tracked here as a subsumption problem. `csharp-type3`
was, and is fixed. The other four are not this plan's defect: traced, their whole-method pairs are never
*admitted* — `structural 0.0` after the inserted statement, and token overlap below `FUSED_THRESHOLD` on
those shorter bodies, where csharp clears it at ≈0.91. No subsumption order can recover a pair that was
never built.

Pinned red by [`type3_enclosing_method.rs`](../../crates/deslop/tests/type3_enclosing_method.rs) — `dart`,
`go`, `python`, `ts-type3-stmt`. `ts-type3-stmt` is the sharpest of them: one inserted statement takes the
visible clone count from one to zero.

Tracked here only until whichever plan owns candidate admission takes it. Content evidence must **not**
move into pair admission to close this: it is a cluster measurement, and cluster-level facts it depends on
(the canonical-member mean, the verbatim-member share) would change meaning as well as cost. Measured on
the 2026-08-18 repository run, 123,663 fingerprints produced 595,609 candidate pairs of which 11,868
survived into 3,616 closure components; content attachment cost ≈134 ms on the components and would be
asked of ~596,000 pairs at admission.

## Re-verify the fused false positives

These reports predate `[FUSION-CONTENT-GATE]`; measure before changing code. None is closeable until the
corpus can express *"these two things are not duplicates"* — section A of
[`corpus-assertion.md`](corpus-assertion.md), the same gap #401 reports.

**Re-measured 2026-08-19, after the `verbatim_dominated` repair.** The three suppression pins that were
red are now green: `python_issue_72_monkeypatch` (1), `python_dict_assert_payload_proof` (4) and
`python_literal_variation_calls` (2). Each asserts a *suppression*, so green means those false positives
are no longer live. They were red because `verbatim_dominated` certified non-verbatim members as verbatim
and forced `agreement` to 1.0; the fix requires one token-identical family — equal shape digest *and*
equal collapsed-leaf keys — to hold a strict majority.

- **Assertion idioms** (#71, #103, #285) — `python_issue_72_monkeypatch.rs` and the `python_dict_assert_*`
  suites are green; the idiom families are suppressed.
- **Data-table / object-literal families** (#283, #284) — recheck the language-agnostic data category
  shipped for #336 before treating these as open detector defects. `python_issue_133_constant_table` and
  `fsharp_issue_336_data_table_category` are green, so the category itself is intact.
- **Helper call sites** (#79) — `python_literal_variation_calls.rs` is green; the f-string endpoint family
  is suppressed.
- **#362 / `[RANK-STRUCTURAL-ONLY]`** — two unrelated const-declaration files must not become the
  repository's largest ranked finding.

## Fused close-outs

Deslop's agents never close issues (`CLAUDE.md`), so an item here is done when its evidence is **recorded
and named**; a human performs the close.

| issue | what remains |
|---|---|
| #343 | nothing — evidence complete 2026-08-19: `pair_admission_bounded_max` 3/3, `fused_golden_invariants` 2/2, `issue_343_sum_clamp_saturation` 3 passed + 1 pre-existing ignore |
| #355 | nothing — verified 2026-08-19: `dart_issue_197_single_file_structural_only` 1 passed, **0 ignored**, no `#[ignore]` in the file, re-verified after the subsumption change that briefly broke it |
| #339 | the curated-corpus F# token re-measure. Local suites green 2026-08-19 — `fsharp_issue_339_sibling_window_rename` (2), `fsharp_issue_339_token_fallback_rename` (1) |
| #336 | the curated F# run. `fsharp_issue_336_data_table_category` 4/4 green 2026-08-19 |
| #345 | audit the rest of the public fusion doc set. `fusion.md`'s `rename_consistency` definition and `pipeline.md`'s `[PIPELINE-CLUSTER-SUBSUME]` ladder are back in agreement with the code |
| #331 | re-verify the real-repository claim through the repaired corpus assertion; reopen if it does not survive |
| #347 | three consecutive green corpus runs, named when closing |

#339, #336, #331 and #347 all need `make test-corpus` clones this environment lacks.

---

# Checklist

## Engine defects — unblocked

- [ ] **#410** — decide `RENAME_EVIDENCE_HALF_MASS`'s shape vs. a certified-total bypass; re-measure against
      `dart_issue_197`, the F# data-table corpus, `type2_rename_anchor_floor`, `fused_golden_bands`; do not
      lower `CONTENT_SUPPORT_FLOOR`. No longer pinned red — the reported pin is green as of 2026-08-19.

## Engine defects — owned elsewhere

- [ ] **#408 residue** — four languages' whole-method Type-3 pairs are never admitted. Hand to the plan
      that owns candidate admission; keep `type3_enclosing_method.rs` red until it lands.

## Blocked on `corpus-assertion.md` section A

- [ ] **#71 / #103 / #285** — assertion idioms.
- [ ] **#79** — helper call sites.
- [ ] **#283 / #284** — data-table / object-literal families.
- [ ] **#362** — `[RANK-STRUCTURAL-ONLY]`; unrelated const declarations as the largest ranked finding.

## Close-outs — evidence recorded, a human closes

- [x] **#343** — evidence complete.
- [x] **#355** — evidence complete.
- [ ] **#345** — audit the remaining public fusion docs.
- [ ] **#339** — curated-corpus F# token re-measure.
- [ ] **#336** — curated F# run.
- [ ] **#331** — re-verify through the repaired corpus assertion.
- [ ] **#347** — three consecutive green corpus runs.

---

# Ledger

Kept only for fused repair IDs cited from tests or specifications.

| ID | What it fixed | Held by |
|---|---|---|
| `[REPAIR-RENAME-ANCHOR-MASS]` (#405) | Replaced a four-literal cliff with smoothly weighted Baker-corroborated anchor mass | `type2_rename_anchor_floor.rs`, `fused_golden_bands.rs`, `js_language_features.rs`, `js_ts_clone_buckets.rs`, `common/signals.rs`, `taxonomy.md` |
| `[REPAIR-SUBSUME-CONTENT-FIRST]` (#367, #408) | Measured content before destructive cross-cluster subsumption, and made the survivor election read it: a demoted view never deletes a credible one, a demoted encloser yields only to verbatim-proven nesting, and between credible views enclosure stands | `cross_cluster_collapse.rs`, `type3_enclosing_method.rs`, `cluster/subsume.rs`, `[PIPELINE-CLUSTER-SUBSUME]` in `pipeline.md` |
| `[REPAIR-RENAME-LITERAL-ECHO]` (#409) | Counted a literal renamed alongside its symbol as consistent rename evidence instead of disproof, so a more complete rename can never score below a less complete one | `rename_literal_monotonicity.rs`, `js_language_features.rs`, `content/rename.rs`, `[FUSION-CONTENT-GATE]` in `fusion.md` |
