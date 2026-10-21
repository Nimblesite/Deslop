# Watcher delivery accuracy quarantine

Generated from captured test output and the Rust syntax tree at 2026-10-03T13:23:01.272270+00:00.

## Failing behavior

- Test: `source_burst_preserves_every_changed_path` ([source](../crates/deslop-core/src/live/watcher/tests/delivery.rs)).
- Command: `cargo test --release -p deslop-core --features live --lib source_burst_preserves_every_changed_path -- --nocapture`.
- Original failure log: [idle-cpu-watcher-delivery-red.log](../target/idle-cpu-watcher-delivery-red.log).
- Original log SHA-256: `81d4c60d87e62e961da089cbd66d2310ab99f8d738788c7055bcd31c012706aa`.

```text
Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
    Finished `release` profile [optimized] target(s) in 1m 06s
     Running unittests src/lib.rs (target/release/deps/deslop_core-c58a8a9300276a83)

running 1 test

thread 'live::watcher::tests::delivery::source_burst_preserves_every_changed_path' (965159) panicked at crates/deslop-core/src/live/watcher/tests/delivery.rs:44:5:
assertion `left == right` failed: a full watcher queue must not discard source edits
  left: 256
 right: 257
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test live::watcher::tests::delivery::source_burst_preserves_every_changed_path ... FAILED

failures:

failures:
    live::watcher::tests::delivery::source_burst_preserves_every_changed_path

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 323 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p deslop-core --lib`
```

## Mandatory quarantine

- Production source: [watcher.rs](../crates/deslop-core/src/live/watcher.rs#L220).
- Delivery calls now entering quarantine: `4`.
- Remaining unchecked `try_send` calls in this module: `0`.
- Eligible watcher events deliberately panic. This is an accuracy quarantine, not a completed CPU fix.
- The failing assertion remains unchanged. No post-quarantine test run is claimed.

```rust
fn quarantined_delivery(_sender: &Sender<PathBuf>, _path: PathBuf) {
    panic!("[LIVE-WATCHER] accuracy quarantine: source_burst_preserves_every_changed_path pins lost source edits");
}
```

## Interrupted CPU work

- Original CPU reproduction: [excluded-file churn](idle-cpu-excluded-churn.md).
- Full-suite and interrupted-coverage results: [validation](idle-cpu-validation.md).
- Post-fix CPU comparison and Windows-native verification remain incomplete.
- Original formatting was recoverable for the files shown in [the recovered formatter diagnostic](../target/idle-cpu-original-format-diagnostics.diff). Unrelated formatter edits remain in `crates/deslop-test-support/src/bin/corpus-score.rs` and `crates/deslop-test-support/src/corpus_score/tests/render.rs`; no exact original diagnostic was available.
