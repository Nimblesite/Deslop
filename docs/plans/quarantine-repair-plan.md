# Quarantine repair plan — replacing the panics with working code

Branch `fused` carried five accuracy quarantines. **Every one of them is purged.** No `panic!` survives, no dead panicking function is kept as a marker, no `clippy::panic` suppression remains anywhere in `crates/` — `grep -rn QUARANTINED crates/` and `grep -rn "allow(" crates/ | grep panic` both return nothing. [`BRANCH_REVIEW.md`](../../BRANCH_REVIEW.md) adds executable regressions against the same surfaces.

| Site | Issue | Working replacement | Status |
|---|---|---|---|
| `pair.rs::PairScore::fused` | #343 | `bounded_fused()` — ships, all callers migrated | **deleted** (R0). Admission boundary still unpinned → R5. |
| `config.rs::built_in_excluded` | #342 | `corpus_built_in_excluded` — ships, zero callers left | **deleted** (R0). One live caller handed it an empty config → R3. |
| `session/render.rs::snapshot_corpus` | #301 | `snapshot_corpus_ordered` — ships but is **defective** | **deleted** (R0); fix the replacement → R2. |
| `pair/candidates.rs::add_embedding_pair` | #351 | written — cosine merge | **shipped** (R1). |
| `cluster_filters/declaration_family.rs::is_single_file_declaration_family` | — | written — content-evidence substance test | **shipped** (R9). |

**There are no tombstones in this codebase.** A panicking husk left behind "so the defect can't come back" is dead code pretending to be a guard: it guards nothing, it fails `-D dead-code`, it needs a linter suppression to survive, and it teaches the next reader that panics are an acceptable resting state. The thing that stops a defect returning is the pinning test — `issue_343_sum_clamp_saturation`, `issue_342_scan_root_under_excluded_ancestor`, `corpus_repos.rs::determinism_gate`. Those tests assert rendered behaviour through the CLI; they do not reference the deleted function and do not weaken by one assertion when it goes. The quarantine `panic!` is a **transitional state between finding the defect and shipping the fix**, nothing more. When this plan lands, `grep -rn QUARANTINED crates/` returns nothing.

## Ordering

```
R0 purge the three dead panics ── mechanical, no behaviour change, do it now
R1 cosine merge ──┬──► R8 measured cluster signals ──► R5 admission pin ──► corpus re-baseline (embeddings on)
                  └──► R4 non-finite guard
R2 snapshot path order ──► R3 watcher exclusion
R6 docs ── independent, land last so it describes shipped behaviour
```

R1 changes rendered signals and visible/hidden routing on every embeddings-on report, so only baselines measured after it are worth recording, and R5 pins the boundary R1 moves. The corpus determinism gates run embeddings **off**, so R1 cannot move them — but R2 can, and that is its test. **R8 exists because R1's restored evidence exposed the next defect down**: cluster signals were averaged over the discovery edges of the transitive-closure component, so the very cosines R1 restored diluted byte-proven pairs. R1 without R8 leaves `issue_343` red.

---

## R0 — `[REPAIR-PURGE-QUARANTINE]` (all three dead panics)

**Defect.** Three functions exist only to panic. They are unreachable, they carry `#[allow(clippy::panic)]` (and `dead_code` on one) purely to get past the workspace gates, and each one's replacement already ships and is already the only caller path. Keeping them is a standing linter-suppression exemption and a live `panic = "deny"` violation in a codebase whose own rule is that panics are for control flow never, and for quarantine only until the fix lands.

**Contract.** Deleting an unreachable function cannot change one byte of any report. If any assertion anywhere moves, the function was not unreachable and that is a separate finding to report — not something to work around.

**Working code.** Delete, whole:

