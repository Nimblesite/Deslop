# 0.33.0 release accuracy review

**Verdict: NO-GO.** Current tip `4c4f5d9ed64cdf745b3e1109ddf0b38d8bfaab60` includes `0fca6f6d38badb95429e67c9ec2620d711c8cebe` and is not more accurate than [0.32.0 (`f92300e5`)](https://github.com/Nimblesite/Deslop/commit/f92300e5e1004ef6c53a94174a0d7e842232ec80).

## Blocking evidence

`cargo check --workspace --all-targets` passes. `cargo test -p deslop --test suite --release` does not: **444 passed, 14 failed, 2 ignored**.

| Severity | Defect proven by the failing suite | Required action |
|---|---|---|
| P0 | Python executable payloads are hidden or missed: computed dictionary values, executable decorator arguments, decorated class-body logic, and computed monkeypatch values. Static-decorator suppression also disappears from `clusters_hidden`. | Make the language-specific idiom proof recursively reject executable values and decorated class bodies; preserve detection telemetry for genuinely suppressed static payloads. |
| P0 | False positives remain in Python same-file queue statements and TypeScript diagnostic scaffolding. | Reject same-file scaffolding before publication while retaining the positive cross-file controls and exact cluster-count assertions. |
| P0 | Dart accessor family published as literal-free windows (lines 12-15) that omit the endpoint each method calls, glued to class-shell and method-body views of the same methods. | Judge the rename on the whole method ([FUSED-CONTENT-GATE-INTERIOR]), refuse shell-and-body echoes of an exact function ([FUSED-SHARED-SUBTREE-ECHO]), publish the seven whole methods. Ranking is duplicated mass alone ([RANK-MASS-SUM]): six copies of a method out-weigh one copy of the control, so the family ranks first by design. |
| P0 | C# Type-1 ranges cover different namespaces/classes, so two identical methods slice to unequal bytes. F# sibling-window ranges likewise expand asymmetrically to near-whole files. | Select one symmetric authored extent before closure and derive metrics/IDs only from final ranges. Keep the strengthened byte-equality and exact-range assertions. |
| P0 | ANN representative collapse finds all eight copies but loses the byte-verbatim proof. | Carry byte identity through collapse and assert it on the published cluster. |
| P1 | `verbatim_subgroup_idiom_price` cannot run because a required fixture path is missing (`No such file or directory`). | Restore the complete fixture and make both cross-file and intra-file halves execute; do not ignore or relax the test. |

## Direct 0.32.0 comparison

Both binaries scanned the same current fixtures with identical node floors and incremental analysis disabled.

| Fixture | 0.32.0 | 0.33.0 tip | Decision |
|---|---:|---:|---|
| Python computed dict payload | 1 correct cross-file cluster; 14/14 duplicated LOC (100%) | 0 visible clusters; 0/14 duplicated LOC (0%) | **Recall regression.** A real clone is hidden. |
| Dart rename without anchors | Real clone ranked first; 116 duplicated LOC (85.29%) | Seven whole accessors (mass 612) first, shared class prefix second, real clone third; 116 duplicated LOC (85.29%) | **Fixed.** Same coverage as 0.32.0; every published range holds its endpoint literal; order follows [RANK-MASS-SUM]. |
| Python same-shape backends | Wrong backend pair; real queue clone missed | Real queue clone found, but an extra same-file cluster remains | Improved recall, still fails precision and release ground truth. |

## Test-integrity check

The latest commits fixed the chained-dict false positive, restored the VSIX coverage threshold to **95%**, and made the incremental golden pass. The C# replacement test is stronger—not weakened—and now exposes the bad ranges. No resolved finding is carried forward above. The broken verbatim fixture is a new test-execution regression.

## Release gate

- All 14 failures pass without deleted assertions, relaxed values, or new ignores.
- Paired fixtures meet or exceed 0.32.0 for precision, recall, ranges, ranking, and Rust-calculated metrics.
- The missing verbatim fixture is restored and both arbitration paths execute.
- A fixed candidate reruns the full suite and immutable baseline comparison.

**Release decision: NO-GO.**
