# Idle CPU

## [LIVE-SCHEDULER-IDLE] Sleep until work arrives

The scheduler waits for watcher events whenever its queue is empty. Pending changes still use the existing debounce rules; elapsed idle ticks must never replay as a burst. The contract is in [live.md](../specs/live.md), the implementation is in `crates/deslop-core/src/live/scheduler.rs`, and its deterministic virtual-time test is in `crates/deslop-core/src/live/scheduler/tests.rs`.

Validate the scheduler test, existing live edit/deletion tests, the full Rust suite, and coverage. Measure native process CPU separately: `scripts/reports/idle_cpu.py` emits [the measurement report](../../reports/idle-cpu-native-sampling.md). A quiet sample does not resolve a workload that has not been reproduced.

## [LIVE-WATCHER-DELIVERY] Retain bursts through scheduler shutdown

Replace the watcher bridge's capacity-limited, unchecked sends with one queue that retains admitted paths while the scheduler is busy. Keep arrival order, callback-local deduplication, and later edits to the same path. Remove the old delivery path; add no retry worker or polling loop. Once the watcher closes, drain accepted paths and dispatch any final pending changes immediately before stopping. A seed-only session retains dispatched changes until its cold pipeline is installed.

The contract is in [live.md](../specs/live.md). `crates/deslop-core/src/live/watcher/tests/delivery.rs` checks bursts beyond the former capacity, repeated edits, admitted event classes, and channel closure. `crates/deslop-core/src/live/scheduler/tests.rs` proves the final analysis pass completes with virtual time. Preserve every assertion in the failing burst test. Run `crates/deslop-lsp/tests/live/bursts.rs` against the real LSP to verify source bursts, clone removal and restoration, and directory deletion against full report assertions before the final native CPU comparison.

## [LIVE-SCHEDULER-REMOVAL-COST] Look up deleted paths in an index

Keep live-file membership indexed by path as well as file identifier in `crates/deslop-core/src/state.rs`. Update both views through one wrapper so subtree removal in `pipeline/session/change.rs` visits matching paths instead of scanning the workspace. Include empty and skipped files in live membership; fingerprint storage is not a complete file index.

The CLI assertions in `crates/deslop/tests/rerun/removals.rs` pin bounded candidate visits and preserve the rendered cluster, occurrence paths, ranking, counts, and empty delta for an unrelated deletion. [live.md](../specs/live.md) defines the contract. Repeat the native churn workload after rebuilding to measure the complete watcher-to-report loop.

Resolve a known-missing path through its surviving parent instead of first canonicalising the absent leaf. `pipeline/session/change/tests.rs` pins the lookup count and preserves existing-file, native-alias, and dangling-symlink behavior.

## [LIVE-WATCHER-CONFIG-COST] Resolve plausible configuration aliases

Reject unrelated build-event paths before resolving all filesystem ancestors. Preserve explicit config paths, same-name aliases through parent symlinks, differently named leaf symlinks, and native Windows filename semantics. Move the matching function and resolver-call tests into `crates/deslop-core/src/config/paths.rs` and `config/paths/tests.rs`; both the watcher and pipeline call that one implementation. Verify the helper without the `live` feature. [live.md](../specs/live.md) defines the contract.

The full-loop reproduction is `python3 scripts/reports/idle_churn.py`. It owns an LSP process, waits for analysis completion, creates and removes ignored build files, measures CPU, and compares the complete reports before and after. Its output is [the excluded-file churn report](../../reports/idle-cpu-excluded-churn.md).

## [LIVE-WATCHER-REMOVAL] Reject typed non-source file removals

Use `RemoveKind::File` to reject unsupported extensions before scheduling analysis. Keep configuration and ignore-rule bypasses, admit formerly included source files despite their current exclusion, and preserve every directory or uncertain removal. The real callback assertions in `crates/deslop-core/src/live/watcher/tests/delivery/removals.rs` enforce the contract in [live.md](../specs/live.md).

## [LIVE-PARENT-LIVENESS] Probe Windows parents through native handles

Replace the shared `tasklist` subprocess in `crates/deslop-core/src/process.rs` with a native process handle and a zero-timeout wait. Preserve live/dead decisions, conservatively retain a parent on permission or unknown errors, and close the handle after each probe. Unix behavior stays unchanged.

The Windows test `process::tests::parent_probe_uses_no_child_processes` exercises the production implementation. `scripts/lib/windows_job_probe.py` counts all probe job processes, including exited children, while the test independently checks live and reaped PIDs. The original Windows implementation failed this check under Wine before the fix; retain that evidence and run the same assertions afterward. Wine validates the Windows executable and API behavior, while native Windows CI remains the platform validation. Register the Windows-only test in `crates/deslop/tests/skip_policy_contract.rs`, and keep CLI shutdown and LSP lifecycle checks in the full validation run.

## [LIVE-CACHE-SEED-READINESS] Wait for real startup completion

Retain dispatched changes while a cached startup report is visible and its real pipeline is still being built. Publish cold-pass `Idle` only after installation and replay of the session's deferred changes. This describes completion of that pass; changes still waiting for debounce belong to the next batch. `crates/deslop-core/src/live/scheduler.rs` and `crates/deslop-lsp/src/cache_seed.rs` implement the ordering; virtual-time tests in `live/scheduler/tests.rs` verify it. The native measurement must also keep the workspace quiet when sampling quiet CPU.

Retain the generated startup-state value rather than only an active flag. After a failed cold refresh, a late `initialized` notification must preserve `Errored` and its exact message. Keep state updates and initial-state publication ordered; `crates/deslop-lsp/src/cache_seed/tests/readiness.rs` checks the failure sequence before and after the fix.
