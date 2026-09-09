# Deslop dedup checkpoint

Generated mechanically at 2026-09-09T12:14:37.752452+00:00.

Status: stopped under AGENTS.md accuracy quarantine. The requested 5,000-line dedup target is not achieved.

The CLI test reports the real control clone correctly but also reports registry setup whose only varying work is path payloads. The incomplete statement classifier and binding/receiver data-flow walkers were replaced with mandated panics. No detector repair was attempted.

Issue: https://github.com/Nimblesite/Deslop/issues/534

Test: `cargo test -p deslop --test suite -- registry_call_payload_variation_keeps_only_authored_control`

The failure was observed before quarantine. No post-quarantine green test/CI claim is made. Calls into quarantined classification deliberately panic.

## Measured source changes

Counts include new helper files and concurrent changes since the saved baseline; they are not all attributable to this coordinator. Quarantine removals and accuracy fixtures are excluded from the dedup subtotal.

| File | Before | After | Net removed | Category |
| --- | ---: | ---: | ---: | --- |
| `clients/vscode/src/test/suite/context-menus.e2e.test.ts` | 212 | 202 | 10 | dedup/concurrent edit |
| `clients/vscode/src/test/unit/gitignore-prompt.unit.test.ts` | 254 | 232 | 22 | dedup/concurrent edit |
| `clients/vscode/src/test/unit/report-store.helpers.ts` | 95 | 100 | -5 | dedup/concurrent edit |
| `clients/vscode/src/test/unit/temp-file.helpers.ts` | 18 | 61 | -43 | dedup/concurrent edit |
| `clients/vscode/src/test/unit/tree.helpers.ts` | 133 | 174 | -41 | dedup/concurrent edit |
| `clients/vscode/src/test/unit/tree.metrics.unit.test.ts` | 424 | 404 | 20 | dedup/concurrent edit |
| `clients/vscode/src/test/unit/tree.session.unit.test.ts` | 139 | 125 | 14 | dedup/concurrent edit |
| `clients/vscode/src/test/unit/tree.topOffenders.unit.test.ts` | 1232 | 1191 | 41 | dedup/concurrent edit |
| `clients/vscode/src/webview/panels.ts` | 292 | 292 | 0 | dedup/concurrent edit |
| `crates/deslop-core/src/cluster_filters/calls/dataflow.rs` | 102 | 26 | 76 | accuracy quarantine/test (not dedup) |
| `crates/deslop-core/src/cluster_filters/calls/statements.rs` | 132 | 130 | 2 | accuracy quarantine/test (not dedup) |
| `crates/deslop-core/src/lang/csharp.rs` | 326 | 286 | 40 | dedup/concurrent edit |
| `crates/deslop-core/src/lang/csharp_merge.rs` | 172 | 154 | 18 | dedup/concurrent edit |
| `crates/deslop-core/src/lang/dart.rs` | 370 | 322 | 48 | dedup/concurrent edit |
| `crates/deslop-core/src/lang/python.rs` | 346 | 267 | 79 | dedup/concurrent edit |
| `crates/deslop-core/src/lang/rust_lang.rs` | 424 | 368 | 56 | dedup/concurrent edit |
| `crates/deslop-core/src/refactor/tables.rs` | 196 | 262 | -66 | dedup/concurrent edit |
| `crates/deslop-core/src/report_facts.rs` | 89 | 87 | 2 | dedup/concurrent edit |
| `crates/deslop-core/tests/live.rs` | 1461 | 1442 | 19 | dedup/concurrent edit |
| `crates/deslop-core/tests/pair_comparison.rs` | 293 | 302 | -9 | dedup/concurrent edit |
| `crates/deslop-lsp/tests/virtual_document.rs` | 209 | 195 | 14 | dedup/concurrent edit |
| `crates/deslop-mcp/tests/cli.rs` | 2785 | 2748 | 37 | dedup/concurrent edit |
| `crates/deslop/tests/cli/support.rs` | 268 | 213 | 55 | dedup/concurrent edit |
| `crates/deslop/tests/common/mod.rs` | 592 | 629 | -37 | dedup/concurrent edit |
| `crates/deslop/tests/common/scan_dir.rs` | 54 | 66 | -12 | dedup/concurrent edit |
| `crates/deslop/tests/cross_cluster_collapse.rs` | 402 | 397 | 5 | dedup/concurrent edit |
| `crates/deslop/tests/cross_language.rs` | 85 | 82 | 3 | dedup/concurrent edit |
| `crates/deslop/tests/csharp_issue_66_route_mapping.rs` | 75 | 65 | 10 | dedup/concurrent edit |
| `crates/deslop/tests/defaults.rs` | 528 | 527 | 1 | dedup/concurrent edit |
| `crates/deslop/tests/fixtures/rust-call-scaffolding/control_first.rs` | 0 | 13 | -13 | accuracy quarantine/test (not dedup) |
| `crates/deslop/tests/fixtures/rust-call-scaffolding/control_second.rs` | 0 | 13 | -13 | accuracy quarantine/test (not dedup) |
| `crates/deslop/tests/fixtures/rust-call-scaffolding/registry_first.rs` | 0 | 8 | -8 | accuracy quarantine/test (not dedup) |
| `crates/deslop/tests/fixtures/rust-call-scaffolding/registry_second.rs` | 0 | 8 | -8 | accuracy quarantine/test (not dedup) |
| `crates/deslop/tests/fsharp_deep_match_stack_overflow.rs` | 129 | 120 | 9 | dedup/concurrent edit |
| `crates/deslop/tests/issue_165_dart_generated_header.rs` | 101 | 80 | 21 | dedup/concurrent edit |
| `crates/deslop/tests/issue_168_deep_nesting_no_crash.rs` | 65 | 56 | 9 | dedup/concurrent edit |
| `crates/deslop/tests/issue_169_dart_const_registry.rs` | 105 | 79 | 26 | dedup/concurrent edit |
| `crates/deslop/tests/issue_169_dart_filter_precision.rs` | 104 | 94 | 10 | dedup/concurrent edit |
| `crates/deslop/tests/issue_190_data_table_demote.rs` | 221 | 199 | 22 | dedup/concurrent edit |
| `crates/deslop/tests/jwt_independent_verification_false_positive.rs` | 94 | 85 | 9 | dedup/concurrent edit |
| `crates/deslop/tests/python_generated_template_false_positive.rs` | 75 | 68 | 7 | dedup/concurrent edit |
| `crates/deslop/tests/python_issue_133_constant_table.rs` | 135 | 126 | 9 | dedup/concurrent edit |
| `crates/deslop/tests/rust_issue_147_iter_collect_idiom.rs` | 104 | 94 | 10 | dedup/concurrent edit |
| `crates/deslop/tests/rust_issue_150_mod_declarations.rs` | 36 | 25 | 11 | dedup/concurrent edit |
| `crates/deslop/tests/rust_issue_176_match_dispatch.rs` | 64 | 55 | 9 | dedup/concurrent edit |
| `crates/deslop/tests/rust_test_boilerplate_false_positive.rs` | 93 | 164 | -71 | accuracy quarantine/test (not dedup) |
| `scripts/reports/duplication_anatomy.py` | 0 | 143 | -143 | dedup/concurrent edit |

