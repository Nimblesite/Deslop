# 0.33.0 release accuracy review

**Verdict: NO-GO.** Current tip `9645392` is more accurate than [0.32.0 (`f92300e5`)](https://github.com/Nimblesite/Deslop/commit/f92300e5e1004ef6c53a94174a0d7e842232ec80) on every Dart, C#, F#, JavaScript and TypeScript fixture in the suite, but seven Python and TypeScript pins are still red, and five of them contradict the content-gate spec rather than the code.

## Blocking evidence

`cargo check --workspace --all-targets`, `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass. `cargo test -p deslop --test suite --release` does not: **449 passed, 7 failed, 4 ignored** (the four ignores are curated skips owned by gh #369, #422, #489 and #491).

| Severity | Defect proven by the failing suite | Required action |
|---|---|---|
| P0 | **Spec conflict, decision needed.** `python_dict_assert_payload_proof` (×4) and `python_issue_72_monkeypatch::a_computed_value…` expect shape-identical test functions whose every identifier and literal differs to be *admitted* and then judged by the noise filters (visible for computed payloads, hidden-and-counted for static ones). [FUSED-CONTENT-GATE] refuses those pairs before closure: measured agreement 0.06–0.33 and rename consistency 0.0 cross-file (floor 0.70), agreement 0.63 and rename 0.12 same-file (floor 0.85) — the same rule that keeps `issue_134` at zero clusters. The filters never see them, so `clusters_hidden` is 0 and nothing is published. | Either the spec admits shape-only test scaffolding at the gate (and the 134 class returns), or these five tests are rewritten to assert refusal at the gate. Not a code defect; the fixtures share only one identifier (`reconcile_amount`, `monkeypatch`) beyond structure. |
| P0 | `python_same_shape_backends`: a byte-identical one-line `pending.append(job.identifier)` repeated inside one function publishes as a 9-node same-file cluster. `ts_issue_285`: two of the seven scenarios share the same `expectErrorMessages` literal, so the pair is an invariant literal-bearing position under [CLONE-NOISE-LITERAL-VARIATION-CALLS] and is not suppressed; the family-level verdict ([CLONE-NOISE-VERBATIM-SUBGROUP-FAMILY]) does not reach it because the family mixes view depths and its call sequences do not share a header. | No spec names a minimum statement count for a same-file finding; one is needed before the one-liner can be refused. For 285, judge the literal-variation family per view depth so a fragment of a convicted family is hidden with it. |
| ~~P0~~ Fixed | Dart accessor family published as literal-free windows (lines 12-15) that omit the endpoint each method calls, glued to class-shell and method-body views of the same methods. | Judge the rename on the whole method ([FUSED-CONTENT-GATE-INTERIOR]), refuse shell-and-body echoes of an exact function ([FUSED-SHARED-SUBTREE-ECHO]), publish the seven whole methods. Ranking is duplicated mass alone ([RANK-MASS-SUM]): six copies of a method out-weigh one copy of the control, so the family ranks first by design. |
| ~~P0~~ Fixed | C# Type-1 ranges cover different namespaces/classes, so two identical methods slice to unequal bytes. F# sibling-window ranges likewise expand asymmetrically to near-whole files. | Fixed by [PIPELINE-CLUSTER-EXACT-SCOPE-STRADDLE], [FUSED-SHARED-SUBTREE-ECHO] on function runs, and the JavaScript/JSX symmetric-extent pin; `csharp_type1_type2_byte_truth`, `fsharp_issue_339_sibling_window_rename`, `issue_389` and `js_ts_extensions` are green. |
| P2 | ANN representative collapse finds all eight copies but loses the byte-verbatim proof. | Embedding routes are a 0.33.0 non-goal: curated skip under gh #489 (and gh #491 for the embeddings-on/off file-set invariant), assertions intact, plan section in `docs/plans/embedding-accuracy-plan.md`. |
| ~~P1~~ Fixed | `verbatim_subgroup_idiom_price` cannot run because a required fixture path is missing (`No such file or directory`). | Fixture restored; both halves execute and pass. |

## Direct 0.32.0 comparison

Both binaries scanned the same current fixtures with identical node floors and incremental analysis disabled.

| Fixture | 0.32.0 | 0.33.0 tip | Decision |
|---|---:|---:|---|
| Python computed dict payload | 1 correct cross-file cluster; 14/14 duplicated LOC (100%) | 0 visible clusters; 0/14 duplicated LOC (0%) | **Spec decision.** The two tests share one callee name and their structure; every other identifier and literal differs (agreement 0.06). 0.32.0 admitted shape alone; [FUSED-CONTENT-GATE] does not. Whether that pair is a clone is the open question above. |
| Dart rename without anchors | Real clone ranked first; 116 duplicated LOC (85.29%) | Seven whole accessors (mass 612) first, shared class prefix second, real clone third; 116 duplicated LOC (85.29%) | **Fixed.** Same coverage as 0.32.0; every published range holds its endpoint literal; order follows [RANK-MASS-SUM]. |
| Python same-shape backends | Wrong backend pair; real queue clone missed | Real queue clone found; the contract pair is suppressed and counted; one byte-identical one-line statement repeated in one function still publishes | Improved recall and precision on the contract pair; the one-liner needs a minimum-extent rule. |

## Test-integrity check

The latest commits fixed the chained-dict false positive, restored the VSIX coverage threshold to **95%**, and made the incremental golden pass. The C# replacement test is stronger—not weakened—and now exposes the bad ranges. No resolved finding is carried forward above. The broken verbatim fixture is a new test-execution regression.

## Release gate

- All 7 remaining failures pass without deleted assertions or relaxed values, after the content-gate decision above is taken and recorded in the spec.
- Paired fixtures meet or exceed 0.32.0 for precision, recall, ranges, ranking, and Rust-calculated metrics.
- The missing verbatim fixture is restored and both arbitration paths execute.
- A fixed candidate reruns the full suite and immutable baseline comparison.

**Release decision: NO-GO.**
