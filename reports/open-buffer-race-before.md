# Background refresh source-currency assertion

Source-currency protection was temporarily disabled; the original implementation was restored after this run.

Command: `cargo test -p deslop-core --features live --test suite live::embedding_refresh_keeps_latest_report_readable_while_provider_is_blocked -- --exact --nocapture`

Exit status: **101**

```text
   Compiling deslop-core v0.0.0-dev (/home/christian/code/deslop/target/resource-fixes-integration/crates/deslop-core)
   Compiling deslop-test-support v0.0.0-dev (/home/christian/code/deslop/target/resource-fixes-integration/crates/deslop-test-support)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 9.47s
     Running tests/suite.rs (target/debug/deps/suite-53fe695498f100df)

running 1 test

thread 'live::embedding_refresh_keeps_latest_report_readable_while_provider_is_blocked' (116496) panicked at crates/deslop-core/tests/live.rs:1025:5:
assertion `left == right` failed: a completed background job must not restore the older editor text
  left: [ReportCluster { id: "54a6b248b64f425d", rank: 1, severity: "warning", kind: NearlyIdentical, mass: 58, canonical_node_count: 58, occurrences: [ReportOccurrence { path: "Alpha.cs", start_byte: 0, end_byte: 383, start_line: 1, end_line: 19, hidden: false, in_diff: None }, ReportOccurrence { path: "Beta.cs", start_byte: 0, end_byte: 414, start_line: 1, end_line: 19, hidden: false, in_diff: None }], occurrences_total: 2, occurrence_count: 2, occurrences_truncated: false, intersects_diff: None, is_newly_introduced: None }]
 right: []
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test live::embedding_refresh_keeps_latest_report_readable_while_provider_is_blocked ... FAILED

failures:

failures:
    live::embedding_refresh_keeps_latest_report_readable_while_provider_is_blocked

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 209 filtered out; finished in 0.88s

error: test failed, to rerun pass `-p deslop-core --test suite`

```
