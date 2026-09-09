# Commit f92300e5 review checks

Generated mechanically from captured Cargo output and a tree-sitter comparison of failed merge output against the committed goldens.

Target: `f92300e5e1004ef6c53a94174a0d7e842232ec80`, built from its GitHub source archive under `target/commit-f92300e5-review/`. Current-workspace checks ran with HEAD `a0303c1fccaa41b9c5c959eaa7112933a4e4d7b1` and concurrent edits.

## Exact-commit test results

| Check | Passed | Failed | Ignored | Evidence |
| --- | ---: | ---: | ---: | --- |
| Detector and exclusion checks | 17 | 0 | 1 | [log](../target/commit-f92300e5-review/historical-focused-assets.log) |
| Subsumption, AST access, and merge checks | 33 | 0 | 0 | [log](../target/commit-f92300e5-review/historical-supported-1.log) |
| Live dependency updates | 2 | 0 | 0 | [log](../target/commit-f92300e5-review/historical-supported-3.log) |
| AST goldens and Type-2/Type-3 detection | 21 | 0 | 0 | [log](../target/commit-f92300e5-review/historical-final-1.log) |
| Embedding evidence and Python assertion filtering | 11 | 0 | 3 | [log](../target/commit-f92300e5-review/historical-final-2.log) |
| **Total** | **84** | **0** | **4** | |

Ignored tests were not executed. Their captured reasons follow; these cases are outside the passing evidence. Full CI and full workspace coverage were not run.

```text
test an_embedding_only_pair_does_not_join_occurrences_of_different_size ... ignored, GH #369: the size guard removed the incoherent member, but the scan still renders two families instead of one — the surviving extra is an embedding-only false positive over the same fixture. Assertions are intact — run with `-- --ignored`.
test embeddings_on_never_moves_a_reported_bucket ... ignored, GH #356: ollama-provider suite, excluded from the release gate. `csharp-type3` publishes Delta.cs/Epsilon.cs as two `structural_only` clusters with embeddings off and one `same_behavior` cluster with them on — the bucket follows the discovery route, not the code. BRANCH_REVIEW.md requires this stay red rather than be baselined. Assertions are intact — run with `-- --ignored`.
test embeddings_on_reports_every_file_set_embeddings_off_reported ... ignored, GH #356: ollama-provider suite, excluded from the release gate. `ts-mixed-band` publishes a four-file clone with embeddings off and nothing with them on — ANN bridges mutate structural components before measurement (`session/render.rs`). BRANCH_REVIEW.md requires this stay red rather than be baselined. Assertions are intact — run with `-- --ignored`.
test mid_band_cluster_confidence_never_exceeds_its_strongest_axis ... ignored, GH #369: the two-ledger scan renders two embedding-only false positives and hides the real clone, so this reports cluster_count 2. Both false pairs carry structural = 0 and token_jaccard = 0 and survive on MockOllama's length-residue cosine alone. Assertions are intact — run with `-- --ignored`.
```

## Current-workspace test results

| Command | Exit | Evidence |
| --- | ---: | --- |
| `cargo test -p deslop --test suite -- cross_cluster_enclosure dart_forwarding_fail_open declaration_family embedding_route_invariance pair_size_coherence python_dict_assert cli::cache_and_debug` | 0 | test result: ok. 46 passed; 0 failed; 1 ignored; 0 measured; 448 filtered out; finished in 34.59s |
| `cargo test -p deslop --test suite -- config_include_dependencies issue_342_scan_root_under_excluded_ancestor embedding_non_finite js_ts_clone_buckets python_literal_variation_calls` | 0 | test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 473 filtered out; finished in 4.78s |
| `cargo test -p deslop-core --features live --test suite -- refactor_ast_access refactor_merge live_merge_plan cluster_subsumption cluster_overlap_collapse` | 101 | test result: FAILED. 39 passed; 3 failed; 0 ignored; 0 measured; 151 filtered out; finished in 18.95s |

## Current merge-golden failure analysis

Tree-sitter leaf tokens compared after normalizing only the first declared helper identifier.

| Test | Golden helper | Generated helper | All remaining AST tokens equal |
| --- | --- | --- | --- |
| `csharp_leafgap_cluster_merges_to_golden` | `MergedFromCluster_7023e4` | `MergedFromCluster_d01828` | True |
| `dart_leafgap_merges_to_golden` | `mergedFromCluster_22921e` | `mergedFromCluster_462cc8` | True |
| `rust_leafgap_merges_and_compiles` | `merged_from_cluster_ff0d17` | `merged_from_cluster_5c3b57` | True |

## Reproduction commands

Commands below are retained from the original execution transcript. Setup attempts with missing archived CSS or missing `test-support`, and the AST filter that matched zero tests, were superseded by the populated passing runs above.

Command: `cargo test --manifest-path /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/f92300e5e1004ef6c53a94174a0d7e842232ec80/Cargo.toml --target-dir /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/build -p deslop --test cross_cluster_enclosure --test declaration_family_mixed_component --test declaration_family_plurality --test pair_size_coherence --test config_include_dependencies --test dart_forwarding_fail_open`
Command: `cargo test --manifest-path /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/f92300e5e1004ef6c53a94174a0d7e842232ec80/Cargo.toml --target-dir /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/build -p deslop --test cross_cluster_enclosure --test declaration_family_mixed_component --test declaration_family_plurality --test pair_size_coherence --test config_include_dependencies --test dart_forwarding_fail_open`
Command: `cargo test --manifest-path /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/f92300e5e1004ef6c53a94174a0d7e842232ec80/Cargo.toml --target-dir /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/build -p deslop-core --features live --test refactor_merge --test refactor_merge_refusals --test refactor_ast_access --test live_merge_plan --test cluster_subsumption --test cluster_overlap_collapse`
Command: `cargo test --manifest-path /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/f92300e5e1004ef6c53a94174a0d7e842232ec80/Cargo.toml --target-dir /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/build -p deslop-core --features live,test-support --test refactor_merge --test refactor_merge_refusals --test refactor_ast_access --test live_merge_plan --test cluster_subsumption --test cluster_overlap_collapse`
Command: `cargo test --manifest-path /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/f92300e5e1004ef6c53a94174a0d7e842232ec80/Cargo.toml --target-dir /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/build -p deslop --test cli ast_golden`
Command: `cargo test --manifest-path /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/f92300e5e1004ef6c53a94174a0d7e842232ec80/Cargo.toml --target-dir /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/build -p deslop-lsp --features deslop-core/test-support --test dependency_reactivity`
Command: `cargo test --manifest-path /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/f92300e5e1004ef6c53a94174a0d7e842232ec80/Cargo.toml --target-dir /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/build -p deslop --test cli -- debug_ast_dump_matches_committed_golden detects_type2_clone detects_type3_clone`
Command: `cargo test --manifest-path /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/f92300e5e1004ef6c53a94174a0d7e842232ec80/Cargo.toml --target-dir /Users/christianfindlay/Documents/Code/DeslopWorktree/target/commit-f92300e5-review/build -p deslop --test embedding_route_invariance --test embedding_non_finite --test issue_343_sum_clamp_saturation --test python_dict_assert_payload_proof --test python_dict_assert_reach --test python_dict_assert_rhs_logic`
