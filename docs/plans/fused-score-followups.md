# Fused confidence — what is left

One document. It replaces `root-cause-fusion.md`, `quarantine-repair-plan.md` and
`worktree-fused-score-followups-regression-audit.md`, all three deleted.

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

## Established baseline

| Property | Held by |
|---|---|
| Fusion is the strongest single axis, never the sum, at admission | `pair_admission_bounded_max.rs`, `fused_golden_invariants.rs` |
| Rendered signals are measured between the occurrences the report shows, never averaged over discovery edges | `cluster::signals::measured_signals`, `[FUSION-CLUSTER-SIGNALS]` |
| Shape-saturating clusters are re-scored against measured content evidence | `buckets::content_gated_signals`, `[FUSION-CONTENT-GATE]` |
| All three agent bands are reachable in six languages | `fused_golden_bands.rs` — verbatim, maximal rename, lean maximal rename and shape-only cases |
| Rename evidence is Baker-corroborated anchor mass, never a literal-count cliff | `type2_rename_anchor_floor.rs`, `js_ts_clone_buckets.rs`, the `rename_lean` scenarios and `[TECH-PMATCH-BAKER]` |
| Content evidence tests each byte position once through the collapsed frontier | `tokens::collapsed_leaves`, the template-literal and optional-chaining cases in `js_language_features.rs` |
| Every rendered component stays in `[0,1]`; only byte-proven duplication saturates | `fused_golden_invariants.rs`, swept over 21 corpora |

## The remaining fused gap

**Destructive cross-cluster subsumption runs before content measurement.**
`build_ranked_fused_clusters` materialises clusters with
`ContentEvidence::unmeasured()`, sorts them by raw geometry, and calls
`collapse_cross_cluster_overlap`. Only the survivors reach `attach_content_evidence` in
[`session/render.rs`](../../crates/deslop-core/src/pipeline/session/render.rs). The final report does
reweight and sort with content-gated confidence, but it cannot recover a stronger view already deleted
by subsumption.

The #344 reader gap is closed. Every decision surface — HTML, Markdown, CLI text, the VSIX cluster panel,
LSP diagnostics, code lenses, and the refactor preconditions — now shows or consumes the same seven-field
confidence explanation, through one renderer per language runtime. Section 2 records what each got.

---

# TODO

## 1. Close the pre-content subsumption gap

### Measured decision: do not move cluster evidence into pair admission

On the 2026-08-18 repository run:

- 123,663 fingerprints produced 595,609 candidate pairs;
- 11,868 pairs survived into 3,616 closure components;
- ranking/subsumption left 1,349 clusters;
- the interval from `ranked clusters built` to `render complete`, which contains content attachment, was
  about 134 ms.

The current pass is cheap because it measures cluster members after closure. Admission would instead ask
for content on nearly 596,000 candidate pairs. Caching leaf keys could avoid repeated tree walks, but it
would not remove the pair comparisons, and the existing evidence includes cluster-level facts such as the
canonical-member mean and verbatim-member share. Treating those as pair evidence would change their
meaning as well as their cost.

- [x] Keep pair admission on the bounded score axes. Content evidence remains a cluster measurement.
- [ ] Split materialisation from destructive cross-cluster subsumption: materialise closure components,
      attach content evidence, then choose the surviving view and perform the final report reweight.
- [ ] Add a black-box regression with two low-agreement enclosing views over a byte-identical inner clone.
      The identical inner occurrence set must survive with `agreement = 1`, `fused = 1` and bucket
      `identical`; the enclosing shape-only view must not delete it before measurement.
- [ ] Preserve `[PIPELINE-CLUSTER-SUBSUME]`'s file-coverage and enclosure guarantees for ties where
      content does not distinguish the views.

## 2. Finish #344 — one confidence explanation on every decision surface

Every surface is landed. Two renderers hold the whole field list — one per language runtime:
[`render/signals.rs`](../../crates/deslop-core/src/render/signals.rs) for the CLI, Markdown, HTML and LSP
surfaces, and [`src/types/signals.ts`](../../clients/vscode/src/types/signals.ts) for the VS Code ones. No
surface restates the fields, so they cannot drift.

