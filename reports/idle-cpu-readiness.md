# Cache startup readiness test evidence

Generated from captured command output at 2026-10-03T21:47:49.811044+00:00.

## seed-scheduler-readiness-red.log

[target/seed-scheduler-readiness-red.log](../target/seed-scheduler-readiness-red.log)

```text
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7.15s
     Running unittests src/lib.rs (target/debug/deps/deslop_core-e210a06444ca108c)

running 1 test

thread 'live::scheduler::tests::seed_only_scheduler_stays_running_after_queuing_changes' (2912283) panicked at crates/deslop-core/src/live/scheduler/tests.rs:214:5:
queued seed-only work must not publish Idle before the pipeline exists
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test live::scheduler::tests::seed_only_scheduler_stays_running_after_queuing_changes ... FAILED

failures:

failures:
    live::scheduler::tests::seed_only_scheduler_stays_running_after_queuing_changes

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p deslop-core --lib`
```

## seed-commit-readiness-red.log

[target/seed-commit-readiness-red.log](../target/seed-commit-readiness-red.log)

```text
   Compiling stable_deref_trait v1.2.1
   Compiling smallvec v1.15.1
   Compiling writeable v0.6.3
   Compiling regex-automata v0.4.18
   Compiling futures-sink v0.3.34
   Compiling futures-core v0.3.34
   Compiling litemap v0.8.2
   Compiling log v0.4.29
   Compiling utf8_iter v1.0.4
   Compiling zerofrom v0.1.7
   Compiling icu_normalizer_data v2.2.0
   Compiling icu_properties_data v2.2.0
   Compiling tracing-core v0.1.36
   Compiling httparse v1.10.1
   Compiling tokio v1.53.1
   Compiling form_urlencoded v1.2.2
   Compiling yoke v0.8.2
   Compiling tower-layer v0.3.3
   Compiling futures-channel v0.3.34
   Compiling tower-service v0.3.3
   Compiling parking_lot_core v0.9.12
   Compiling notify v8.2.0
   Compiling pin-project v1.1.11
   Compiling lazy_static v1.5.0
   Compiling bitflags v1.3.2
   Compiling ureq-proto v0.6.4
   Compiling zerovec v0.11.6
   Compiling zerotrie v0.2.4
   Compiling hashbrown v0.14.5
   Compiling rustix v1.1.5
   Compiling thread_local v1.1.9
   Compiling futures-util v0.3.34
   Compiling sharded-slab v0.1.7
   Compiling tracing v0.1.44
   Compiling parking_lot v0.12.5
   Compiling tracing-log v0.2.0
   Compiling nu-ansi-term v0.50.3
   Compiling tower v0.5.3
   Compiling ureq v3.4.2
   Compiling dashmap v5.5.3
   Compiling instant-distance v0.6.1
   Compiling tinystr v0.8.3
   Compiling potential_utf v0.1.5
   Compiling icu_collections v2.2.0
   Compiling icu_locale_core v2.2.0
   Compiling tempfile v3.27.0
   Compiling icu_provider v2.2.0
   Compiling icu_normalizer v2.2.0
   Compiling icu_properties v2.2.0
   Compiling futures-executor v0.3.34
   Compiling tower v0.4.13
   Compiling bstr v1.12.1
   Compiling regex v1.12.3
   Compiling matchers v0.2.0
   Compiling tokio-util v0.7.17
   Compiling tracing-subscriber v0.3.23
   Compiling futures v0.3.34
   Compiling idna_adapter v1.2.1
   Compiling tree-sitter v0.26.12
   Compiling globset v0.4.18
   Compiling assert_cmd v2.2.2
   Compiling idna v1.1.0
   Compiling url v2.5.8
   Compiling ignore v0.4.33
   Compiling lsp-types v0.94.1
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
   Compiling tower-lsp v0.20.0
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
   Compiling deslop-lsp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-lsp)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 12.82s
     Running unittests src/lib.rs (target/debug/deps/deslop_lsp-4860c8902b936658)

running 1 test

thread 'cache_seed::tests::readiness::cold_pass_remains_active_until_pending_changes_are_committed' (2944122) panicked at crates/deslop-lsp/src/cache_seed/tests/readiness.rs:62:5:
a cold pass waiting to commit must remain active
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test cache_seed::tests::readiness::cold_pass_remains_active_until_pending_changes_are_committed ... FAILED

failures:

failures:
    cache_seed::tests::readiness::cold_pass_remains_active_until_pending_changes_are_committed

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 64 filtered out; finished in 0.02s

