# 0.33.0 release accuracy audit

**PR #494: merge.** It fixes a false positive, regresses nothing, and differs from merged main on exactly one fixture.

**0.33.0 release: NO-GO.** The line inherited a same-file recall band from PR #485 that reports less than 0.32.0 on duplication its own fixtures call real. That is not this PR's doing and this PR does not fix it.

## What was compared

Three binaries scanned all 206 fixtures with identical inputs (`--min-nodes 8 --embeddings off --no-incremental`): 0.32.0 built from `f92300e5`, merged main `1ecfc997` (PR #485), and PR #494.

| Comparison | Fixtures that differ |
|---|---:|
| PR #494 against merged main | 1 |
| PR #494 against 0.32.0 | 71 |

The one difference from main is `js-mjs-cjs-family`. Nothing else in the pipeline's output moved.

## What PR #494 fixes

**[FUSED-SHARED-SUBTREE-ECHO] One-sided echo pairs no longer widen a finding (gh #493).** A rescue pair with one endpoint enclosing an exact whole-function clone and the other lying inside it shares nothing beyond that clone, so the whole pair is claimed and the rescue refuses it. The JavaScript family published three whole files, 53 duplicated lines, for three byte-identical declarations holding 45. It now publishes the declaration at lines 3-17 in all three files, matching 0.32.0, and keeps the `.js`/`.mjs` whole-file copy as its own finding. Pinned by `js_ts_extensions::javascript_family_clusters_across_js_mjs_and_cjs_extensions`.

**[PIPELINE-CLUSTER-SUBSUME-STRADDLE] Two padded windows that straddle a nested view collapse onto it.** Both straddlers are dropped, the nested view is restored, and subsumption runs to a fixed point. Pinned by `cross_cluster_collapse::padded_windows_straddling_a_verbatim_block_publish_the_block` and two unit pins in `cluster_subsumption`.

**[METRICS-REPO-WEIGHTED] / [EXIT-CODES-WEIGHTED] The one duplication percentage is pinned on every surface.** A recursive key scan proves no report field is weighted, the metrics object carries exactly its eight engine figures, the text and HTML renderers print the same headline built from those figures, and `--fail-over-weighted` is a usage error.

## Release blockers, all inherited from PR #485

| Severity | Finding | Evidence |
|---|---|---|
| P0 | Two methods differing only in literals are refused within a file. | `dart-forwarding-business-pair`'s `standardTotal` and `premiumTotal` are structurally identical and differ in one string and one integer. They measure agreement 0.727 and rename consistency 0.0, so [FUSED-CONTENT-GATE] refuses them below the 0.85 same-file promote floor. The zero is by design: same-file rename evidence takes the stricter min of literal affirmation and identifier coverage, and these methods rename no identifier, so `max(agreement, rename)` collapses to agreement alone. A two-literal copy of an eleven-position method cannot reach 0.85 that way. 0.32.0 published them. The module documentation in `dart_forwarding_fail_open.rs` calls the pair liftable and says the forwarding proof must not hide it, while the test now asserts no cluster spans the file. Documentation and assertion disagree about whether the finding is real. gh #496. |
| P0 | Four recall pins were inverted rather than answered. | In 0.32.0 `dart_forwarding_fail_open.rs` asserted `expect_visible_only(report, 2, ...)` for the business pairs, a two-occurrence cluster for each transform fixture, and `assert_single_file_cluster(cluster, 5, "Api.dart")` for the five-wrapper family. All four now assert `expect_pair_rejected_at_admission`: no cluster may span the file. `wrappers_sharing_a_body_keep_the_family_visible` still carries that name while asserting nothing is visible, and the module documentation above all four still states the positive contract. gh #497. |
| P0 | The same band drops three more 0.32.0 findings. | `dart-forwarding-duplicate-route` (5 wrappers, one shared body — the copy-paste bug the fixture exists to catch), `dart-forwarding-transform-before-delegation`, and `csharp-merge-manyholes` (`Sprawl.cs:3-12` / `:14-23`, six `Set` calls and a `Commit`, literals only varying). Each published in 0.32.0 and publishes nothing now. Measured agreement runs 0.545 to 0.75 with rename consistency 0.0 throughout. |
| P0 | gh #492 is a live false negative and its pin is skipped. | `csharp-merge-drift`'s two drifted methods measure overlap 0.82 with support 0.5625 and never cluster. `csharp_same_file_type3_reports_both_methods_in_one_cluster` keeps every assertion and now carries `#[ignore]`, taking the suite from four curated skips to five. A plan records debt; it does not close it. |

All three have one root: within a file, the promote floor is 0.85 and the shared-subtree rescue does not run at all. Raising recall by admitting same-file rescue candidates was tried in this PR and reverted, because it publishes accessor families, helper call sites and data tables (see below). The band needs a discriminator that separates a copied method from a shape family. `docs/plans/same-file-rescue-plan.md` carries the candidates and the acceptance conditions.

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

## Everything 0.32.0 published that PR #494 does not

123 clusters across 71 fixtures.

| Kind | Fixtures | Clusters | Reading |
|---|---:|---:|---|
| Fixtures named for the false positive they hold — prologue, dissimilar functions, unrelated xUnit classes, shape-only pairs, lookalikes | 10 | 36 | 0.32.0 published shape alone; the content gate demands content. Improvement. |
| Clusters whose every occurrence is a single line | — | 49 | one line repeated is not extract-worthy. Improvement. |
| Same-file near-misses | 18 | 29 | the blockers above. `csharp-merge-drift`, `-manyholes`, `-readafter`, `-operatordrift`, `-typeconflict`, `-writtencontext`, `-writtenhole`, three `dart-forwarding-*`, `csharp-mixed-declaration-component`, `csharp-nonbijective-pair`, `csharp-type4`. Regression against 0.32.0. |
| Fixtures with their own pins | 25 | 51 | asserted by tests that are green. |

Every one of these losses is also a loss on merged main.

## Verification

- `cargo test --release -p deslop --test suite`: **457 passed, 0 failed, 5 ignored** — gh #369, #422, #489, #491 and #492, each with a plan and a registry row.
- `cargo test --release -p deslop-core --features live --test suite`: **187 passed, 0 failed**.
- The gh #492 pin run with `--ignored` still fails, for the false negative above.
- `cargo clippy --release --all-targets -- -D warnings`, `cargo fmt --check` and the repository duplication gate: pass.
- No assertion was deleted or relaxed. `cross_cluster_collapse::widest_same_declaration_view_is_the_published_finding` gained line extents and the shared statement run beside the byte spans it always checked.

## Release gate

1. Merge PR #494. It is strictly better than main and worse than it nowhere.
2. Answer gh #497 first: decide whether the four Dart forwarding findings are real. Every other question in this band depends on that answer.
3. Then gh #496, whether a same-file pair varying only literals should clear 0.85 on positional agreement alone, and gh #492 on top of it.
4. Restore the Dart forwarding controls to the positive contract their documentation and names state, or change both and say why.
5. Re-run the 206-fixture paired matrix and prove no occurrence set 0.32.0 published is missing.
6. Full CI matrix green on the final head.