Putting the evidence on the wire also paid for itself in the test vocabulary. `assert_structural_only_contract`
(`crates/deslop/tests/common/signals.rs`) previously stood in a single blanket bound —
`embedding_cos < STRUCTURAL_ONLY_MAX_SUPPORT` — for **both** routes into the bucket, because its own doc
recorded that "no helper reading three signals can reconstruct which route ran". That bound belongs only to
the evidence-free route; the content-gated route demotes on content and may legitimately hold any cosine
short of `EMBEDDING_SUPPORT_FLOOR`, so the assertion claimed a property the engine never promised and would
have fired spuriously the moment anyone asserted the contract with embeddings on. Each door is now checked by
its own entry condition against the measured `agreement` / `rename_consistency`, which is strictly *more*
assertive: the content-gated branch now proves the gate actually refused. Mutation-verified — disabling that
branch reddens `js-classes` (support 0.18) and `js-async` (support 0.29).

- [x] **VSIX `SignalStrip` and `HelpBubble` render all seven fields.** The four confidence axes, then a
      helped `CONTENT EVIDENCE` heading over `agreement` / `rename` / `literal`, then one plain-English
      reading of the shape score against that evidence — grounded only in the numbers already on screen,
      never re-deriving the engine's bucket. `signal-evidence.unit.test.ts` asserts the exact rendered
      sentence for four families and then parses both components with the TypeScript AST to prove neither
      reaches into a signal field itself, which is what a second formatter would look like. Nine tests, and
      the spec caught up: [webview-runtime.md §VSIX-WEBVIEW](../specs/webview-runtime.md) still said "four
      tiny bars".
- [x] **LSP diagnostics and code lenses carry the whole explanation.** Both now end in
      `render::signals::plain_explanation` — all seven fields, markup-free, one rendering shared with the
      report, because a client renders a diagnostic message and a lens title verbatim and a Markdown code
      span would show as a literal backtick. Pinned black-box by `editor_non_interference.rs`, which spells
      the expected string out itself instead of borrowing the renderer, so a surface that quietly drops the
      fused score or a content field fails the test rather than agreeing with itself. Six tests green;
      `lsp.md` [LSP-DIAGNOSTICS] / [LSP-CODE-LENS] say the same.
- [x] **Refactor preconditions consume the measured evidence, not just the bucket.**
      `preconditions::content_refusal` refuses any cluster whose shape saturates while
      `max(agreement, rename_consistency)` sits below the content floor, and quotes the shared explanation as
      its reason, so a user comparing an LSP refusal against the report reads the same numbers said the same
      way. It only ever refuses *more*: two exemptions stand where stronger proof already does — `Identical`,
      awarded on raw-source byte equality, and the anchor-free row-4 near-miss, whose members do not align
      position for position so neither content population can evaluate them. Pinned in both directions by
      `refactor_content_gate.rs` (6 tests) — `no_genuine_clone_fixture_is_convicted_by_the_content_gate` is
      the false-negative half.
- [x] **7 of the 17 restored** (`a3fe320be^` content): `js-generators`, `js-structural-control`,
      `js-tagged-templates`, `jsx-tsx-components`, `ts-generics`, `tsx-small`, `typescript-small`. All 36
      tests across the six suites that use them stay green, so this is pure strengthening: they now prove
      the engine promotes a **maximal** rename on Baker-corroborated anchor mass rather than proving two
      near-identical files match. `typescript-small` went from a two-identifier difference to a total
      bijection — `nearly_identical, fused 0.81` carried by `rename_consistency 0.90` while `agreement` is
      only 0.35 — and `js-generators` now promotes at `agreement 0.07`. Those numbers are legible only
      because #344 put the evidence on the wire.
- [ ] **The other 10 files (7 families): adjudicated, and they pin three distinct engine defects.** Seven
      independent judges plus an adversarial reviewer read both sides of every pair and re-measured. The
      claim that `[REPAIR-RENAME-ANCHOR-MASS]` unblocked all 17 is half true — but the reason the remaining
      seven demote is **not** that their renames are unprovable. Ranked by rename purity:

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
      assertion would assert something untrue; and no floor, threshold or assertion may be moved to make any
      of them green.

      The reviewer explicitly **rejected** quarantining `content.rs:321` with a `panic!`. That line is
      load-bearing precision machinery — measured, the #197 family already renders three visible
      `nearly_identical` clusters carried by `agreement` with `rename_consistency` at 0.31–0.50, so removing
      the literal term raises them rather than lowering them. The defect is the *blindness* of the count,
      not the term's existence, so it is a scoped design fix ([#409](https://github.com/Nimblesite/Deslop/issues/409)) rather than a quarantine.