error: test failed, to rerun pass `-p deslop-lsp --lib`
```

## seed-scheduler-readiness-green.log

[target/seed-scheduler-readiness-green.log](../target/seed-scheduler-readiness-green.log)

```text
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 8.20s
     Running unittests src/lib.rs (target/debug/deps/deslop_core-e210a06444ca108c)

running 3 tests
test live::scheduler::tests::empty_scheduler_sleeps_before_and_after_a_pass ... ok
test live::scheduler::tests::seed_only_scheduler_stays_running_after_queuing_changes ... ok
test live::scheduler::tests::closed_watcher_flushes_queued_changes_before_scheduler_stops ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 334 filtered out; finished in 0.01s
```

## seed-commit-readiness-green.log

[target/seed-commit-readiness-green.log](../target/seed-commit-readiness-green.log)

```text
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
   Compiling deslop-lsp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-lsp)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.50s
     Running unittests src/lib.rs (target/debug/deps/deslop_lsp-4860c8902b936658)

running 8 tests
test cache_seed::tests::live_batch_yield_tracks_embedding_mode ... ok
test cache_seed::tests::refresh_error_pushes_errored_state ... ok
test cache_seed::tests::initial_state_is_idle_once_the_scan_has_settled ... ok
test cache_seed::tests::initial_state_is_running_while_cold_pass_active ... ok
test cache_seed::tests::open_session_reports_cache_seed_status ... ok
test cache_seed::tests::spawn_refresh_pushes_running_then_report_then_idle ... ok
test cache_seed::tests::background_initialise_and_commit_pushes_report_and_idle_state ... ok
test cache_seed::tests::readiness::cold_pass_remains_active_until_pending_changes_are_committed ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 57 filtered out; finished in 0.02s

     Running unittests src/main.rs (target/debug/deps/deslop_lsp-0de3e97d601a807d)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/suite.rs (target/debug/deps/suite-9de6053f95cbe33b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 85 filtered out; finished in 0.00s
```

## seed-readiness-clippy.log

[target/seed-readiness-clippy.log](../target/seed-readiness-clippy.log)

```text
    Checking deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
    Checking deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
    Checking deslop-lsp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-lsp)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.07s
```

## seed-error-state-red.log

[target/seed-error-state-red.log](../target/seed-error-state-red.log)

```text
   Compiling deslop-lsp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-lsp)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.79s
     Running unittests src/lib.rs (target/debug/deps/deslop_lsp-4860c8902b936658)

running 1 test

thread 'cache_seed::tests::readiness::late_initialization_preserves_the_cold_refresh_error' (3023866) panicked at crates/deslop-lsp/src/cache_seed/tests/readiness.rs:18:5:
assertion `left == right` failed
  left: Some("idle")
 right: Some("errored")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test cache_seed::tests::readiness::late_initialization_preserves_the_cold_refresh_error ... FAILED

failures:

failures:
    cache_seed::tests::readiness::late_initialization_preserves_the_cold_refresh_error

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 65 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p deslop-lsp --lib`
```

## seed-error-state-green.log

[target/seed-error-state-green.log](../target/seed-error-state-green.log)

```text
   Compiling deslop-lsp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-lsp)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.92s
     Running unittests src/lib.rs (target/debug/deps/deslop_lsp-4860c8902b936658)

running 9 tests
test cache_seed::tests::live_batch_yield_tracks_embedding_mode ... ok
test cache_seed::tests::initial_state_is_running_while_cold_pass_active ... ok
test cache_seed::tests::refresh_error_pushes_errored_state ... ok
test cache_seed::tests::initial_state_is_idle_once_the_scan_has_settled ... ok
test cache_seed::tests::open_session_reports_cache_seed_status ... ok
test cache_seed::tests::readiness::late_initialization_preserves_the_cold_refresh_error ... ok
test cache_seed::tests::spawn_refresh_pushes_running_then_report_then_idle ... ok
test cache_seed::tests::background_initialise_and_commit_pushes_report_and_idle_state ... ok
test cache_seed::tests::readiness::cold_pass_remains_active_until_pending_changes_are_committed ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 57 filtered out; finished in 0.02s

     Running unittests src/main.rs (target/debug/deps/deslop_lsp-0de3e97d601a807d)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/suite.rs (target/debug/deps/suite-9de6053f95cbe33b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 85 filtered out; finished in 0.00s
```

## seed-error-state-clippy.log

[target/seed-error-state-clippy.log](../target/seed-error-state-clippy.log)

```text
    Checking deslop-lsp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-lsp)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.26s
```
