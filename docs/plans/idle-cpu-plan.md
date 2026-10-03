# Idle CPU

## [LIVE-SCHEDULER-IDLE] Sleep until work arrives

The scheduler waits for watcher events whenever its queue is empty. Pending changes still use the existing debounce rules; elapsed idle ticks must never replay as a burst. The contract is in [live.md](../specs/live.md), the implementation is in `crates/deslop-core/src/live/scheduler.rs`, and its deterministic virtual-time test is in `crates/deslop-core/src/live/scheduler/tests.rs`.

Validate the scheduler test, existing live edit/deletion tests, the full Rust suite, and coverage. Measure native process CPU separately: `scripts/reports/idle_cpu.py` emits [the measurement report](../../reports/idle-cpu-native-sampling.md). A quiet sample does not resolve a workload that has not been reproduced.

## [LIVE-SCHEDULER-REMOVAL-COST] Look up deleted paths in an index

Keep live-file membership indexed by path as well as file identifier in `crates/deslop-core/src/state.rs`. Update both views through one wrapper so subtree removal in `pipeline/session/change.rs` visits matching paths instead of scanning the workspace. Include empty and skipped files in live membership; fingerprint storage is not a complete file index.

The CLI assertions in `crates/deslop/tests/rerun/removals.rs` pin bounded candidate visits and preserve the rendered cluster, occurrence paths, ranking, counts, and empty delta for an unrelated deletion. [live.md](../specs/live.md) defines the contract. Repeat the native churn workload after rebuilding to measure the complete watcher-to-report loop.

## [LIVE-WATCHER-CONFIG-COST] Resolve plausible configuration aliases

Reject unrelated build-event paths before resolving all filesystem ancestors. Preserve explicit config paths, same-name aliases through parent symlinks, differently named leaf symlinks, and native Windows filename semantics. The matching function and resolver-call tests live in `crates/deslop-core/src/live/watcher.rs` and its `watcher/tests.rs` module; [live.md](../specs/live.md) defines the contract.

The full-loop reproduction is `python3 scripts/reports/idle_churn.py`. It owns an LSP process, waits for analysis completion, creates and removes ignored build files, measures CPU, and compares the complete reports before and after. Its output is [the excluded-file churn report](../../reports/idle-cpu-excluded-churn.md).

Windows parent liveness currently invokes an external process on each poll. Replacing that backend still requires a failing behavioral test executed on Windows; a source-shape assertion is insufficient. Keep the CLI shutdown and LSP protocol lifecycle checks in the validation run.