- [ ] **[#409](https://github.com/Nimblesite/Deslop/issues/409) is a recall defect, one-sided.** The issue
      originally claimed `literal_preservation`'s blind count was wrong in *both* directions. I re-measured
      the precision half myself and it does not hold, so it is withdrawn: two files identical except
      `* 0.9` vs `* 0.75` render `nearly_identical` at `fused 0.9762`, and that is the **right** answer —
      97.6% identical code differing by one constant is textbook parameterise-me duplication, and the
      rendered text already says "small differences may matter (Type-3 near-miss)". The audit's own control,
      a genuinely unrelated same-shape pair, measured `rename 0.000 / agreement 0.000 / fused 0.000` —
      correctly annihilated. Nothing shows unrelated code being promoted.

      What stands is the recall half, measured: a literal renamed *alongside the symbol it names* is counted
      as disproof of the rename. `"OrderService"`→`"UserService"` costs `ts-decorators` 0.368 of
      `rename_consistency` while costing `agreement` 0.045. That makes the fix narrower and safer than first
      described — there is no precision case to trade against — but it must still be re-measured against
      `dart_issue_197_single_file_structural_only.rs` and the F# data-table corpus, because
      `literal_preservation` is what holds those families out of the act-now band.

      **Pinned** by `crates/deslop/tests/rename_literal_monotonicity.rs` (red on purpose) with two minimal
      fixtures differing by exactly one string literal — `ts-rename-literal-consistent` renames
      `"OrderService"` → `"UserService"` with its symbol, `ts-rename-literal-inconsistent` leaves it behind.
      The score is **inverted**, not merely low: the thorough rename renders `structural_only, fused 0.3833,
      rename 0.4259`; the half-finished one renders `nearly_identical, fused 0.7714, rename 0.8571`. Finishing
      a rename drops the pair below the reuse line, so the tool advises worse the more carefully the developer
      renamed. The load-bearing assertion is the monotonicity one — *a more complete rename can never be
      weaker evidence of a rename than a less complete one* — because it is true independently of where any
      floor sits and therefore cannot be satisfied by moving one. This single test replaces restoring five
      softened fixtures that would all have pinned the same root cause.

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
pre-content subsumption regression and repair ──► #344 remaining readers
                                              └──► fused false-positive re-measurement
corpus-assertion.md A–E ─────────────────────────► supplies the precision pins
```

---

# Ledger

Kept only for fused repair IDs cited from tests or specifications.

| ID | What it fixed | Held by |
|---|---|---|
| `[REPAIR-PURGE-QUARANTINE]` | Deleted functions that existed only to abort | no remaining quarantine marker under `crates/` |
| `[REPAIR-ADMISSION-PIN]` (#343) | Pinned bounded-max arithmetic at admission | `pair_admission_bounded_max.rs` |
| `[REPAIR-DECLARATION-FAMILY]` | Bounded sibling-boilerplate filtering in both configurations | `dart_issue_197_single_file_structural_only.rs`, `declaration_family_plurality.rs`, `declaration_family_mixed_component.rs`, `refactor_merge`, `issue_190_data_table_demote.rs` |
| `[REPAIR-PY-DICT-ASSERT-DEPTH]` | Recognised the pytest dict-assert idiom at every relevant AST depth | `python_issue_107_chained_dict_assert.rs` |
| `[REPAIR-DOC-TRUTH]` (#345) | Removed the obsolete sum-and-clamp public contract | `[FUSION-STRATEGY-BOUNDED-MAX]` |
| `[REPAIR-RENAME-ANCHOR-MASS]` (#405) | Replaced a four-literal cliff with smoothly weighted Baker-corroborated anchor mass | `type2_rename_anchor_floor.rs`, `fused_golden_bands.rs`, `js_language_features.rs`, `js_ts_clone_buckets.rs` |
| `[REPAIR-CONTENT-FRONTIER]` | Stopped collapsed non-leaves from re-testing bytes already represented by descendants | `js_language_features.rs`, `fused_golden_invariants.rs` |