Measured dedup/concurrent-edit net reduction: 290 physical source lines. This does not credit quarantine deletions.

## Direct validation logs (before quarantine)

### `target/deslop-astra1-dedup/lang-refactor-tests.log`

```text
test refactor_merge::dart_leafgap_merges_to_golden ... ok
test refactor_content_gate::byte_proven_cross_file_family_reaches_consolidation_resolution ... ok
test refactor_merge::wire_edit_uri_is_rfc8089_for_a_canonicalised_absolute_path ... ok
test refactor_merge::three_site_merge_defaults_trailing_parameter ... ok
test refactor_merge_refusals::comment_drift_refuses_via_residual_proof ... ok
test refactor_merge::wire_edit_uri_is_rfc8089_for_the_platform_absolute_path ... ok
test refactor_consolidate::rust_cross_file_definition_consolidates_and_compiles ... ok
test refactor_merge_refusals::python_merge_always_refuses ... ok
test refactor_merge_refusals::operator_drift_publishes_only_its_verbatim_tail ... ok
test refactor_merge_refusals::declared_inside_read_after_refuses ... ok
test refactor_merge_refusals::literal_type_conflict_routes_to_ai_or_human ... ok
test refactor_merge::csharp_leafgap_plan_is_deterministic ... ok
test live_merge_plan::live_session_consolidates_cross_file_cluster ... ok
test refactor_merge_refusals::written_context_variable_refuses ... ok
test refactor_merge_refusals::dart_written_context_variable_refuses ... ok
test refactor_merge_refusals::written_hole_identifier_refuses ... ok
test refactor_merge_refusals::structural_drift_routes_to_ai_or_human ... ok
test refactor_merge::rust_leafgap_merges_and_compiles ... ok
test refactor_merge_refusals::too_many_holes_refuse ... ok
test refactor_extract_negative::cross_file_occurrences_refused ... ok
test refactor_content_gate::shape_only_cross_file_family_is_rejected_before_closure ... ok

test result: ok. 96 passed; 0 failed; 0 ignored; 0 measured; 97 filtered out; finished in 0.73s

```

