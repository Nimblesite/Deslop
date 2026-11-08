# [OUTPUT-REPORT-REUSE] Report generation verification

Commands below were executed against the feature branch. No tests were skipped or assertions weakened; cache-layer isolation removes only the completed-report entry in disposable fingerprint-store fixtures. Coverage collection stops on the existing quarantined ML tests, so no coverage percentage or green full-CI result is claimed.

## Focused report cases from the full CLI gate

```text

test cli::from_report::from_report_preserves_current_report_fields_issue_85 ... ok

test cli::from_report::cross_cluster_collapse_removes_occurrence_subset_clusters ... ok

test cli::from_report::from_report_preserves_mass_and_kind_in_html ... ok

test cli::from_report::from_report_rejects_retired_bucket_wire ... ok

test cli::from_report::include::replay_preserves_all_clone_kinds_when_the_original_include_field_is_absent ... ok

test cli::from_report::include::replay_without_gates_does_not_require_a_configuration_file ... ok

test cli::from_report::include::replay_resolves_current_kind_limits_without_changing_measured_clusters ... ok

test cli::from_report::include::replay_requires_kind_measurements_when_kind_limits_are_configured ... ok

test cli::from_report::reuse::unchanged_exports_reuse_calculations_and_source_changes_invalidate_them ... ok

test cli::from_report::freshness::completed_reports_track_membership_and_content_even_with_preserved_mtime ... ok

test cli::from_report::freshness::completed_reports_invalidate_for_policy_corruption_and_cache_opt_out ... ok

```

## CLI full release gate — BASELINE FAIL

`CARGO_BUILD_JOBS=2 make test`

```text
    Finished `release` profile [optimized] target(s) in 58.37s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 12 ignored; 0 measured; 0 filtered out; finished in 0.00s
test cli::detection::osprey_flavors::a_repository_of_only_malformed_osprey_ml_still_reports ... FAILED
test cli::detection::osprey_flavors::malformed_osprey_ml_is_skipped_visibly_and_the_scan_continues ... FAILED
test cli::detection::osprey_recovery::intact_osprey_duplicates_survive_an_unfinished_neighboring_declaration ... FAILED
test result: FAILED. 603 passed; 3 failed; 4 ignored; 0 measured; 0 filtered out; finished in 107.95s
```

## Remaining core/LSP workspace tests — BASELINE FAIL

`cargo test --release --workspace --exclude deslop --all-targets --features deslop-core/live,deslop-lsp/profiling`

```text
    Finished `release` profile [optimized] target(s) in 1.60s
test result: ok. 354 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.81s
test result: ok. 222 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.14s
test result: ok. 74 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.02s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test live_osprey_unparsed::a_repository_of_only_malformed_osprey_ml_still_boots ... FAILED
test live_osprey_unparsed::broken_osprey_ml_file_is_skipped_without_killing_the_live_server ... FAILED
test notifications::pair_compare::completed_discovery_is_reused_by_repeated_pair_comparisons ... FAILED
test result: FAILED. 94 passed; 3 failed; 1 ignored; 0 measured; 0 filtered out; finished in 22.45s
```

## MCP and test-support remainder — PASS

`cargo test --release -p deslop-mcp -p deslop-test-support --all-targets --features deslop-core/live`

```text
    Finished `release` profile [optimized] target(s) in 1m 11s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 146 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.09s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 211 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.30s
```

## Embedding/fingerprint cache isolation — PASS

`cargo test --release -p deslop --test suite cli::embedding_ollama::ollama_incremental_plus_embeddings_second_run_hits_both_caches -- --exact`

```text
    Finished `release` profile [optimized] target(s) in 2m 10s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 609 filtered out; finished in 2.91s
```

## Workspace release static analysis — PASS

`cargo clippy --release --workspace --all-targets --features deslop-core/live,deslop-lsp/profiling -- -D warnings`

```text
    Finished `release` profile [optimized] target(s) in 12.21s
```

## Native VS Code report command — PASS

`node node_modules/@vscode/test-cli/out/bin.mjs --label workspace --code-version 1.144.0 --run out/test/suite/report-exports.e2e.test.js --fail-zero`

```text
    ✔ HTML button reuses the live calculations and one report tab
  1 passing (62ms)
Exit code:   0
```

## VS Code compilation — PASS

`npm run compile`

```text

> deslop-live@0.0.0-dev compile
> tsc -p ./

```

## VS Code report-test lint — PASS

`node node_modules/eslint/bin/eslint.js src/test/suite/report-exports.e2e.test.ts`

```text

```

## Coverage collection — BLOCKED BY BASELINE TESTS

`CARGO_BUILD_JOBS=2 make coverage`

```text
info: cargo-llvm-cov currently setting cfg(coverage); you can opt-out it by passing --no-cfg-coverage
    Finished `release` profile [optimized] target(s) in 9m 12s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 12 ignored; 0 measured; 0 filtered out; finished in 0.00s
test cli::detection::osprey_flavors::a_repository_of_only_malformed_osprey_ml_still_reports ... FAILED
test cli::detection::osprey_flavors::malformed_osprey_ml_is_skipped_visibly_and_the_scan_continues ... FAILED
test cli::detection::osprey_recovery::intact_osprey_duplicates_survive_an_unfinished_neighboring_declaration ... FAILED
test result: FAILED. 603 passed; 3 failed; 4 ignored; 0 measured; 0 filtered out; finished in 113.74s
```

## Source and implementation

- `crates/deslop/src/report_cache.rs`: content/settings validation, checksum and atomic bounded persistence.
- `crates/deslop/tests/cli/from_report/reuse.rs` and `freshness.rs`: rendered-output and invalidation journeys.
- `clients/vscode/src/test/suite/report-exports.e2e.test.ts`: real button commands, complete snapshot/generation equality, and one report tab.
- `crates/deslop-lsp/tests/live/report_exports.rs`: real stdio HTML/JSON commands before and after an edit.
- See `report-reuse-duplication.md` and `report-reuse-cleanup.md` for the focused duplicate scan and disk evidence.

## Format and baseline revision

`cargo fmt --all -- --check` completed successfully. The feature branch was fetched and merged with `origin/main` again after verification; Git reported it already up to date.