- [`config.rs:661-718`](../../crates/deslop-core/src/config.rs#L661-L718) — doc block, `#[must_use]`, `#[allow]`, `pub fn built_in_excluded`. `pub`, so also check for re-exports before deleting.
- [`pair.rs:64-108`](../../crates/deslop-core/src/pair.rs#L64-L108) — doc block, `#[allow]`, `PairScore::fused`.
- [`render.rs:105-162`](../../crates/deslop-core/src/pipeline/session/render.rs#L105-L162) — doc block, `#[allow]`, `snapshot_corpus`. R2 then fixes `snapshot_corpus_ordered`, which stays.

The defect analysis in each doc block is worth keeping — move it into the pinning test's module doc, where it sits next to the assertion that actually enforces it, not into a comment above deleted code.

R0 is independent of R1–R6 and blocks nothing. Do it first so the branch stops carrying suppressions while the real repairs are in flight.

## R1 — `[REPAIR-COSINE-MERGE]` (#351, review P0)

**Defect.** `add_embedding_pair` discarded `pair.cosine` for any pair already in the candidate map. Structural pairs rendered `embedding_cos = 0.0` — byte-identical files reported as semantically unrelated. Cross-file LSH pairs lost their cosine, which reclassifies them `lsh_only` (needs `token_jaccard ≥ 0.90` instead of the fused gate) and routes their cluster `loosely_similar` → hidden, while the same pair discovered by ANN alone is shown. Discovery order decided whether a real duplicate was visible.

**Contract.** A measured cosine belongs to the pair, not the pass that surfaced it. Discovery route is telemetry, never evidence.

The old comment justified the discard as preserving "unique-recall accounting". **No such consumer exists** — grep finds no production reader of discovery route, and `issue_91_embedding_roi.rs:44` already asserts the opposite.

**Working code:**

```rust
fn add_embedding_pair(
    pair: &EmbeddingPair,
    scores: &mut HashMap<(usize, usize), f64>,
    cosines: &mut HashMap<(usize, usize), f64>,
) {
    let key = order(pair.left, pair.right);
    let _structural = scores.entry(key).or_insert(0.0_f64);
    record_cosine(key, pair.cosine, cosines);
}
```

`record_cosine` keeps the maximum, so re-entry is idempotent and adds no #301 surface. `same_file_pair` and the `fingerprints` parameter become dead — delete them.

**Blast radius** — every consumer of `embedding_cos`:

| Consumer | Predicate | Effect |
|---|---|---|
| `pair.rs:242` `survival_decision` | `structural <= 0 && embedding_cos <= 0` | LSH pairs stop being `lsh_only`; more survive. Intended. |
| `buckets.rs:351` `classify_signals` | `embedding_cos >= 0.80 && structural < 0.50` | LSH pairs route visible instead of hidden. Intended — the false negative closing. |
| `report.rs:332` mega-cluster hide | `structural < 0.10 && embedding_cos >= 0.80 && size > 10 && nodes > 500` | Large LSH families can newly hide. **Risk — needs a fixture.** |
| `report_render.rs:404` C# Type-3 near miss | `embedding_cos <= EPSILON` | C# near-misses gaining a cosine lose the carve-out. **Risk — needs a fixture.** |
| `cluster.rs:400` `is_embedding_dominant` | `structural < ceiling && embedding_cos >= floor` | Unaffected (structural pairs score 1.0). |
| `pair.rs:325` `mean()` | sums, divides | **Dilutes** — restored cosines flood the component and drag byte-proven pairs off their bucket. → R8. |

The two risk rows are where R1 could trade one false negative for another. Fixture first; if a cluster disappears, the threshold moves, never the assertion.

**Deeper layer found while implementing R1.** Merging the cosine at the call site was not enough: `classify_snippet` in the embedding pass deduplicated fingerprints by *snippet content hash*, so only the first fingerprint with a given body received a vector and every other copy was dropped. Byte-identical duplicates share their body by definition — the more perfect the duplicate, the more certainly its embedding evidence was destroyed, and the missing cosine rendered `0.0`. Fixed by rebuilding the lookup phase around `SnippetGroup`: the provider *request* is deduplicated by content hash (cost, legitimately), the *result* fans out to every fingerprint in the group (`EmbeddingBatch::push(&[usize], &[f32])`). Failure and shared-input counters count group members, not groups.

## R8 — `[REPAIR-CLUSTER-SIGNAL-TRUTH]` (exposed by R1) · P0

**Defect.** `ClusterTotals::mean()` averaged the three signals over **every surviving pair in the union-find component**. Closure admits any above-threshold edge, so the mix reflects discovery topology — structural star buckets, ANN top-k fan-out, LSH band width — not the occurrences the report shows. Before R1 the dilution was masked by missing cosines; with evidence restored, the two-byte-identical-file corpus rendered the true cluster at `structural = 0.36` and routed it `same_behavior` instead of `identical`. The subsumption pass compares `signals.structural`, so diluted values also let contained artifact clusters escape collapse.

**Contract** — `[FUSION-CLUSTER-SIGNALS]`, `docs/specs/fusion.md`. A rendered signal describes the rendered occurrences: the per-signal mean over every unordered pair of collapsed members. `structural` is Merkle-hash equality, `token_jaccard` is the `MinHash` estimate between the two signatures, `embedding_cos` is the cosine of the two vectors by the same arithmetic that admitted the pair. A pair missing an input for one signal leaves that signal's numerator *and* denominator untouched — absence never enters a mean as a measured `0.0`.

**Working code.** `FusedCluster` drops `mean_score` and carries membership only; `ClusterTotals` is deleted. `cluster::signals::measured_signals` measures the triple after same-file overlap collapse, so the values describe exactly the occurrence list rendered. `EmbeddingOutcome` gains `vectors: HashMap<usize, Vec<f32>>` and `embedding::cosine_similarity` becomes the crate's single cosine definition — a second implementation would let the report disagree with the pipeline about the same two vectors.

## R2 — `[REPAIR-SNAPSHOT-PATH-ORDER]` (#301 second order, review P0)

**Defect.** `snapshot_corpus_ordered` (`render.rs:174-181`) sorts by `FileId`. The registry never unregisters, so re-adding a byte-identical file issues a new, higher id, moving the fingerprint sequence and the LSH star centre. Measured in the LSP: restoring byte-identical source moved duplicated LOC from 96 (100%) to 56 (58.33%). Same defect class as the quarantined `snapshot_corpus`, reached another way — the tombstone only removed the `RandomState` half.

**Contract.** `[PIPELINE-DETERMINISM]` holds over corpus *state*, not edit history: identical paths and bytes produce identical reports whatever sequence of edits got there.

**Working code.** Sort by normalized workspace-relative path, `FileId` as tie-breaker, using the existing `live_paths: HashMap<FileId, PathBuf>`. Two traps:

- **Normalize before comparing** — strip the root and compare components, or `/repo/./src/a.ts` and `/repo/src/a.ts` sort apart, and Windows separator/case differences make raw string order wrong.
- **Never `filter_map` away a `per_file` entry missing from `live_paths`** — that turns a bookkeeping bug into a false negative. Update both maps as a unit or make the mismatch a `CoreError`.

**Blast radius.** Path order should equal discovery-walk order for a single root, so cold-scan output should not move. Assert it: if the corpus gates move, walk order is what needs pinning.

## R3 — `[REPAIR-WATCH-EXCLUSION]` (review P0)

**Defect.** `file_watch.rs:62` builds the watcher with `ExclusionConfig::empty().with_scan_root(root)` — `include_dependencies = false`. The cold scan honours the opt-in, so a new `node_modules/pkg/Gamma.cs` is filtered before scheduling and the live report disagrees with the batch report for the same corpus. Not a #342 misfire: `corpus_built_in_excluded` is correct, it is being handed an empty config.

**Contract.** Live and batch apply one exclusion policy. A config the cold scan honoured cannot be dropped at the watcher boundary.

**Working code.** Clone the session's resolved `exclusion` at `start`, pass it into `LiveWatcher::start` behind the `Arc` the watcher already takes, and swap it on the same path that calls `reload_exclusion` so both sides move at once. Artefact directories stay unconditionally excluded — the opt-in is for dependencies.

## R4 — `[REPAIR-VECTOR-FINITE]` (review P1)

**Defect.** `3.5e38` overflows the `f32` conversion, `CosinePoint::new` normalizes to `NaN`, and every `NaN` comparison is false. Both `cosine < MIN_COSINE` (`embedding/pairs.rs:136-140`) and `embedding_cos <= 0.0` (`pair.rs:242`) **fail open**, so a malformed provider response manufactures clusters. R1 raises the severity: cosines will reach far more admission decisions.

**Working code.** Three guards, because one is not enough: reject non-finite components at ingest (counted as failed subtrees, like oversized inputs); reject non-finite distances and cosines in both exact and ANN paths (`cosine_from_distance`'s `.clamp` returns `NaN` for `NaN` — clamp is not a guard); normalize axes to finite before every survival predicate. `bounded_fused()` already filters non-finite, which is why the fused gate alone does not fail open.

## R5 — `[REPAIR-ADMISSION-PIN]` (review P1)

**Defect.** The #343 fixture proves rendered confidence for a pair whose strongest axis already clears `FUSED_THRESHOLD`; it would still pass if *admission* reverted to the sum, because sum and max agree above the line. The tombstone guards the call site, nothing guards the arithmetic.

**Test.** Paired calibration, after R1 makes refresh reliable: two files at cosine **0.86** must cluster and be visible; the same pair at **0.82** — every axis below 0.85, old sum above it — must yield `cluster_count == 0` **and** `clusters_hidden == 0` (hidden-but-present means admission still happened). Poll distinct model versions so a stale empty report cannot pass. The 0.82 case is the whole test.

## R9 — `[REPAIR-DECLARATION-FAMILY]` · P0 · **shipped**

**Defect.** `cluster_filters/declaration_family.rs::is_single_file_declaration_family` decided whether a single-file `structural_only` cluster is sibling-declaration boilerplate. It could not tell that boilerplate from real duplication **in either configuration**, and each configuration had a red test.

Its first question was `if members.len() < 3 { return false }` — a size threshold standing in for a structural question, the same defect class the review flagged in `calls.rs`. With the floor in place a two-window settings family walked through: `index.dart` `[2490,4001]` / `[6180,7668]`, `get`/`reset`/`update` methods, `structural = 1.00` / `token_jaccard = 0.00`, topping the report as exactly the REST surface the filter exists to suppress. **False positive**, pinned by `single_file_structural_only_method_families_do_not_top_the_report`.

Its second question — the CST declaration-vs-statement discriminator, meant to answer the same thing honestly so the floor could go — did not answer it either. With the floor removed, `csharp-merge-rename` produced **zero clusters**: a genuine two-method C# clone under consistent renames, liftable by the merge planner, was classified a sibling-declaration family and erased. **False negative**, pinned by `refactor_merge::consistent_renames_lift_without_parameters`.

Bisected against `HEAD`, one file at a time: `cluster.rs` and `cluster/subsume.rs` leave `refactor_merge` 9/9; the floor's removal alone turned it red.

**Contract.** `descendant_for_byte_range` returning `method_declaration` says only *where* a member sits, never whether its siblings differ solely in their literals — and that is the question. Neither the member count nor the covering node kind may decide it. A replacement compares the members' collapsed content and is green on **both** pinning tests before it returns anything; a body-difference threshold expressed as a member count is the same defect wearing a different constant.

**Shipped code.** The evidence already existed: `content.rs` measures what normalisation erased, per aligned leaf position, for every cluster. It now records two booleans rather than one, because the single one conflated two very different findings:

- `substance_varies` — some aligned literal differs, **or** the identifier substitution needs more than one consistent mapping.
- `identifiers_vary` — only the second half.

The split is the heart of the repair. **Differing literals are not evidence of scaffolding, because differing literals are exactly what a parameterised merge lifts.** `csharp-merge-drift`'s `ApplyStandard`/`ApplyPremium` differ only in `"standard"`/`100` versus `"premium"`/`250`; suppressing on literal variation alone erased them and took the LSP merge offer *and its refusal reason* with it (`code_action::drifted_fixture_resolve_disables_with_reason`, `code_action_refusal::refused_resolve_surfaces_showmessage_warning`). Differing **names** are evidence: sibling REST methods reach different call targets (`getMethod` / `deleteMethod` / `putMethod`) and no single substitution explains that.

The filter is those signals plus two structural guards:

```rust
category == CloneCategory::Logic
    && !members.is_empty()
    && !spans_multiple_files(..)
    && content.substance_varies
    && (content.identifiers_vary || every_window_covers_two_or_more_siblings)
```

`substance_varies` deliberately carries **no** literal-anchor floor, unlike `pair_rename_consistency`. That floor guards a *score* against agreement by coincidence — with few anchors, matching literals are weak evidence *for* a rename. A *disagreement* needs no floor: differing bytes at an anchored position are evidence on their own, and `RateMath`'s two-literal body under a maximal rename is still a clone. This is why the C# pair survives where the anchor-gated score abandons it.

The **plurality** guard is the question the filter is named for and the one the deleted code never asked: a family is *plural*. It counts members of the enclosing declaration container (`class_body`, `declaration_list`, …) that the window touches — not per-language declaration node kinds, because tree-sitter-dart has no `method_declaration` at all: a Dart class member is a generic node identified by the `function_body` it carries, so a kind list is wrong on the very language this filter exists for.

The **data-category** guard is the third. A table of constructor rows varies its literals by construction — that is what a table *is* — so the substance test convicts every one of them. But a table's payload is its substance, repeating it is a real finding, and the user already chooses its fate through the three-way `data_clones` policy. Without this guard the Dart `highlight_data.dart` table was hidden outright and `issue_190_data_table_demote` lost both `default_demotes_data_table_below_logic_clone` and `keep_mode_restores_data_table_to_the_top` — a false negative traded for the one being fixed.

**Still open — `dart_issue_197` is RED.** One shape resists every signal above. The vendored meilisearch file contains twelve `resetX()` methods whose entire body is `return await _getTask(http.deleteMethod('<endpoint>'))`: same call targets, same shape, differing only in the endpoint literal. That is **structurally identical to `csharp-merge-drift`** — single file, sibling methods, consistent identifiers, varying literals — and every discriminator that hides the Dart family also erases the C# merge target. The only thing separating them is how much logic each body carries: one delegating statement versus eight, i.e. whether the extraction is worth doing at all. That is a product judgement about the reportable floor, not something the content evidence can answer, so the test is left red rather than resolved by a constant that would silently re-erase the merge fixtures. `refactor_merge` (9/9), both LSP `code_action` suites, `issue_190` (5/5), `rank_structural_only_policy` (5/5) and `issue_134` are green; `dart_issue_197` is the one that is not, and it is a real false positive.

**Deleted code** (verbatim, so the shortcut is recognisable if it is ever proposed again): the member-count floor; the same-`file_id` and `uniform_language` guards; `collect_snippets` + `snippets.iter().all(member_is_declaration_context)`; `member_is_declaration_context`, which took `descendant_for_byte_range(start, end-1)` and accepted `is_declaration_kind` or `is_declaration_body_kind`; and those two `matches!` kind lists (C# `method_declaration`/`constructor_declaration`/`property_declaration`/`field_declaration`/`class_declaration`/`struct_declaration`/`interface_declaration`/`record_declaration`; Rust `function_item`/`struct_item`/`impl_item`/`trait_item`/`mod_item`/`enum_item`; Python `function_definition`/`class_definition`/`decorated_definition`; Dart `method_signature`/`function_signature`/`declaration`/`extension_declaration`; JS/TS `method_definition`/`public_field_definition`/`function_declaration`/`generator_function_declaration`/`abstract_class_declaration`/`type_alias_declaration`/`enum_declaration`/`property_signature`; bodies `declaration_list`/`compilation_unit`/`source_file`/`class_body`/`extension_body`/`mixin_body`/`enum_body`/`program`/`interface_body`/`object_type`/`statement_block`).

## R10 — `[REPAIR-PY-DICT-ASSERT-DEPTH]` · P0 · **shipped**

**Defect.** `[CLONE-NOISE-PY-DICT-ASSERT]` recognised the chained `assert payload[k1][k2]` idiom only through `enclosing_kind(.., ["function_definition"])` — that is, only when the reported range sat *inside* a `test_*` function. Fingerprinting emits one subtree per AST node, so the same idiom is also offered as the whole function and as the whole module, and [PIPELINE-CLUSTER-SUBSUME] only collapses views covering the same region **in both directions**. A module-wide view naming a different file set than the assert-run view therefore survives subsumption on its own — and was published: `test_configs_patch.py [0,296]` + `test_openapi.py [0,249]`, `structural_only`, two unrelated pytest modules as a whole-file duplicate. **False positive**, pinned by `python_issue_107::chained_dict_assertions_across_test_files_do_not_cluster`.

It surfaced when `covers_same_region` became bidirectional. The stricter predicate is correct; it stopped *masking* this cluster by collapsing it into a view that was itself filtered. Being hidden by an incorrect collapse is not being filtered — the noise cluster was always there.

**Shipped code.** `python_dict_assert.rs` matches the `test_*` functions the range **intersects** — enclosing or enclosed — so one idiom is recognised at every depth. Two consequences follow, and both are part of the idiom rather than concessions to it: the literal payload assignment `data = {...}` is accepted alongside the asserts (a dict literal only — a call, fixture reference or comprehension is program logic), and members whose reported bytes are all identical are exempt via `raw_snippet_texts_differ`, because a verbatim copy of a test is real duplication whatever idiom it is written in.

Extracted from `python.rs` rather than added to it: that file was at 490 lines and the 500-line budget is not negotiable for a tool that detects duplication.

## R6 — `[REPAIR-DOC-TRUTH]` (#345, review P1)

`fusion.md:13,17-18` claims embeddings default on with config/env fallbacks that do not exist and names `nomic-embed-code`; shipped CLI defaults off with `nomic-embed-text`. Both site languages (`research-background.md:61,155,208` / `:62,156,209`) still teach sum-and-clamp via `PairScore::fused` — public pages documenting quarantined code. `[FUSION-STRATEGY-MAX-SUM]` must migrate to `[FUSION-STRATEGY-BOUNDED-MAX]`: a durable ID naming the quarantined arm is a trap. Plus remaining #345 drift.

## Merge gate

| Test | Must be |
|---|---|
| `deslop --test issue_343_sum_clamp_saturation` | 4/4 green (R1) |
| `deslop-lsp --test branch_accuracy` | green, both cases (R1) |
| `deslop-lsp --test dependency_reactivity` | green, both directions (R3) |
| `deslop-lsp --test history_determinism` | green, both cycles (R2) |
| malformed-provider regression | green (R4) |
| admission calibration 0.86 / 0.82 | green, and red against a restored sum (R5) |
| corpus determinism `nest` / `jellyfin` | unchanged: 1293 / 30.0687%, 1933 / 19.8354% (R2) |
| corpus gate, embeddings on | first recorded measurement (R1) |
| `deslop --test dart_issue_197` · `single_file_structural_only_method_families_do_not_top_the_report` | green (R9) — and red against a restored member-count floor |
| `deslop-core --test refactor_merge` | 9/9 green (R9) — `csharp-merge-rename` must produce clusters |
| `deslop --test issue_190_data_table_demote` | 5/5 green (R9) — a data table is demoted or restored by policy, never hidden |
| `deslop --test python_issue_107_chained_dict_assert` | green (R10) — no whole-module view of the idiom reaches the report |
| `grep -rn QUARANTINED crates/` | **zero hits** (R0 + R1 + R9) |
| `grep -rn "allow(" crates/ \| grep panic` | **zero hits** (R0 + R1 + R9) |

The three R9 rows are required **together**, and that is the whole point of them. Each one alone is satisfiable by the defect in one of its directions — which is exactly how the member-count floor survived as long as it did, and how its removal then traded the Dart data table away for the C# clone. `dart_issue_197` alone passes with the floor. `refactor_merge` alone passes without it. `issue_190` alone passes if the filter never fires. Only the three together state the contract: **suppress sibling scaffolding, keep proven renames, and leave data tables to their policy.**

---

# TODO

## R0 — `[REPAIR-PURGE-QUARANTINE]` · **do first, mechanical**

- [x] Confirm `PairScore::fused` has zero callers (`grep -rn "\.fused()" --include="*.rs" crates/`) and is not re-exported.
- [x] Confirm `built_in_excluded` has zero callers and is not re-exported from `lib.rs` or any other crate — it is `pub`.
- [x] Confirm `snapshot_corpus` has zero callers (`pub(super)`, carries `dead_code`).
- [x] Move each doc block's defect analysis into the module doc of its pinning test (`issue_343_sum_clamp_saturation.rs`, `issue_342_scan_root_under_excluded_ancestor.rs`, `corpus_repos.rs`).
- [x] Delete `config.rs:661-718` entirely — doc, `#[must_use]`, `#[allow]`, `pub fn built_in_excluded`.
- [x] Delete `pair.rs:64-108` entirely — doc, `#[allow]`, `PairScore::fused`.
- [x] Delete `render.rs:105-162` entirely — doc, `#[allow]`, `snapshot_corpus`. Leave `snapshot_corpus_ordered` for R2.
- [x] `make lint` clean with `--features live,test-support`, no new suppressions added anywhere.
- [x] All three pinning suites still green with **zero** assertions changed. If any moves, the function was reachable — STOP and report.
- [x] `grep -rn QUARANTINED crates/` returns only the R1 site, which R1 then removes.

## R1 — `[REPAIR-COSINE-MERGE]` #351 · P0 · **the only remaining panic**

Tests first — each must be watched failing for the real reason.

- [x] Fixture: cross-file pair the mega-cluster hide could newly swallow (`structural < 0.10`, `embedding_cos ≥ 0.80`, `size > 10`, `canonical_node_count > 500`). Assert it stays **visible** after R1. → `embedding_route_invariance::embeddings_on_reports_every_file_set_embeddings_off_reported`, pinned as a sweep over every cluster of every corpus rather than one hand-built cluster. **RED** — see [the route-invariance findings](#embeddings-on-loses-findings-and-moves-buckets).
- [x] Fixture: C# LSH Type-3 near miss (`report_render.rs:404` carve-out). Assert it keeps its bucket once it carries a cosine. → `embedding_route_invariance::embeddings_on_never_moves_a_reported_bucket`. **RED** — same section.
- [x] Test: **discovery-route invariance** — same two files, two `--min-nodes` values so one run finds the pair structurally/by LSH and the other only by ANN. Assert identical signal triple, bucket, visible count, hidden count across both runs.
- [x] Test: cross-file LSH overlap with cosine ≥ 0.80 renders its cosine and routes to a visible bucket, not `loosely_similar`.
- [x] Watch all four fail (the two invariance/overlap tests fail on the quarantine panic; the two fixtures fail or pass-for-the-wrong-reason — verify which). Both risk-row tests were watched failing, and both fail for the real reason, not the panic.

Then the code.

- [x] Replace both panicking arms in `add_embedding_pair` with the unconditional `entry().or_insert(0.0)` + `record_cosine` body.
- [x] Delete `same_file_pair` and drop the now-unused `fingerprints` parameter from `add_embedding_pair` and `add_embedding_pairs`.
- [x] Rewrite the `add_embedding_pairs` doc comment — remove the "unique-recall accounting" claim, which described a consumer that does not exist.
- [x] Delete the `#[allow(clippy::panic, …)]` attribute with the panics. No `clippy::panic` suppression may survive R1 — with R0 done, this is the last one in the workspace.

Verify.

- [x] `issue_343_sum_clamp_saturation` 4/4 green.
- [x] `deslop-lsp --test branch_accuracy` green (both cases, including `lsp_embedding_refresh_is_bounded_and_reproducible`).
- [x] Re-run the fused/bucket suites that read `embedding_cos`: buckets, hidden routing (#58/#120/#122), `issue_91`, `issue_93`, `issue_98_99_108_120_122_thresholds`, `fused_golden_bands` — all green. `fused_golden_invariants::no_golden_report_renders_a_constant_fused_score` is **red**, pinning a real defect: see [`ts-mixed-band`](#ts-mixed-band-one-confidence-for-three-bands).
- [x] Confirm corpus determinism gates unchanged (they run embeddings off — a move here means R1 leaked into the batch path). Measured: `nest` 1293 / 30.0687%, `jellyfin` 1933 / 19.8354%, both runs of both gates.
- [ ] Record the first embeddings-on corpus measurement.
- [ ] Comment on #351 with the before/after and the two risk-row outcomes. Do not close.

## Embeddings on loses findings and moves buckets · **open, red**

The two R1 risk rows, made executable in `crates/deslop/tests/embedding_route_invariance.rs`. Both run the same corpus twice — once with the embedding pass off, once with it served by the deterministic mock provider — and compare the *published file sets* (not cluster ids, which legitimately change when a cluster gains an occurrence). Both are red, so the review's prediction was right: R1 restored one false negative and opened others.

**`csharp-type3` — a proven duplicate is re-labelled as a semantic guess.** Embeddings off publishes two `structural_only` clusters over `{Delta.cs, Epsilon.cs}`, each at `structural 1.0`. Embeddings on publishes **one** cluster over the same pair, bucketed `same_behavior`. Two findings became one, and the surviving label claims embedding-only evidence for a duplication the cold run had proven structurally. The bucket followed the discovery route, which is exactly what [FUSION-CLUSTER-SIGNALS] forbids.

**`ts-mixed-band` — the corpus reports nothing at all.** Embeddings off publishes a four-file `nearly_identical` cluster. Embeddings on publishes zero clusters. Every finding in the corpus disappears when the semantic signal is added.

Neither is the mega-cluster hide firing on size (both clusters are far under `size > 10`), so the cause is upstream of it — the restored cosines are changing cluster *membership* through the union-find closure, and the re-measured signals then route the merged component differently. That is the R8 dilution failure mode reappearing at the membership layer rather than the mean layer.

- [ ] Establish where the cosine edges change membership: closure admission, or the collapse that precedes `measured_signals`.
- [ ] Fix so a cluster's bucket is a function of its occurrences, never of which pass reached them.
- [ ] Re-run the corpus gates with embeddings on afterwards — the R1 "first embeddings-on measurement" box stays open until this is closed, because a measurement taken now would record the defect as a baseline.

## `ts-mixed-band`: one confidence for three bands · **open, red**

Found while re-running the R1 verification suites. `ts-mixed-band` stages five TypeScript files: `ledger_a` the original, `ledger_d`/`ledger_e` one parenthesis apart from it, `ledger_c` a renamed copy, and `ledger_b` a same-shape family whose identifiers *and* every literal differ. The report publishes **one** visible cluster — `ledger_a`, `ledger_b`, `ledger_d`, `ledger_e` at `structural 1.0 / token_jaccard 1.0 / fused 1.0`, bucket `nearly_identical` — plus one hidden `loosely_similar` cluster over all five. `ledger_c` never surfaces.

Two consequences, both accuracy-grade:

- **The rendered confidence cannot separate a one-token edit from a wholesale rewrite.** `token_jaccard = 1.0` between `ledger_a` and `ledger_b` means the token axis carries no evidence the structural axis does not already carry — it is measuring the same normalised signature, so a full rename plus a full literal change scores identically to a byte copy. That is the axis whose whole job is to catch what structural normalisation erases.
- **`ledger_c` is a false negative**, present in no visible cluster.

Pinned by two red tests, neither of which may be weakened to land this branch:

- `deslop --test fused_golden_invariants::no_golden_report_renders_a_constant_fused_score`
- `deslop --test cross_cluster_enclosure::ts_mixed_band_renders_a_distinct_confidence_per_band`

Not caused by R1 or by the subsumption work: the corpus entered the sweep when `ts-mixed-band` was prepended to `SWEEP`, which is also what silently dropped PHP from `SWEEP.iter().take(6)` (R7). **R7's "replace `take(6)` with an explicit golden corpus list" must not be used to drop `ts-mixed-band` from the sweep** — that would retire a red test by narrowing coverage.

- [ ] Decide whether `token_jaccard` is measuring the normalised signature rather than the token stream, and pin the answer with a two-file rename-plus-literal fixture.
- [ ] Establish why `ledger_c` reaches no visible cluster.
- [ ] Fix the axis, not the fixture; re-measure the corpus gates afterwards.

## R2 — `[REPAIR-SNAPSHOT-PATH-ORDER]` #301 · P0

- [x] Strengthen `deslop-lsp/tests/history_determinism.rs` while red: assert the **whole** report equal (cluster ids, ranks, `duplication_percent`, per-file metrics), not just `duplicated_loc`.
- [ ] Add the reverse cycle (add A→B, remove both, re-add B→A) so the assertion is order-invariant.
- [x] Confirm it fails for the `FileId` ordering, not harness drift.
- [x] Implement path ordering in `snapshot_corpus_ordered` from `live_paths`, `FileId` as tie-breaker.
- [x] Normalize: strip the workspace root, compare `Path` components — not raw absolute strings.
- [ ] Make a `per_file` entry with no `live_paths` entry a hard error, never a silent drop.
- [x] `history_determinism` green both cycles.
- [x] Corpus determinism gates still 1293 / 30.0687% and 1933 / 19.8354%.
- [x] Grep the render path for any remaining `FileId`-ordered iteration.
- [ ] Update the #301 comment: the first fix was partial, this completes it. Do not close.
- [ ] Correct the #301 row in [`fused-score-followups.md`](fused-score-followups.md) — it currently claims determinism is fixed.

## R3 — `[REPAIR-WATCH-EXCLUSION]` · P0

- [ ] Strengthen `dependency_reactivity.rs` while red: assert the **negative** direction (no opt-in → a new `node_modules` file must not enter the report).
- [ ] Add config reactivity: flip `include_dependencies` in `.deslop.toml`, assert the live report converges with no restart.
- [x] Pass the session's resolved `ExclusionConfig` into `LiveWatcher::start` instead of `ExclusionConfig::empty()`.
- [x] Swap the shared `Arc` on the `reload_exclusion` path so watcher and session never disagree.
- [ ] Assert artefact directories stay excluded regardless of the dependency opt-in.
- [x] `dependency_reactivity` green both directions.

## R4 — `[REPAIR-VECTOR-FINITE]` · P1

- [x] Mock provider returning `3.5e38`, `NaN`-producing, and infinite components.
- [x] Assert the scan fails loudly or counts the vector as failed — never that it silently produces clusters.
- [x] Assert the failed-subtree counter accounts for every rejected vector.
- [x] Reject non-finite components before cache/index insertion; count as failed like an oversized input.
- [x] Reject non-finite distances and cosines in **both** exact and ANN paths (`.clamp` is not a `NaN` guard).
- [x] Normalize pair axes to finite before every survival predicate.
- [ ] Grep for remaining raw `embedding_cos` comparisons with no finite guard upstream.
- [ ] Decide whether the fail-open sites need the quarantine treatment or a guarded replacement — **report to the user before repairing either.**

## R5 — `[REPAIR-ADMISSION-PIN]` · P1 · after R1

- [ ] Calibration test at cosine 0.86: pair clusters, cluster is visible.
- [ ] Calibration test at cosine 0.82: every axis < 0.85, old sum > 0.85. Assert `cluster_count == 0` **and** `clusters_hidden == 0`.
- [ ] Poll distinct model versions between the two so a stale empty report cannot pass.
- [ ] **Inversion check:** restore the sum in a scratch build, confirm 0.82 goes red, discard the build. A test that never fails against the old code asserts nothing.

## R6 — `[REPAIR-DOC-TRUTH]` #345 · P1 · land last

- [ ] `fusion.md:13,17-18` — document shipped defaults (`--embeddings off`, `nomic-embed-text`) or change the defaults; delete the config/env fallbacks that do not exist.
- [ ] `site/src/docs/research-background.md:61,155,208` — replace sum-and-clamp with bounded max + content gate.
- [ ] `site/src/zh/docs/research-background.md:62,156,209` — same, Chinese.
- [ ] Rename `[FUSION-STRATEGY-MAX-SUM]` → `[FUSION-STRATEGY-BOUNDED-MAX]` across all carrying files in one change; verify `grep [FUSION-STRATEGY-` still finds spec → code → tests.
- [ ] `REPORTING-CONTEXT.md` — separate the admission threshold from the rendered confidence; they share one name.
- [ ] `mcp.md` — `top-offenders` sorts by weight, not fused score.
- [ ] `corpus/known-failures.json` — add `fused_spread` / `type2_recall` check ids.
- [ ] Grep: no doc names `PairScore::fused` as live code.

## R7 — smaller review fixes

- [ ] `fused_golden_invariants.rs:246` — replace `SWEEP.iter().take(6)` with an explicit golden corpus list; prepending `ts-mixed-band` silently dropped PHP.
- [ ] `Makefile:5` — references the deleted `docs/plans/PLAN.md`.
- [ ] Fix SKILL-relative `docs/…` links in the changed spec-check / submit-pr skills.
- [ ] `.claude/skills/submit-pr/SKILL.md:45` — wrong step number.
- [ ] `docs/plans/lang-roadmap.md` — replace `P-LANG-0/3/5` with descriptive hierarchical ids plus real spec/code/test cross-references.
- [ ] `docs/plans/vsix-ux-plan.md:7` — remove already-shipped work.

## Carried from `fused-score-followups.md` (not part of this plan, still open)

- [ ] #344 — confidence to every consumer; wire `agreement` / `rename_consistency` / `literal_fraction`; restore the 17 softened fixtures.
- [ ] Six skipped VSIX tests (A first — the regression this release introduced).
- [ ] #339 — fallback token signatures under-report `token_jaccard`.
- [ ] `workflow_dispatch` the corpus gate after merge; close #301 / #331 / #336 only on a green run.
