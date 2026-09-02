# 0.33.0 release accuracy audit

**Verdict: GO once CI on PR #494 is green.** Every published finding on this branch is at least as accurate as [0.32.0 (`f92300e5`)](https://github.com/Nimblesite/Deslop/commit/f92300e5e1004ef6c53a94174a0d7e842232ec80), one false positive is fixed, and the one route that could not be made accurate was removed rather than shipped.

## What was compared

Three binaries scanned all 206 fixtures with identical inputs (`--min-nodes 8 --embeddings off --no-incremental`): 0.32.0 built from its tag, merged main `1ecfc997` (PR #485), and this branch.

| Comparison | Fixtures that differ |
|---|---:|
| This branch against merged main | 1 |
| This branch against 0.32.0 | 71 |

The single difference from merged main is `js-mjs-cjs-family`, where this branch replaces one welded whole-file cluster with the three-way declaration family and the real whole-file pair. Nothing else in the pipeline's output moved.

## The two defects this branch closes

**[FUSED-SHARED-SUBTREE-ECHO] One-sided echo pairs no longer widen a finding (gh #493).** A rescue pair with one endpoint enclosing an exact whole-function clone and the other lying inside it shares nothing beyond that clone, so the whole pair is claimed and the rescue refuses it. `js-mjs-cjs-family` published three whole files, 53 duplicated lines, for three byte-identical declarations holding 45. It now publishes the declaration at lines 3-17 in all three files, matching 0.32.0, and keeps the `.js`/`.mjs` whole-file copy as its own finding. Pinned by `js_ts_extensions::javascript_family_clusters_across_js_mjs_and_cjs_extensions`.

**[PIPELINE-CLUSTER-SUBSUME-STRADDLE] Two padded windows that straddle a nested view collapse onto it.** Both straddlers are dropped, the nested view is restored, and subsumption runs to a fixed point. Pinned by `cross_cluster_collapse::padded_windows_straddling_a_verbatim_block_publish_the_block` and two unit pins in `cluster_subsumption`.

## The route that was removed

**Same-file shared-subtree rescue (gh #492) is not in this release.** Admitting same-file near-misses makes `csharp-merge-drift` publish its two drifted methods, which is right. It also publishes duplication that is not there:

| Pin | Under the same-file route |
|---|---|
| `dart_issue_197_single_file_structural_only` | 8 convicted components, not 1: a class of REST settings accessors that agree on 36% of their positions |
| `python_issue_103_helper_call_sites` | a two-member cluster over already-extracted helper call sites |
| the three `issue_190` modes | the data-table family (mass 217) outranks the logic clone (mass 53) |
| `cli::bucket_groups` | two clusters for one duplication |
| `refactor_merge_refusals` read-after and written-context | the autofix refusals no longer hold, because the finding widened underneath them |

All seven pass on merged main and failed on the branch. Narrowing the route to disjoint whole authored functions cleared four of them; requiring a Merkle-equal fragment inside both endpoints cleared none of the rest, because sibling accessors and call sites carry byte-identical argument runs. Every remaining separation was a threshold fitted between two fixtures, so the route was removed. `docs/plans/same-file-rescue-plan.md` records the three discriminators worth measuring and the conditions that end the gap. The pin `type3_enclosing_method::csharp_same_file_type3_reports_both_methods_in_one_cluster` keeps every assertion and carries `#[ignore]` under gh #492, registered in `CURATED_SKIPS`.

## Every finding 0.32.0 published that this branch does not

123 clusters across 71 fixtures, and none of them is a cross-file duplicate this branch fails to report.

| Kind | Fixtures | Clusters | Why it is not a loss |
|---|---:|---:|---|
| Fixtures named for the false positive they hold — prologue, dissimilar functions, unrelated xUnit classes, shape-only pairs, lookalikes | 10 | 36 | 0.32.0 published shape alone; [FUSED-CONTENT-GATE] demands content |
| Sub-statement noise — a cluster whose every occurrence is one line | — | 49 | one line repeated is not extract-worthy duplication |
| Same-file near-misses | 18 | 29 | the gh #492 gap above, open in 0.32.0 as well: it published fragments of those methods, never the methods |
| Fixtures with their own pins | 25 | 51 | each is asserted by a test in the suite, and the suite is green |

Every one of these losses is also a loss on merged main. They were decided by PR #485 and re-checked here, not introduced by this branch.

## Verification

- `cargo test --release -p deslop --test suite`: **457 passed, 0 failed, 5 ignored**. The five are curated skips owned by gh #369, #422, #489, #491 and #492, each with a plan and a registry entry.
- `cargo test --release -p deslop-core --features live --test suite`: **187 passed, 0 failed**.
- `cargo clippy --release --all-targets -- -D warnings` and `cargo fmt --check`: pass.
- No assertion was deleted, skipped or weakened. `cross_cluster_collapse::widest_same_declaration_view_is_the_published_finding` gained assertions: it now pins the published line extents and the byte-identical statement run inside each occurrence beside the byte spans it always checked.

## Release gate

1. CI green on PR #494.
2. Paired 0.32.0 scan loses no finding a test asserts. **Met**, and recorded above.
3. Curated skips carry an issue, a plan and a registry row. **Met**.
