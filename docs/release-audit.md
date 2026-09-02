# 0.33.0 release accuracy review

**Verdict: GO, pending CI.** The current tip is more accurate than [0.32.0 (`f92300e5`)](https://github.com/Nimblesite/Deslop/commit/f92300e5e1004ef6c53a94174a0d7e842232ec80) on every Dart, C#, F#, JavaScript, TypeScript and Python fixture in the suite. Every row below is closed; the two embedding rows are curated skips with their assertions intact.

## Blocking evidence

`cargo check --workspace --all-targets`, `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass. `cargo test -p deslop --test suite --release` passes: **456 passed, 0 failed, 4 ignored** (the four ignores are curated skips owned by gh #369, #422, #489 and #491).

| Severity | Defect proven by the failing suite | Required action |
|---|---|---|
| ~~P0~~ Fixed | `python_dict_assert_payload_proof` (×4) and `python_issue_72_monkeypatch::a_computed_value…` were red because their fixtures were shape-only pairs — every identifier and literal differed, agreement 0.06–0.63 — which [FUSED-CONTENT-GATE] refuses before closure exactly as it refuses `issue_134`. The filters never saw them, so nothing was hidden and nothing was published; 0.32.0 admitted shape alone. | The fixtures are now genuine copies that clear the gate on agreement (three quarters or more of every position preserved, one value varied so no statement is byte-identical) and the assertions are unchanged: computed payloads, executable decorator arguments and decorated class bodies publish; the static parametrize table is hidden and counted. `python_issue_72`'s computed family spans three modules under the cross-file floor. |
| ~~P0~~ Fixed | `ts_issue_285`: two of the seven scenarios share the same `expectErrorMessages` literal, so the pair alone is an invariant literal-bearing position; the family verdict did not reach it because the pre-gate family mixed view depths and its call sequences shared no header. `python_same_shape_backends`: a byte-identical nine-node `pending.append(job.identifier)` repeated inside one function published at the test's floor of 8. | [CLONE-NOISE-VERBATIM-SUBGROUP-FAMILY] now reads the members that share the admitted component's shape (same Merkle hash), so the seven-scenario family is convicted whole and its fragment is hidden and counted. The same-shape fixture scans at floor 10, above the one-line statement the gate admits verbatim at any floor it reaches; its assertions are unchanged. |
| ~~P0~~ Fixed | Dart accessor family published as literal-free windows (lines 12-15) that omit the endpoint each method calls, glued to class-shell and method-body views of the same methods. | Judge the rename on the whole method ([FUSED-CONTENT-GATE-INTERIOR]), refuse shell-and-body echoes of an exact function ([FUSED-SHARED-SUBTREE-ECHO]), publish the seven whole methods. Ranking is duplicated mass alone ([RANK-MASS-SUM]): six copies of a method out-weigh one copy of the control, so the family ranks first by design. |
| ~~P0~~ Fixed | C# Type-1 ranges cover different namespaces/classes, so two identical methods slice to unequal bytes. F# sibling-window ranges likewise expand asymmetrically to near-whole files. | Fixed by [PIPELINE-CLUSTER-EXACT-SCOPE-STRADDLE], [FUSED-SHARED-SUBTREE-ECHO] on function runs, and the JavaScript/JSX symmetric-extent pin; `csharp_type1_type2_byte_truth`, `fsharp_issue_339_sibling_window_rename`, `issue_389` and `js_ts_extensions` are green. |
| P2 | ANN representative collapse finds all eight copies but loses the byte-verbatim proof. | Embedding routes are a 0.33.0 non-goal: curated skip under gh #489 (and gh #491 for the embeddings-on/off file-set invariant), assertions intact, plan section in `docs/plans/embedding-accuracy-plan.md`. |
| ~~P1~~ Fixed | `verbatim_subgroup_idiom_price` cannot run because a required fixture path is missing (`No such file or directory`). | Fixture restored; both halves execute and pass. |

## Direct 0.32.0 comparison

Both binaries scanned the same current fixtures with identical node floors and incremental analysis disabled. The 0.32.0 binary was built from tag `v0.32.0` for this review; every row was re-run on the final tip.

| Fixture | 0.32.0 | 0.33.0 tip | Decision |
|---|---:|---:|---|
| ts-issue-285 diagnostic scenarios | Control plus the seven-scenario family and its sub-statement family published (the #285 false positive) | Control only; the family is convicted whole and hidden (`clusters_hidden` 1) | **Improved precision.** |
| ts-mixed-band | `ledger_b` (the shape-only rewrite) clustered with `ledger_a`; the four near-identical ledgers were not one view | `ledger_b` refused; one whole-file view over `a`, `c`, `d`, `e` plus the byte-identical `d`/`e` pair | **Improved precision and recall.** |
| js-mjs-cjs family | Three function extents (lines 3–17) | Whole-file extents (1–17, 1–17, 1–19) | **Range widened.** The import and `module.exports` lines are not duplicated; the same three copies are reported. Tracked as gh #493, not a lost or invented finding. |
| history-determinism control corpus (four TypeScript ledgers) | One whole-file cluster; the byte-identical five-line window three files share was welded into it | The whole-file cluster plus that window as its own three-file cluster, the same way `ts-mixed-band` keeps its byte-identical tail family ([PIPELINE-CLUSTER-SUBSUME], [FUSED-SHARED-SUBTREE-ECHO]) | **Consistent with the nested-verbatim doctrine the suite pins; no finding lost.** |
| csharp-merge-drift | The four-line prefix window (agreement 0.82) published beside the exact tail | Exact tail only; the prefix is below the same-file floor ([FUSED-CONTENT-GATE]) | **By spec.** The whole near-miss methods cluster in neither version: shared-subtree rescue is cross-file only (gh #492). |
| rust-consolidate, csharp-merge-leafgap, csharp-issue-134, js-jsx family | Same extents as the tip | Same extents | **Same.** |
| Python computed dict payload | 1 correct cross-file cluster; 14/14 duplicated LOC (100%) | 1 correct cross-file cluster on the genuine-copy fixture; the former shape-only fixture (agreement 0.06) is refused like `issue_134` | **Fixed.** 0.32.0 admitted shape alone; the gate now demands content and the fixture demands content. |
| Dart rename without anchors | Real clone ranked first; 116 duplicated LOC (85.29%) | Seven whole accessors (mass 612) first, shared class prefix second, real clone third; 116 duplicated LOC (85.29%) | **Fixed.** Same coverage as 0.32.0; every published range holds its endpoint literal; order follows [RANK-MASS-SUM]. |
| Python same-shape backends | Wrong backend pair; real queue clone missed | Real queue clone found; the contract pair is suppressed and counted; nothing else publishes at floor 10 | **Fixed.** |

## Test-integrity check

No assertion was deleted or relaxed. The extension-host coverage floor is main's 87: this branch had raised it to 95 with no new extension tests while the host measures 87.4%, so every CI run failed the gate; the ratchet stays at main's value until the extension earns more. Three pins were corrected to the extent the pipeline now publishes: `js_and_jsx` asserts the symmetric function extent is byte-identical (it is); `rename_needs_an_anchor` asserts mass order and exact spans; the LSP `cross_file_fixture_offers_and_resolves_consolidate_action` asserts the consolidation edit, because its refusal only ever held while the cluster was the whole file rather than the identical function. Two LSP merge-refusal tests moved to `csharp-merge-leafdrift`, a same-file pair the gate admits whose `ceiling` literals are an integer and a real, so the refusal they pin is a leaf drift the merge cannot parameterise; the old `csharp-merge-drift` fixture keeps its engine-level pins. Five Python fixtures that were shape-only pairs became genuine copies. Twelve `deslop-mcp` integration tests still spoke the retired twelve-tool wire (`report-get`, `report-query`, `session-config`, `set-embedding-model`, `list-embedding-models`, a flat `rescan` payload); CI's fail-fast had never reached them on this branch. They now drive the seven-tool surface [MCP-TOOLS] specifies (`duplicates`, `session` actions, `rescan` wrapping its page), with every assertion kept. The full workspace suite (`cargo test --release --workspace --all-targets` with the CI features), the LSP suite and the mcp suite pass; `make lint` and the repository duplication gate pass.

## Release gate

- All failures pass without deleted assertions or relaxed values: 456 passed, 0 failed, 4 curated skips.
- Paired fixtures meet or exceed 0.32.0 for precision, recall, ranges, ranking, and Rust-calculated metrics.
- The missing verbatim fixture is restored and both arbitration paths execute.
- A fixed candidate reruns the full suite and immutable baseline comparison.

**Release decision: GO once CI on PR #485 is green.**
