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

## 1. Close the pre-content subsumption gap — #367, #408

**Destructive cross-cluster subsumption runs before content measurement**
([#367](https://github.com/Nimblesite/Deslop/issues/367), [#408](https://github.com/Nimblesite/Deslop/issues/408)).
`build_ranked_fused_clusters` materialises clusters with `ContentEvidence::unmeasured()`, sorts them by
raw geometry, and calls `collapse_cross_cluster_overlap`. Only the survivors reach
`attach_content_evidence` in [`session/render.rs`](../../crates/deslop-core/src/pipeline/session/render.rs).
The final report does reweight and sort with content-gated confidence, but it cannot recover a stronger
view already deleted by subsumption.

### The red test that pins this section

`content_proven_nested_clone_survives_content_poor_enclosing_view` in
[`cross_cluster_collapse.rs`](../../crates/deslop/tests/cross_cluster_collapse.rs) **fails on `make test`
today**, and is the enforceable statement of the defect. Two TypeScript files wrap one byte-identical
five-line block in otherwise-divergent arithmetic. At `--min-nodes 8 --embeddings off` the report contains
exactly **one** cluster:

```
clusters_total = 1

id 33374cec477dea3e   bucket structural_only   size 2   canonical_node_count 86
occurrences   alpha.ts:1-17 (bytes 0..551),  beta.ts:1-17 (bytes 0..532)
signals       structural 1.00   token_jaccard 1.00   agreement 0.25
              rename_consistency 0.097   literal_fraction 0.25   fused 0.25
```

The byte-identical inner block never appears at all. The enclosing whole-function view — the one the
content gate then demotes to `agreement 0.25` — deleted a clone whose **raw bytes are equal**, before any
content evidence existed to compare the two views. That is the false negative: the strongest available
evidence is destroyed by the weakest available view.

The test requires the inner occurrence set to survive with `size = 2`, bucket `identical`, and
`structural = token_jaccard = agreement = fused = 1`. It stays red until the split below lands; no floor,
threshold, or assertion may be moved to make it green.

### Two fixtures, one defect

`ts-mixed-band` (#367) and `csharp-type3` (#408) are the same defect. #408 was filed as an independent
five-language Type-3 recall hole. Traced, the enclosing clone is *built* and then thrown away:

```
DEBUG clustering by transitive closure candidate_pairs=92
DEBUG cross-cluster subsumption decision="drop_outer"
      survivor="c45b477b557d3686"  survivor_size=2  survivor_structural=1.0
      discarded="1999f08270059b2f" discarded_size=2 discarded_structural=0.0
INFO  bucket distribution visible=2 hidden=0 structural_only=2
```

The discarded cluster is the whole-method pair, `structural = 0.0` because one inserted statement rehashes
every ancestor Merkle. The survivor is a 13-node fragment nested inside it whose `structural = 1.0` holds
by construction: `precision_preference` ([`subsume.rs`](../../crates/deslop-core/src/cluster/subsume.rs))
lets that overturn `strictly_encloses`, and no content evidence exists yet to contradict it. Delete only
the inserted statement and the same fixture renders one `nearly_identical` cluster over both files at
`fused 0.7560` carried by `rename_consistency 0.84`, with both fragments gone. The enclosing view is
reachable; subsumption removes it. The LSH-only floors are not involved — the two `method_declaration`
subtrees are 51 and 45 nodes, clear of `LSH_ONLY_MIN_NODE_COUNT = 40`.

So #408 is blocked by this section rather than being work of its own, and `csharp-type3` is the cheaper of
the two regression fixtures.

### Constraint on the fix

Content evidence stays a **cluster** measurement; it does not move into pair admission. Measured on the
2026-08-18 repository run, 123,663 fingerprints produced 595,609 candidate pairs of which 11,868 survived
into 3,616 closure components, and the whole content-attachment interval cost about 134 ms. Admission
would instead ask for content on ~596,000 pairs, and the evidence includes cluster-level facts — the
canonical-member mean, the verbatim-member share — that would change meaning as well as cost.

- [ ] Split materialisation from destructive cross-cluster subsumption: materialise closure components,
      attach content evidence, then choose the surviving view and perform the final report reweight.
- [ ] Preserve `[PIPELINE-CLUSTER-SUBSUME]`'s file-coverage and enclosure guarantees for ties where
      content does not distinguish the views.
- [ ] Add the Type-3 fixtures as a black-box regression asserting the *enclosing* method pair is the
      visible cluster. Assert the occurrence set, not only the bucket — the fragments would satisfy a
      bucket-only assertion. Measured today at `--min-nodes 8`:

      | fixture | visible | hidden | what is reported |
      |---|---|---|---|
      | `csharp-type3` | 2 | 0 | 13-node guard and loop-header fragments, both `structural_only`, `fused` 0.6667 / 0.5727 |
      | `dart-type3` | 2 | 1 | fragments, `structural_only`, `fused` 0.6000 / 0.5400 |
      | `go-type3` | 2 | 0 | fragments, `structural_only`, `fused` 0.6000 / 0.4500 |
      | `python-type3` | 1 | 0 | fragment, `structural_only`, `fused` 0.6000 |
      | `ts-type3-stmt` | **0** | 1 | nothing visible |

      In no language does the whole-method pair appear. `ts-type3-stmt` is the sharpest assertion
      available: one inserted statement takes the visible clone count from one to zero.

## 2. Content-gate recall defects — #409, #410, and the 10 softened fixtures

- [ ] **The 10 remaining softened files (7 families): adjudicated, and they pin three distinct engine
      defects.** Seven independent judges plus an adversarial reviewer read both sides of every pair and
      re-measured. The claim that `[REPAIR-RENAME-ANCHOR-MASS]` unblocked all 17 is half true — but the
      reason the remaining seven demote is **not** that their renames are unprovable. Ranked by rename
      purity:

      | family | maximal `rename` | blocking term | verdict |
      |---|---|---|---|
      | `ts-qualified-type-rename` | 0.6667 | anchor mass alone — `coverage` and `literal_preservation` are both **1.0** | false negative ([#410](https://github.com/Nimblesite/Deslop/issues/410)) |
      | `ts-decorators` | 0.5467 | `literal_preservation` 3/5 — two strings that are the *same* substitutions | false negative ([#409](https://github.com/Nimblesite/Deslop/issues/409)) |
      | `ts-enums` | 0.5507 | `literal_preservation` 6/9 — three labels renamed with their members | false negative ([#409](https://github.com/Nimblesite/Deslop/issues/409)) |
      | `jsx-entity-invariance` | 0.3889 | `literal_preservation` 1/2 — the entity the fixture exists to prove invariant | false negative ([#409](https://github.com/Nimblesite/Deslop/issues/409)) |
      | `ts-comment-literal-invariance` | 0.2571 | `literal_preservation` 2/6 — two of them genuinely behavioural | weakest; pins the incoherence |
      | `javascript-type3` / `typescript-type3` | 0.4286 | not the content gate at all — shape identity broken by one inserted statement | [#408](https://github.com/Nimblesite/Deslop/issues/408) |

      **Restoring them is the right call, and it produces red tests, which CLAUDE.md calls a correct
      outcome.** Two caveats before doing it: the two `*-type3` assertions must be **re-aimed** first —
      restored, the only cluster is an 8-node `if`-statement fragment, so the current `nearly_identical`
      assertion would assert something untrue; and no floor, threshold or assertion may be moved to make
      any of them green.

      Quarantining `content.rs:321` with a `panic!` was explicitly **rejected** on review. That line is
      load-bearing precision machinery — measured, the #197 family already renders three visible
      `nearly_identical` clusters carried by `agreement` with `rename_consistency` at 0.31–0.50, so
      removing the literal term raises them rather than lowering them. The defect is the *blindness* of the
      count, not the term's existence, so it is a scoped design fix
      ([#409](https://github.com/Nimblesite/Deslop/issues/409)) rather than a quarantine.

- [ ] **[#409](https://github.com/Nimblesite/Deslop/issues/409) is a recall defect, one-sided.** The issue
      originally claimed `literal_preservation`'s blind count was wrong in *both* directions. The precision
      half was re-measured and does not hold, so it is withdrawn: two files identical except `* 0.9` vs
      `* 0.75` render `nearly_identical` at `fused 0.9762`, and that is the **right** answer — 97.6%
      identical code differing by one constant is textbook parameterise-me duplication, and the rendered
      text already says "small differences may matter (Type-3 near-miss)". The audit's own control, a
      genuinely unrelated same-shape pair, measured `rename 0.000 / agreement 0.000 / fused 0.000` —
      correctly annihilated. Nothing shows unrelated code being promoted.

      What stands is the recall half, measured: a literal renamed *alongside the symbol it names* is
      counted as disproof of the rename. `"OrderService"`→`"UserService"` costs `ts-decorators` 0.368 of
      `rename_consistency` while costing `agreement` 0.045. That makes the fix narrower and safer than
      first described — there is no precision case to trade against — but it must still be re-measured
      against `dart_issue_197_single_file_structural_only.rs` and the F# data-table corpus, because
      `literal_preservation` is what holds those families out of the act-now band.

      **Pinned** by `crates/deslop/tests/rename_literal_monotonicity.rs` (red on purpose) with two minimal
      fixtures differing by exactly one string literal — `ts-rename-literal-consistent` renames
      `"OrderService"` → `"UserService"` with its symbol, `ts-rename-literal-inconsistent` leaves it
      behind. The score is **inverted**, not merely low: the thorough rename renders `structural_only,
      fused 0.3833, rename 0.4259`; the half-finished one renders `nearly_identical, fused 0.7714, rename
      0.8571`. Finishing a rename drops the pair below the reuse line, so the tool advises worse the more
      carefully the developer renamed. The load-bearing assertion is the monotonicity one — *a more
      complete rename can never be weaker evidence of a rename than a less complete one* — because it is
      true independently of where any floor sits and therefore cannot be satisfied by moving one. This
      single test replaces restoring five softened fixtures that would all have pinned the same root cause.

- [ ] **[#410](https://github.com/Nimblesite/Deslop/issues/410) is the gate's *other* term, and it is
      independent of #409.** `rename_consistency` is one product —
      `min(literal_preservation, coverage) * anchor_weight(anchors)` in
      [`content.rs`](../../crates/deslop-core/src/content.rs) — and the two issues are its two factors.
      Neither subsumes the other, measured: `ts-qualified-type-rename` scores `literal_preservation 1.0`
      and `coverage 1.0`, so #409's fix cannot move it; it demotes purely on
      `anchor_weight(8) = 8/(8+4) = 0.6667` against `CONTENT_SUPPORT_FLOOR` 0.7 — missing by 0.033 a
      bijection the engine's own coverage and literal terms certify as total. `ts-decorators` is the
      mirror: `literal_preservation 3/5` caps the product at 0.6 whatever the mass term does.

      Fix #409 first, then re-measure #410, because #409 changes #410's only input —
      `anchors = preserved_literal_count(literals) + mapping.explained`, so crediting a literal renamed
      alongside its symbol raises the anchor count as well as the numerator. Recorded as a blocked-by edge
      on the issues.

      **No test pins #410.** Restoring `ts-qualified-type-rename` is what would:
      `typescript_features.rs:104` (`typescript_qualified_type_name_rename_is_token_invariant`) asserts
      `nearly_identical` and goes red the moment the maximal rename comes back. That restoration belongs
      with the ten files above, not before them.

## 3. Re-verify the fused false positives

These reports predate `[FUSION-CONTENT-GATE]`; measure before changing code. None is closeable until the
corpus can express *"these two things are not duplicates"* — section A of
[`corpus-assertion.md`](corpus-assertion.md).

- [ ] **Assertion idioms:** #71, #103 and #285. Nearest pins:
      `python_issue_72_monkeypatch.rs` and the `python_dict_assert_*` suites.
- [ ] **Data-table/object-literal families:** #283 and #284. Recheck the language-agnostic data category
      shipped for #336 before treating them as open detector defects.
- [ ] **Helper call sites:** #79. Nearest pin: `python_literal_variation_calls.rs`.
- [ ] **#362 / `[RANK-STRUCTURAL-ONLY]`:** two unrelated const-declaration files must not become the
      repository's largest ranked finding.

## 4. Fused close-outs

- [ ] **#339:** re-measure the F# token score on the curated corpus, then close only if the offset-invariant
      signature pins still agree.
- [ ] **#343:** `pair_admission_bounded_max.rs` pins admission arithmetic; use the active
      `fused_golden_invariants.rs` assertions for rendered bounds before closing.
- [ ] **#345:** verify the public fusion docs against the tree, then close.
- [ ] **#336:** fixture-level saturation and data categorisation are pinned; the curated F# run remains.
- [ ] **#331:** re-verify the real-repository claim through the repaired corpus assertion; reopen if it
      does not survive.
- [ ] **#347:** require three consecutive green corpus runs and name them when closing.
- [ ] **#355:** verify `dart_issue_197_single_file_structural_only` with no ignored assertions, then close.

## Order

```
§1  #367 pre-content subsumption ──► #408 five-language Type-3 recall
§2  #409 literal_preservation ──► #410 anchor mass ──► restore the 10 softened fixtures
§3  corpus-assertion.md A–E ──► the precision pins the false-positive re-measurement needs
```

§1 and §2 are independent and can run in parallel. §3 cannot start until `corpus-assertion.md` section A
lands, because none of #71 / #79 / #103 / #283 / #284 / #285 / #362 is closeable while the corpus cannot
assert *"these two things are not duplicates"* — the same gap #401 reports.

---

# Ledger

Kept only for fused repair IDs cited from tests or specifications.

| ID | What it fixed | Held by |
|---|---|---|
| `[REPAIR-RENAME-ANCHOR-MASS]` (#405) | Replaced a four-literal cliff with smoothly weighted Baker-corroborated anchor mass | `type2_rename_anchor_floor.rs`, `fused_golden_bands.rs`, `js_language_features.rs`, `js_ts_clone_buckets.rs` |
