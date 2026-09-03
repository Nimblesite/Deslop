# 0.33.0 release accuracy audit

Head at audit: `b5273c16` (PR #501). PRs #494 and #501 are both merged and both carry their weight.

**0.33.0 release: NO-GO.** The line inherited a same-file recall band from PR #485 that reports less than 0.32.0 on duplication its own fixtures call real. Neither merged PR is the cause and neither fixes it. Four recall pins were inverted to assert the loss rather than answer it, so the suite is green over findings the fixtures themselves call real.

## What was compared

Four binaries scanned all 206 fixtures with identical inputs (`--min-nodes 8 --embeddings off --no-incremental`): 0.32.0 built from `f92300e5`, merged main `1ecfc997` (PR #485), PR #494 `bb17d9cb`, and head `b5273c16` (PR #501).

| Comparison | Fixtures that differ |
|---|---:|
| PR #494 against merged main | 1 |
| head against PR #494 | 0 |
| head against 0.32.0 | 71 |

The one difference from main is `js-mjs-cjs-family`. PR #501 rewrote the subsumption stage and changes **no fixture's output at all** — its per-file-set kernel reaches the same verdicts the pairwise scan did, on every fixture, so the performance work carries no accuracy cost. Nothing else in the pipeline's output moved.

## What PRs #494 and #501 fix

**[FUSED-SHARED-SUBTREE-ECHO] One-sided echo pairs no longer widen a finding (gh #493).** A rescue pair with one endpoint enclosing an exact whole-function clone and the other lying inside it shares nothing beyond that clone, so the whole pair is claimed and the rescue refuses it. The JavaScript family published three whole files, 53 duplicated lines, for three byte-identical declarations holding 45. It now publishes the declaration at lines 3-17 in all three files, matching 0.32.0, and keeps the `.js`/`.mjs` whole-file copy as its own finding. Pinned by `js_ts_extensions::javascript_family_clusters_across_js_mjs_and_cjs_extensions`.

**[PIPELINE-CLUSTER-SUBSUME-KERNEL] A view whose absorber leaves the report is judged again against what remains (gh #498).** Survivors were the residue of a scan order: a view that yielded to another was remembered only when it yielded one particular way, so when its survivor was later dropped as a straddler the yielded view vanished with it and no cluster reported its bytes — a lost finding whose appearance depended on the id order of equal-mass views. Subsumption now computes, per file set, the set of views no other published view re-describes and outranks, so nothing a removed view held is forgotten. A cycle in the survivor order is decided by the coverage-mass-id tie-break ([PIPELINE-CLUSTER-SUBSUME-CYCLE]), and both straddlers are dropped before their file set is resolved again ([PIPELINE-CLUSTER-SUBSUME-STRADDLE]). Every subsumption test now holds its result to the spec's report contract as well as its own outcome. Pinned by `cross_cluster_collapse::padded_windows_straddling_a_verbatim_block_publish_the_block` and ten unit pins across `cluster_subsumption/{region,straddle,release}.rs`.

**[METRICS-REPO-WEIGHTED] / [EXIT-CODES-WEIGHTED] The one duplication percentage is pinned on every surface.** A recursive key scan proves no report field is weighted, the metrics object carries exactly its eight engine figures, the text and HTML renderers print the same headline built from those figures, and `--fail-over-weighted` is a usage error.

## Release blockers, all inherited from PR #485

| Severity | Finding | Evidence |
|---|---|---|
| P0 | Two methods differing only in literals are refused within a file. | `dart-forwarding-business-pair`'s `standardTotal` and `premiumTotal` are structurally identical and differ in one string and one integer. They measure agreement 0.727 and rename consistency 0.0, so [FUSED-CONTENT-GATE] refuses them below the 0.85 same-file promote floor. The zero is by design: same-file rename evidence takes the stricter min of literal affirmation and identifier coverage, and these methods rename no identifier, so `max(agreement, rename)` collapses to agreement alone. A two-literal copy of an eleven-position method cannot reach 0.85 that way. 0.32.0 published them. The module documentation in `dart_forwarding_fail_open.rs` calls the pair liftable and says the forwarding proof must not hide it, while the test now asserts no cluster spans the file. Documentation and assertion disagree about whether the finding is real. gh #496. |
| P0 | Four recall pins were inverted rather than answered. | In 0.32.0 `dart_forwarding_fail_open.rs` asserted `expect_visible_only(report, 2, ...)` for the business pairs, a two-occurrence cluster for each transform fixture, and `assert_single_file_cluster(cluster, 5, "Api.dart")` for the five-wrapper family. All four now assert `expect_pair_rejected_at_admission`: no cluster may span the file. `wrappers_sharing_a_body_keep_the_family_visible` still carries that name while asserting nothing is visible, and the module documentation above all four still states the positive contract. gh #497. |
| P0 | The same band drops three more 0.32.0 findings. | `dart-forwarding-duplicate-route` (5 wrappers, one shared body — the copy-paste bug the fixture exists to catch), `dart-forwarding-transform-before-delegation`, and `csharp-merge-manyholes` (`Sprawl.cs:3-12` / `:14-23`, six `Set` calls and a `Commit`, literals only varying). Each published in 0.32.0 and publishes nothing now. Measured agreement runs 0.545 to 0.75 with rename consistency 0.0 throughout. |
| P0 | gh #492 is a live false negative and its pin is skipped. | `csharp-merge-drift`'s two drifted methods measure overlap 0.82 with support 0.5625, and the *methods* never cluster: head publishes `DriftLimits.cs:6-8`/`:18-20` and `:9-12`/`:25-28` as two statement fragments, so the copy-paste pair is reported as two smaller findings that name neither method. `csharp_same_file_type3_reports_both_methods_in_one_cluster` keeps every assertion and now carries `#[ignore]`, taking the suite from four curated skips to five. A plan records debt; it does not close it. |

All four have one root: within a file, the promote floor is 0.85 and the shared-subtree rescue does not run at all. Raising recall by admitting same-file rescue candidates was tried in PR #494 and reverted, because it publishes accessor families, helper call sites and data tables (see below). The band needs a discriminator that separates a copied method from a shape family. `docs/plans/same-file-rescue-plan.md` carries the candidates and the acceptance conditions.

## The route that was built and removed

Admitting same-file near-misses reports `csharp-merge-drift`'s methods, which is right, and also publishes duplication that is not there.

| Pin | Under the same-file route |
|---|---|
| `dart_issue_197_single_file_structural_only` | 8 convicted components, not 1: REST settings accessors agreeing on 36% of their positions |
| `python_issue_103_helper_call_sites` | a two-member cluster over already-extracted helper call sites |
| the three `issue_190` modes | the data-table family (mass 217) outranks the logic clone (mass 53) |
| `cli::bucket_groups` | two clusters for one duplication |
| `refactor_merge_refusals` read-after and written-context | the autofix refusals no longer hold, because the finding widened underneath them |

All seven pass on merged main and failed under the route. Narrowing to disjoint whole authored functions cleared four. Requiring a Merkle-equal fragment inside both endpoints cleared none of the rest, because sibling accessors and call sites carry byte-identical argument runs. Every remaining separation was a threshold fitted between two fixtures, so the route came out.

## Everything 0.32.0 published that head does not

123 cluster occurrence sets across 71 fixtures no longer appear at the same line extents. Matching an occurrence set exactly counts a re-extented view as both a loss and a gain, so the sets were re-read against the bytes head actually covers:

| Kind | Sets | Reading |
|---|---:|---|
| Every occurrence is a single line | 49 | one line repeated is not extract-worthy. Improvement. |
| The region is still published, at a different extent | 23 | the enclosing-view election moved the boundary; no bytes lost. The seven `*-type3` fixtures are here, and their pins are green. |
| Genuinely unreported at head | 55 | 33 same-file, 22 cross-file. Split below. |

Of the 55 regions head reports nothing for, the 22 cross-file ones sit almost entirely in fixtures named for the false positive they hold — `*-prologue-false-positive`, `*-dissimilar-functions`, `*-shape-only-*`, `ts-issue-284-produce-then-assert`, `python-issue-115-strenum`, `verbatim-plus-stranger`. 0.32.0 published shape alone; the content gate demands content. Improvement, and each is held by a green pin.

The 33 same-file regions are the release blockers. Stripping the ones whose fixtures exist to catch a false positive leaves the real duplication head is blind to:

| Fixture | Region | What is lost |
|---|---|---|
| `dart-forwarding-duplicate-route` | `Api.dart:22-40` | five wrappers, two of which DELETE `/indexes/dup/settings` — the copy-paste bug the fixture was built for |
| `dart-forwarding-business-pair` | `Pricing.dart:35-43` | `standardTotal` / `premiumTotal`, one string and one integer apart |
| `dart-forwarding-transform-before-delegation` | `Billing.dart:37-45` | computation ahead of the delegating call |
| `dart-forwarding-transform-after-delegation` | `Ledger.dart:36-44` | the same-class call that carries the divergence |
| `csharp-merge-manyholes` | `Sprawl.cs:3-12` / `:14-23` | six `Set` calls and a `Commit`, literals only varying |
| `csharp-merge-drift` | `DriftLimits.cs:5-8` / `:17-20` | gh #492. Head publishes `:6-8`/`:18-20` and `:9-12`/`:25-28` as two fragments and never the two methods |
| `csharp-merge-operatordrift` | `Drift.cs:3-11` / `:13-21` | |
| `csharp-mixed-declaration-component` | `BillingAccruals.cs:18-58` | three occurrences |
| `csharp-nonbijective-pair` | `InvoiceTotals.cs:14-40` | |
| `csharp-type4` | `Iterative.cs`, `Recursive.cs` | two same-file pairs |
| `ast-golden-go` | `Sample.go:29-32` / `:82-85` | |

Every one of these losses is also a loss on merged main.

## Test weakening

The suite is green, and on this band it is green because four assertions were turned around.

`crates/deslop/tests/dart_forwarding_fail_open.rs` states one contract in its prose and the opposite in its assertions. The module header says the forwarding proof "must fail open" and that "everything it cannot read must therefore keep its cluster visible"; `DUPLICATE_ROUTE_WHY` says "hiding the family erases a real finding"; `Pricing.dart` says in its own header that "hiding either pair is a false negative". Under all of that, `same_class_helper_calls_are_not_forwarding`, `a_same_class_call_after_delegation_is_not_forwarding`, `a_same_class_call_before_delegation_is_not_forwarding` and `wrappers_sharing_a_body_keep_the_family_visible` each call `expect_pair_rejected_at_admission`, which asserts **no cluster may span the fixture's file**. The last one asserts the negation of its own name.

In 0.32.0 those four read `expect_visible_only(report, 2, ...)`, a two-occurrence cluster per transform fixture, and `assert_single_file_cluster(cluster, 5, "Api.dart")`. The inversion landed in `1ecfc997` (PR #485). Code, spec and tests do not agree, and the disagreement is load-bearing: it is the only reason the Dart half of the recall band does not show up as failures. gh #497.

Separately, `type3_enclosing_method::csharp_same_file_type3_reports_both_methods_in_one_cluster` was added already carrying `#[ignore]`. Its assertions are intact and it fails for the real reason when run — this is new debt honestly registered, not an inverted pin. It takes the curated skip set from four issues to five.

Nothing in PR #494 or PR #501 deletes or relaxes an assertion. One gate did move: `corpus_confidence::check_mass` now reads its two-occurrence floor off `occurrences_total` rather than `occurrence_count`, so a cluster whose siblings are `report_hide`-suppressed is not condemned ([EXCLUSION-CONFIG]). It is justified and unit-covered, but the end-to-end corpus gate that would exercise it is itself skipped under gh #422.

## Verification

Re-run at head `b5273c16`:

- `cargo test --release -p deslop --test suite`: **459 passed, 0 failed, 5 ignored** — gh #369, #422, #489, #491 and #492, each with a plan and a registry row.
- `cargo test --release -p deslop-core --features live --test suite`: **192 passed, 0 failed**.
- The gh #492 pin run with `--ignored` still fails, for the false negative above.
- `cargo clippy --release --all-targets -- -D warnings` and `cargo fmt --check`: pass.
- No report field on any fixture matches `/weight/i`; `metrics` carries exactly its eight engine figures; `--fail-over-weighted` is rejected as an unknown argument.

## Release gate

1. ~~Merge PR #494.~~ Done, with PR #501 on top. Both are strictly better than the main they landed on and worse than it nowhere.
2. Answer gh #497 first: decide whether the four Dart forwarding findings are real. Every other question in this band depends on that answer.
3. Then gh #496, whether a same-file pair varying only literals should clear 0.85 on positional agreement alone, and gh #492 on top of it.
4. Restore the Dart forwarding controls to the positive contract their documentation and names state, or change both and say why. Until then the suite asserts a loss instead of measuring it.
5. ~~Re-run the 206-fixture paired matrix.~~ Done at head: identical to PR #494, 71 fixtures from 0.32.0, 55 regions genuinely unreported. The 33 same-file ones are the blocker and are itemised above.
6. Full CI matrix green on the final head.
