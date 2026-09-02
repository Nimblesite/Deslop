# 0.33.0 release accuracy audit

**Verdict: GO once CI on PR #494 is green and the paired 0.32.0 scan of every fixture reports no lost finding.** The reviewed worktree is more accurate than 0.32.0 on every paired fixture below, including the same-file Type-3 false negative found during this audit; code, spec and pins cite the same ids.

## Audit point

- Baseline: 0.32.0, `f92300e5e1004ef6c53a94174a0d7e842232ec80`.
- Merged 0.33 line: `1ecfc99712b06d92f7c237acdc5b6421208be96f` (PR #485).
- Reviewed worktree: `fixes-2` (PR #494), which carries the #492 and #493 corrections.
- `0fca6f6d38badb95429e67c9ec2620d711c8cebe` does **not** fix #492. It changed cluster scope/election and the content gate, not shared-subtree candidate admission. The defect remained through merged `1ecfc997`; it is fixed only by the current worktree changes to candidate admission and rescue measurement.

## Blocking findings

| Severity | Finding | Evidence | Action |
|---|---|---|---|
| ~~P0~~ Fixed | Same-file Type-3 methods were false negatives. | The new black-box pin first failed with fragments only, then passed after valid same-file candidates reached rescue measurement. The authored pair measures structural overlap 0.8205. Its content support is 0.5625, proving that the 0.85 ordinary fused threshold must not be misapplied as the rescue content floor. | The configurable rescue agreement floor, the ordinary 0.85 fused threshold and the disjoint-range guard stay as they are; `[FUSED-SHARED-SUBTREE-SAME-FILE]` is specified in `fused.md`. The exact-function echo index now holds same-file clones too ([FUSED-SHARED-SUBTREE-ECHO]), so two sibling classes wrapping one byte-identical method publish the method and never the classes (`same_file_rescue`, new fixture `csharp-same-file-class-echo`). |
| P1 | Four ignored E2E tests leave accuracy/performance routes outside the release proof. | `embedding_perf::duplicate_subtree_embeddings_are_collapsed_before_ann`, `embedding_route_invariance::embeddings_on_reports_every_file_set_embeddings_off_reported`, `issue_343_sum_clamp_saturation::mid_band_pair_stays_visible_with_a_real_bucket`, and `perf_sample::perf_sample_bounded_scan`. | Keep them explicitly excluded from the 0.33 accuracy claim; do not describe an ignored route as green. No embedding work is part of this fix. |
| P1 | The previous audit overstated test integrity. | Several passing fixes changed fixtures or test inputs: Python shape-only fixtures became genuine copies, `python_same_shape_backends` moved to node floor 10, and LSP merge-refusal coverage moved off `csharp-merge-drift`. Assertions were retained, but “no test was weakened” is not proven by assertion count alone. | Preserve the new #492 black-box pin, and require the final paired-baseline run to prove the changed inputs did not hide a 0.32 finding. |

## Accuracy comparison with 0.32.0

| Area | 0.33.0 result | Decision |
|---|---|---|
| TypeScript noise and mixed-band fixtures | The #285 scenario family is hidden as noise; the shape-only `ledger_b` is refused while the four real ledger copies form the authored view. | Improved precision and recall. |
| Python computed payload and backend fixtures | Genuine computed copies publish; shape-only contracts are suppressed; the real queue clone survives at the intentional node floor. | Improved, subject to the changed-input baseline check above. |
| Dart accessor family | Seven whole methods publish; fragment echoes are refused; ranking uses duplicated mass. | Improved extent and ranking with 0.32 coverage retained. |
| C# and F# authored extents | Asymmetric namespace/file expansion is replaced by authored method extents; byte-truth pins pass. | Improved precision. |
| JavaScript `.js`/`.mjs`/`.cjs` family | The three byte-identical functions publish at function extent; the real whole-file `.js`/`.mjs` pair remains separate; `.cjs` import/export lines are not counted. The focused E2E passes. | #493 is fixed in the reviewed worktree; improved precision over the widened 0.33 output without losing the real pair. |
| C# same-file merge drift (`csharp-merge-drift`) | 0.32.0 publishes four fragment clusters — two-line pairs at L7-8/L9-10/L19-20/L25-26, seven single statements, L9-12/L25-28 and L5-8/L17-20 — and never the methods. The current worktree publishes `DriftLimits.cs:3-13` and `:15-29` as one cluster and absorbs every fragment view. | Improved recall and precision without weakening the content or range guards. |
| C# same-file read-after drift (`csharp-merge-readafter`) | 0.32.0 publishes the shared six-line prefix window (L5-10 / L16-21) beside a ten-occurrence single-statement family. The current worktree publishes `ApplyStandard` L3-12 and `ApplyPremium` L14-26 as one cluster; the window and the statements are its fragments. | Improved recall and precision; `widest_same_declaration_view_is_the_published_finding` is repinned from the window to the two methods under [FUSED-SHARED-SUBTREE-SAME-FILE]. |
| C# sibling classes wrapping one exact method (`csharp-same-file-class-echo`, new) | Both versions publish the byte-identical `Reconcile` method at L7-22 and L31-46. The same-file rescue could have admitted the two class shells and let enclosure widen the finding to L3-25 / L27-49; the echo index refuses them. | Same extents as 0.32.0; the new route is held to the echo rule. |

## Verification run

- `cargo check --workspace --all-targets`: pass.
- `cargo fmt --check`: pass.
- `javascript_family_clusters_across_js_mjs_and_cjs_extensions`: pass.
- `csharp_same_file_type3_reports_both_methods_in_one_cluster`: pass after first being observed red on the pre-fix code.
- Focused release-profile runs on the reviewed tree — `same_file_rescue`, `cross_cluster_collapse`, `type3_enclosing_method`, `js_ts_extensions`, `python_dict_assert_payload_proof`, `csharp_merged_clone_families`, `issue_389`, `fsharp_issue_339`, `incremental_multilang_golden`, `rename_needs_an_anchor`: pass. `cargo clippy --release -p deslop-core -p deslop --all-targets -- -D warnings`: pass.
- Full workspace/CI run: CI on PR #494; no green claim is made from an earlier commit.

## Release gate

1. ~~Synchronize `[FUSED-SHARED-SUBTREE-SAME-FILE]` in `docs/specs/fused.md`~~ — done; code, spec and pins cite the same id.
2. Re-run the paired 0.32.0/current fixture matrix with identical inputs and node floors.
3. Run the complete CI feature matrix and record exact pass/fail/ignored counts.

**Release decision: GO when gates 2 and 3 pass.**
