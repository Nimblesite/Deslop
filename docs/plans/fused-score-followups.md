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

# TODO

## 1. Close the pre-content subsumption gap — #367, #408 — ✅ LANDED 2026-08-19

**Destructive cross-cluster subsumption ran before content measurement**
([#367](https://github.com/Nimblesite/Deslop/issues/367), [#408](https://github.com/Nimblesite/Deslop/issues/408)).
`build_ranked_fused_clusters` materialised clusters with `ContentEvidence::unmeasured()`, sorted them by
raw geometry, and called `collapse_cross_cluster_overlap`; only the survivors reached
`attach_content_evidence`. The final report reweighted with content-gated confidence, but it could not
recover a stronger view already deleted by subsumption. Fixed: content is attached before the collapse
and the survivor election reads it (details on the checklist below). #408's admission-level residue stays
open, pinned red by `type3_enclosing_method.rs`.

### The test that pins this section

`content_proven_nested_clone_survives_content_poor_enclosing_view` in
[`cross_cluster_collapse.rs`](../../crates/deslop/tests/cross_cluster_collapse.rs) — **green since the
§1 landing** — is the enforceable statement of the defect. Two TypeScript files wrap one byte-identical
five-line block in otherwise-divergent arithmetic. At `--min-nodes 8 --embeddings off` the report contains
exactly **one** cluster:

```
clusters_total = 1

id 33374cec477dea3e   bucket structural_only   size 2   canonical_node_count 86
occurrences   alpha.ts:1-17 (bytes 0..551),  beta.ts:1-17 (bytes 0..532)
signals       structural 1.00   token_jaccard 1.00   agreement 0.25
              rename_consistency 0.097   literal_fraction 0.25   fused 0.25
```

The byte-identical inner block never appeared at all. The enclosing whole-function view — the one the
content gate then demoted to `agreement 0.25` — deleted a clone whose **raw bytes are equal**, before any
content evidence existed to compare the two views. That was the false negative: the strongest available
evidence destroyed by the weakest available view.

The test requires the inner occurrence set to survive with `size = 2`, bucket `identical`, and
`structural = token_jaccard = agreement = fused = 1`. The split below landed it; no floor, threshold, or
assertion was moved to make it green.

### Two fixtures, one defect

`ts-mixed-band` (#367) and `csharp-type3` (#408) were the same defect. #408 was filed as an independent
five-language Type-3 recall hole. Traced, the enclosing clone was *built* and then thrown away:

```
DEBUG clustering by transitive closure candidate_pairs=92
DEBUG cross-cluster subsumption decision="drop_outer"
      survivor="c45b477b557d3686"  survivor_size=2  survivor_structural=1.0
      discarded="1999f08270059b2f" discarded_size=2 discarded_structural=0.0
INFO  bucket distribution visible=2 hidden=0 structural_only=2
```

The discarded cluster was the whole-method pair, `structural = 0.0` because one inserted statement
rehashes every ancestor Merkle. The survivor was a 13-node fragment nested inside it whose
`structural = 1.0` holds by construction: `precision_preference`
([`subsume.rs`](../../crates/deslop-core/src/cluster/subsume.rs)) let that overturn `strictly_encloses`,
and no content evidence existed yet to contradict it. The LSH-only floors were not involved — the two
`method_declaration` subtrees are 51 and 45 nodes, clear of `LSH_ONLY_MIN_NODE_COUNT = 40`.

`csharp-type3` was the cheaper regression fixture and is now green: the enclosing method pair renders
`nearly_identical, fused 0.9141` with every fragment absorbed. The other four languages proved *not* to
be this defect — their whole-method pairs are never admitted at all, so #408's remainder is an admission
problem, not a subsumption one (see the third checklist item).

### Constraint on the fix

Content evidence stays a **cluster** measurement; it does not move into pair admission. Measured on the
2026-08-18 repository run, 123,663 fingerprints produced 595,609 candidate pairs of which 11,868 survived
into 3,616 closure components, and the whole content-attachment interval cost about 134 ms. Admission
would instead ask for content on ~596,000 pairs, and the evidence includes cluster-level facts — the
canonical-member mean, the verbatim-member share — that would change meaning as well as cost.

- [x] Split materialisation from destructive cross-cluster subsumption: materialise closure components,
      attach content evidence, then choose the surviving view and perform the final report reweight.
      **Landed:** `build_ranked_fused_clusters` now attaches content between ranking and
      `collapse_cross_cluster_overlap`; the survivor election consults the renderer's own routing
      (`measured_kind`): a demoted view never deletes a credible one, and in the reverse direction a
      demoted encloser yields only to *verbatim-proven* nested duplication
      (`ContentEvidence::verbatim_dominated`) — so it still absorbs its own diluted sub-windows.
      Between two credible views enclosure stands untouched. Three sharper comparisons were built,
      measured, and removed: raw support (shattered `csharp-fact-cross-cluster`'s 0.8947 method pair
      into fragments), the rendered fused grade (read `RENAME_CONSISTENCY_DISCOUNT` as doubt and let
      `ts-type2-loop`'s proven rename pair be deleted by its own inner fragments), and an act-now
      support grade at 0.85 (elected a verbatim core over the credible 0.8 window enclosing it and
      orphaned that window's other absorbed views — `issue_343_sum_clamp_saturation` counted the
      orphan). The §-pin `content_proven_nested_clone_survives_content_poor_enclosing_view` is green.
- [x] Preserve `[PIPELINE-CLUSTER-SUBSUME]`'s file-coverage and enclosure guarantees for ties where
      content does not distinguish the views. **Landed:** within one credibility tier the
      ladder is unchanged; `cross_cluster_enclosure.rs` (all five) and
      `cluster_subsumption`/`cluster_overlap_collapse` stay green. Spec updated to name the two content
      gates.
- [x] Add the Type-3 fixtures as a black-box regression asserting the *enclosing* method pair is the
      visible cluster. Assert the occurrence set, not only the bucket — the fragments would satisfy a
      bucket-only assertion. **Landed:** `crates/deslop/tests/type3_enclosing_method.rs`.
      `csharp-type3` is green — the pair now renders `nearly_identical, fused 0.9141` with every
      fragment absorbed. `dart`/`go`/`python`/`ts-type3-stmt` stay **red on purpose**: traced, their
      whole-method pairs are never *admitted* (structural 0.0 after the insertion and a token overlap
      below `FUSED_THRESHOLD` on these shorter bodies; csharp clears it at ≈0.91), so no subsumption
      order can recover them — the residue of #408 named in the §2 table. Measured before the fix at
      `--min-nodes 8`:

      | fixture | visible | hidden | what is reported |
      |---|---|---|---|
      | `csharp-type3` | 2 | 0 | 13-node guard and loop-header fragments, both `structural_only`, `fused` 0.6667 / 0.5727 |
      | `dart-type3` | 2 | 1 | fragments, `structural_only`, `fused` 0.6000 / 0.5400 |
      | `go-type3` | 2 | 0 | fragments, `structural_only`, `fused` 0.6000 / 0.4500 |
      | `python-type3` | 1 | 0 | fragment, `structural_only`, `fused` 0.6000 |
      | `ts-type3-stmt` | **0** | 1 | nothing visible |

      In no language does the whole-method pair appear. `ts-type3-stmt` is the sharpest assertion
      available: one inserted statement takes the visible clone count from one to zero.

## 2. Content-gate recall defects — #409 ✅ landed, #410 open

- [x] **The 10 remaining softened files (7 families): adjudicated, and they pinned three distinct engine
      defects.** **Status (2026-08-19):** every family's fixture carries its maximal content on disk and
      every family is either green or pinned red by an active test — nothing remains softened. The four
      `literal_preservation` families (`ts-decorators`, `ts-enums`, `jsx-entity-invariance`,
      `ts-comment-literal-invariance`) went green with #409. `ts-qualified-type-rename` is pinned red by
      `typescript_qualified_type_name_rename_is_token_invariant` (#410, open). The `*-type3` families are
      pinned red by `type3_enclosing_method.rs`, whose assertions were re-aimed at the *enclosing* pair as
      this item required, and stay red on the #408 admission residue.

      The original adjudication, kept because it is what the fixes were measured against. Seven
      independent judges plus an adversarial reviewer read both sides of every pair and re-measured. The
      claim that `[REPAIR-RENAME-ANCHOR-MASS]` unblocked all 17 was half true — but the reason the
      remaining seven demoted was **not** that their renames are unprovable. Ranked by rename purity:

      | family | maximal `rename` | blocking term | verdict |
      |---|---|---|---|
      | `ts-qualified-type-rename` | 0.6667 | anchor mass alone — `coverage` and `literal_preservation` are both **1.0** | false negative ([#410](https://github.com/Nimblesite/Deslop/issues/410)) |
      | `ts-decorators` | 0.5467 | `literal_preservation` 3/5 — two strings that are the *same* substitutions | false negative ([#409](https://github.com/Nimblesite/Deslop/issues/409)) |
      | `ts-enums` | 0.5507 | `literal_preservation` 6/9 — three labels renamed with their members | false negative ([#409](https://github.com/Nimblesite/Deslop/issues/409)) |
      | `jsx-entity-invariance` | 0.3889 | `literal_preservation` 1/2 — the entity the fixture exists to prove invariant | false negative ([#409](https://github.com/Nimblesite/Deslop/issues/409)) |
      | `ts-comment-literal-invariance` | 0.2571 | `literal_preservation` 2/6 — two of them genuinely behavioural | weakest; pins the incoherence |
      | `javascript-type3` / `typescript-type3` | 0.4286 | not the content gate at all — shape identity broken by one inserted statement | [#408](https://github.com/Nimblesite/Deslop/issues/408) |

      Both caveats were honoured: the `*-type3` assertions were re-aimed at the enclosing pair before
      restoration (restored, the only cluster is an 8-node `if`-statement fragment, so the old
      `nearly_identical` assertion would have asserted something untrue), and no floor, threshold or
      assertion value was moved to make anything green.

      Quarantining the `literal_preservation` term with a `panic!` was explicitly **rejected** on review,
      and the landed fix confirms that call. The term is load-bearing precision machinery — measured, the
      #197 family already rendered three visible `nearly_identical` clusters carried by `agreement` with
      `rename_consistency` at 0.31–0.50, so removing it would have raised them rather than lowering them.
      The defect was the *blindness* of the count, not the term's existence, so it was a scoped design fix
      ([#409](https://github.com/Nimblesite/Deslop/issues/409)) rather than a quarantine.

- [x] **[#409](https://github.com/Nimblesite/Deslop/issues/409) is a recall defect, one-sided.** The issue
      originally claimed `literal_preservation`'s blind count was wrong in *both* directions. The precision
      half was re-measured and does not hold, so it is withdrawn: two files identical except `* 0.9` vs
      `* 0.75` render `nearly_identical` at `fused 0.9762`, and that is the **right** answer — 97.6%
      identical code differing by one constant is textbook parameterise-me duplication, and the rendered
      text already says "small differences may matter (Type-3 near-miss)". The audit's own control, a
      genuinely unrelated same-shape pair, measured `rename 0.000 / agreement 0.000 / fused 0.000` —
      correctly annihilated. Nothing shows unrelated code being promoted.

      What stood was the recall half, measured: a literal renamed *alongside the symbol it names* was
      counted as disproof of the rename. `"OrderService"`→`"UserService"` cost `ts-decorators` 0.368 of
      `rename_consistency` while costing `agreement` 0.045. That made the fix narrower and safer than
      first described — no precision case to trade against — and it was re-measured against
      `dart_issue_197_single_file_structural_only.rs` and the F# data-table corpus, because
      `literal_preservation` is what holds those families out of the act-now band.

      **Pinned** by `crates/deslop/tests/rename_literal_monotonicity.rs` (green since the fix) with two
      minimal fixtures differing by exactly one string literal — `ts-rename-literal-consistent` renames
      `"OrderService"` → `"UserService"` with its symbol, `ts-rename-literal-inconsistent` leaves it
      behind. The score was **inverted**, not merely low: the thorough rename rendered `structural_only,
      fused 0.3833, rename 0.4259`; the half-finished one rendered `nearly_identical, fused 0.7714, rename
      0.8571`. The load-bearing assertion is the monotonicity one — *a more complete rename can never be
      weaker evidence of a rename than a less complete one* — because it is true independently of where
      any floor sits and therefore cannot be satisfied by moving one.

      **Landed:** `content/rename.rs` now recognises a literal **echo** — a substituted literal position
      whose raw bytes transform into the partner's bytes exactly by one bijection-explained identifier
      substitution. An echo counts as a consistent literal, raises the anchor mass, and corroborates its
      substitution the way a repeated identifier occurrence would. Both monotonicity pins are green.
      Re-measured as this section requires: `dart_issue_197_single_file_structural_only` green (after the
      subsumption verbatim bar below), all three F# data-table/rename suites green,
      `type2_rename_anchor_floor` green, `fused_golden_bands` 7/7. Measured family outcomes at the same
      floors: `ts-decorators` `nearly_identical, fused 0.8864`; `ts-enums` `nearly_identical, rename
      0.913`; `jsx-entity-invariance` `nearly_identical, fused 0.875`; `ts-comment-literal-invariance`
      `nearly_identical, fused 0.9048`; `js-classes` promoted from the gate's blind demotion to
      `nearly_identical, rename 0.8519` and its assertion re-aimed to the accurate verdict. No floor,
      threshold, or existing assertion value was moved.

      The re-measure also surfaced a subsumption interaction: any sub-window of a demoted surface
      measures higher agreement than the surface itself, so the #197 sibling family resurfaced as a
      credible six-line window family the moment its demoted umbrella died. `precision_preference` now
      lets a demoted view yield only to *verbatim-proven* duplication
      (`ContentEvidence::verbatim_dominated`); narrowing a demoted surface cannot launder it into a
      finding, while byte-identical clones still overturn the umbrella that would bury them.

- [ ] **[#410](https://github.com/Nimblesite/Deslop/issues/410) is the gate's *other* term, and it is
      independent of #409.** `rename_consistency` is one product —
      `min(literal_preservation, coverage) * anchor_weight(anchors)` in
      [`content.rs`](../../crates/deslop-core/src/content.rs) — and the two issues are its two factors.
      Neither subsumes the other, measured: `ts-qualified-type-rename` scores `literal_preservation 1.0`
      and `coverage 1.0`, so #409's fix cannot move it; it demotes purely on
      `anchor_weight(8) = 8/(8+4) = 0.6667` against `CONTENT_SUPPORT_FLOOR` 0.7 — missing by 0.033 a
      bijection the engine's own coverage and literal terms certify as total. `ts-decorators` is the
      mirror: `literal_preservation 3/5` caps the product at 0.6 whatever the mass term does.

      The blocked-by edge is discharged: #409 landed first, because it changes #410's only input —
      `anchors = preserved_literal_count(literals) + mapping.explained`, so crediting a literal renamed
      alongside its symbol raises the anchor count as well as the numerator.

      **Re-measured after #409 (2026-08-19): unchanged, as predicted.** The fixture has no substituted
      literals, so no echo fires and the anchor set is identical; the whole-function pair still demotes on
      anchor mass alone and is then deleted in favour of its byte-identical tail fragment (rendered
      `identical`, bytes 70..141 / 67..138 — the post-annotation suffix). The pin is **active red**:
      `typescript_qualified_type_name_rename_is_token_invariant` fails against the maximal rename on disk.

      **What is left.** Eight anchors certified total by the engine's own `coverage 1.0` and
      `literal_preservation 1.0` must not weigh less than the floor they are being tested against. The
      open question is whether `RENAME_EVIDENCE_HALF_MASS` is the wrong shape (a mass term that can never
      reach a floor above `n/(n+4)` for small-but-total bijections) or whether a certified-total bijection
      should bypass the mass discount entirely. Whichever lands must be re-measured against the same
      precision set #409 was — `dart_issue_197`, the F# data-table corpus, `type2_rename_anchor_floor`,
      `fused_golden_bands` — and `CONTENT_SUPPORT_FLOOR` may not be lowered to close the 0.033 gap.

## 3. Re-verify the fused false positives

These reports predate `[FUSION-CONTENT-GATE]`; measure before changing code. None is closeable until the
corpus can express *"these two things are not duplicates"* — section A of
[`corpus-assertion.md`](corpus-assertion.md).

**Baseline measured 2026-08-19** while verifying §1/§2. Three of these pins are **red on `main` as well as
on this branch** — confirmed by running them against a pristine `HEAD` checkout of the engine files, so
they are standing defects, not regressions from the subsumption or rename work:
`python_issue_72_monkeypatch::monkeypatch_setenv_setup_pattern_is_not_duplicate_code`,
`python_dict_assert_payload_proof::a_call_inside_a_consumed_payload_value_is_not_excused`, and
`python_literal_variation_calls::rest_endpoint_family_with_fstring_paths_is_suppressed`. Each asserts a
*suppression*, so a red one means the false positive is live today. Neither §1 nor §2 moved any of them.

- [ ] **Assertion idioms:** #71, #103 and #285. Nearest pins:
      `python_issue_72_monkeypatch.rs` and the `python_dict_assert_*` suites — both red at baseline
      (above), so the idiom families are still being reported as duplicates.
- [ ] **Data-table/object-literal families:** #283 and #284. Recheck the language-agnostic data category
      shipped for #336 before treating them as open detector defects. `python_issue_133_constant_table`
      and `fsharp_issue_336_data_table_category` are green, so the data category itself is intact.
- [ ] **Helper call sites:** #79. Nearest pin: `python_literal_variation_calls.rs` — red at baseline, the
      f-string endpoint family is not being suppressed.
- [ ] **#362 / `[RANK-STRUCTURAL-ONLY]`:** two unrelated const-declaration files must not become the
      repository's largest ranked finding.

## 4. Fused close-outs

Deslop's agents never close issues (`CLAUDE.md`), so an item here is done when its evidence is recorded
and named; a human performs the close.

- [ ] **#339:** re-measure the F# token score on the curated corpus, then close only if the offset-invariant
      signature pins still agree. *Local suites green 2026-08-19 —
      `fsharp_issue_339_sibling_window_rename` (2) and `fsharp_issue_339_token_fallback_rename` (1). The
      curated-corpus run is what remains, and it needs `make test-corpus` clones this environment lacks.*
- [ ] **#343:** `pair_admission_bounded_max.rs` pins admission arithmetic; use the active
      `fused_golden_invariants.rs` assertions for rendered bounds before closing. *Evidence complete
      2026-08-19: `pair_admission_bounded_max` 3/3, `fused_golden_invariants` 2/2,
      `issue_343_sum_clamp_saturation` 3 passed + 1 pre-existing ignore. Ready for a human close.*
- [ ] **#345:** verify the public fusion docs against the tree, then close. *`fusion.md`'s
      `rename_consistency` definition and `pipeline.md`'s `[PIPELINE-CLUSTER-SUBSUME]` ladder were both
      brought back into agreement with the code by §1/§2; the rest of the doc set is unaudited.*
- [ ] **#336:** fixture-level saturation and data categorisation are pinned; the curated F# run remains.
      *`fsharp_issue_336_data_table_category` 4/4 green 2026-08-19; curated run still blocked on corpus
      clones.*
- [ ] **#331:** re-verify the real-repository claim through the repaired corpus assertion; reopen if it
      does not survive.
- [ ] **#347:** require three consecutive green corpus runs and name them when closing.
- [ ] **#355:** verify `dart_issue_197_single_file_structural_only` with no ignored assertions, then close.
      *Verified 2026-08-19: 1 passed, **0 ignored**, no `#[ignore]` anywhere in the file — and it was
      re-verified after the §2 subsumption change that briefly broke it, which is what motivated the
      verbatim-proof bar. Ready for a human close.*

## Order

```
§1  #367 pre-content subsumption ✅ ──► #408 five-language Type-3 recall (admission residue open)
§2  #409 literal echoes ✅ ──► #410 anchor mass (open) ──► restore the 10 softened fixtures ✅
§3  corpus-assertion.md A–E ──► the precision pins the false-positive re-measurement needs
```

**What is left, in order.** #410 (anchor mass) is the only open engine defect in §1–§2 and is unblocked —
#409 landed and did not move it. #408's remainder changed shape: it is no longer a subsumption problem
but a pair-*admission* one, since `dart`/`go`/`python`/`ts-type3-stmt` never admit their whole-method
pairs (structural 0.0 after the inserted statement, token overlap under `FUSED_THRESHOLD` on those
shorter bodies). It is pinned red by `type3_enclosing_method.rs` and belongs to whichever plan owns
candidate admission, not to this section's content gate.

§3 still cannot start until `corpus-assertion.md` section A lands, because none of
#71 / #79 / #103 / #283 / #284 / #285 / #362 is closeable while the corpus cannot assert *"these two
things are not duplicates"* — the same gap #401 reports.

---

# Ledger

Kept only for fused repair IDs cited from tests or specifications.

| ID | What it fixed | Held by |
|---|---|---|
| `[REPAIR-RENAME-ANCHOR-MASS]` (#405) | Replaced a four-literal cliff with smoothly weighted Baker-corroborated anchor mass | `type2_rename_anchor_floor.rs`, `fused_golden_bands.rs`, `js_language_features.rs`, `js_ts_clone_buckets.rs` |
| `[REPAIR-SUBSUME-CONTENT-FIRST]` (#367, #408) | Measured content before destructive cross-cluster subsumption, and made the survivor election read it: a demoted view never deletes a credible one, a demoted encloser yields only to verbatim-proven nesting, and between credible views enclosure stands | `cross_cluster_collapse.rs`, `type3_enclosing_method.rs`, `cluster/subsume.rs`, `[PIPELINE-CLUSTER-SUBSUME]` in `pipeline.md` |
| `[REPAIR-RENAME-LITERAL-ECHO]` (#409) | Counted a literal renamed alongside its symbol as consistent rename evidence instead of disproof, so a more complete rename can never score below a less complete one | `rename_literal_monotonicity.rs`, `js_language_features.rs`, `content/rename.rs`, `[FUSION-CONTENT-GATE]` in `fusion.md` |