### `target/deslop-astra1-dedup/lint.log`

```text
  duration_ms: 1.519417
  type: 'test'
  ...
# Subtest: [CI-COVERAGE-ISOLATION] the clean is never narrowed to the profiles
ok 2 - [CI-COVERAGE-ISOLATION] the clean is never narrowed to the profiles
  ---
  duration_ms: 0.09225
  type: 'test'
  ...
# Subtest: [CI-COVERAGE-ISOLATION] reporting stays a separate command over the cleaned collection
ok 3 - [CI-COVERAGE-ISOLATION] reporting stays a separate command over the cleaned collection
  ---
  duration_ms: 0.086667
  type: 'test'
  ...
1..3
# tests 3
# suites 0
# pass 3
# fail 0
# cancelled 0
# skipped 0
# todo 0
# duration_ms 39.1515
```

### `target/deslop-astra1-dedup/registry-failing-test.log`

```text
   Compiling deslop v0.0.0-dev (/Users/christianfindlay/Documents/Code/DeslopWorktree/crates/deslop)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.50s
     Running tests/suite.rs (target/debug/deps/suite-f95eac62ee1fae7d)

running 1 test
test rust_test_boilerplate_false_positive::registry_call_payload_variation_keeps_only_authored_control ... FAILED

failures:

---- rust_test_boilerplate_false_positive::registry_call_payload_variation_keeps_only_authored_control stdout ----

thread 'rust_test_boilerplate_false_positive::registry_call_payload_variation_keeps_only_authored_control' (5277115) panicked at crates/deslop/tests/rust_test_boilerplate_false_positive.rs:159:5:
assertion `left == right` failed: registry setup only varies path payloads; it is call scaffolding, not a clone
  left: ["control_first.rs", "control_second.rs", "registry_first.rs", "registry_second.rs"]
 right: ["control_first.rs", "control_second.rs"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    rust_test_boilerplate_false_positive::registry_call_payload_variation_keeps_only_authored_control

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 496 filtered out; finished in 1.33s

error: test failed, to rerun pass `-p deslop --test suite`
```

## Table refactor verification

```json
{
  "rows_verified": 70,
  "fields_unchanged": true,
  "files": [
    "crates/deslop-core/src/lang/csharp.rs",
    "crates/deslop-core/src/lang/csharp_merge.rs",
    "crates/deslop-core/src/lang/dart.rs",
    "crates/deslop-core/src/lang/rust_lang.rs",
    "crates/deslop-core/src/lang/python.rs"
  ]
}
```

## Delegated evidence

- Rust CLI worker: `target/deslop-opus-tests/handoff.json` and `target/deslop-astra1-dedup/opus-tests-handoff.stream.jsonl`.
- VSCode worker: `reports/vscode-dedup-loc.md` and `target/deslop-astra1-dedup/opus-vscode.stream.jsonl`.
- Both launched Opus workers acknowledged pause and released their locks. DeslopOpus1 was notified separately through TMC.
- The VSCode handoff records a remaining verification gap concerning comments between AST-rewritten setup statements; its full handoff is in the stream log.

## Preserved reproduction

- `crates/deslop/tests/fixtures/rust-call-scaffolding/`
- `target/deslop-astra1-dedup/registry-controls.json`
- `target/deslop-astra1-dedup/registry-failing-test.log`

## Dedup workflow checkpoint

- [x] Deslop reachable; duplicate surface inventoried.
- [x] `make lint` completed successfully before quarantine.
- [x] Bounded dedup batches applied and tested.
- [ ] All duplicates merged / 5,000 lines removed.
- [ ] Final ci-prep green: prevented by mandatory accuracy STOP.
