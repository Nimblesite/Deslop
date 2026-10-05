# Watcher delivery rewrite

Measured: 2026-10-03T20:59:39.904595+00:00

Command: `cargo test --release -p deslop-core --features live --lib source_burst_preserves_every_changed_path -- --nocapture`

Exit code: `101`

Log: [idle-cpu-quarantine-confirm.log](../target/idle-cpu-quarantine-confirm.log)

```text
   Compiling shlex v2.0.1
   Compiling find-msvc-tools v0.1.13
   Compiling tree-sitter-language v0.1.8
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.45
   Compiling unicode-ident v1.0.24
   Compiling libc v0.2.189
   Compiling memchr v2.8.0
   Compiling zmij v1.0.21
   Compiling cfg-if v1.0.4
   Compiling serde_core v1.0.229
   Compiling crossbeam-utils v0.8.21
   Compiling regex-syntax v0.8.10
   Compiling serde_json v1.0.151
   Compiling zerocopy v0.8.48
   Compiling hashbrown v0.17.0
   Compiling cc v1.4.7
   Compiling aho-corasick v1.1.4
   Compiling equivalent v1.0.2
   Compiling indexmap v2.14.0
   Compiling log v0.4.29
   Compiling autocfg v1.5.0
   Compiling regex-automata v0.4.18
   Compiling itoa v1.0.18
   Compiling bitflags v2.11.1
   Compiling pin-project-lite v0.2.17
   Compiling num-traits v0.2.19
   Compiling httparse v1.10.1
   Compiling futures-core v0.3.34
   Compiling rayon-core v1.13.0
   Compiling anstyle v1.0.14
   Compiling parking_lot_core v0.9.12
   Compiling bytes v1.11.1
   Compiling futures-sink v0.3.34
   Compiling futures-channel v0.3.34
   Compiling same-file v1.0.6
   Compiling scopeguard v1.2.0
   Compiling serde v1.0.229
   Compiling slab v0.4.12
   Compiling futures-task v0.3.34
   Compiling once_cell v1.21.4
   Compiling utf8parse v0.2.2
   Compiling smallvec v1.15.1
   Compiling getrandom v0.4.3
   Compiling bstr v1.12.1
   Compiling cfg_aliases v0.2.1
   Compiling futures-io v0.3.34
   Compiling nix v0.31.3
   Compiling anstyle-parse v1.0.0
   Compiling getrandom v0.2.17
   Compiling lock_api v0.4.14
   Compiling rand_core v0.6.4
   Compiling crossbeam-epoch v0.9.18
   Compiling walkdir v2.5.0
   Compiling crossbeam-deque v0.8.6
   Compiling http v1.4.0
   Compiling tree-sitter-fsharp v0.3.12
   Compiling syn v3.0.3
   Compiling syn v2.0.117
   Compiling blake3 v1.8.7
   Compiling tree-sitter-dart v0.2.0
   Compiling tree-sitter-c-sharp v0.23.5
   Compiling tree-sitter-typescript v0.23.2
   Compiling tree-sitter-rust v0.24.2
   Compiling tree-sitter-python v0.25.0
   Compiling tree-sitter-php v0.24.2
   Compiling tree-sitter-javascript v0.25.0
   Compiling tree-sitter-go v0.25.0
   Compiling colorchoice v1.0.5
   Compiling base64 v0.23.1
   Compiling thiserror v2.0.20
   Compiling either v1.15.0
   Compiling is_terminal_polyfill v1.70.2
   Compiling anstyle-query v1.1.5
   Compiling winnow v1.0.4
   Compiling predicates-core v1.0.10
   Compiling parking_lot v0.12.5
   Compiling tree-sitter v0.26.12
   Compiling anstream v1.0.0
   Compiling toml_parser v1.1.3+spec-1.1.0
   Compiling futures-macro v0.3.34
   Compiling serde_derive v1.0.229
   Compiling futures-util v0.3.34
   Compiling rayon v1.12.0
   Compiling tracing-attributes v0.1.31
   Compiling ppv-lite86 v0.2.21
   Compiling tokio-macros v2.7.1
   Compiling rand_chacha v0.3.1
   Compiling rand v0.8.6
   Compiling ureq-proto v0.6.4
   Compiling ordered-float v3.9.2
   Compiling thiserror-impl v2.0.20
   Compiling fsevent-sys v4.1.0
   Compiling num_cpus v1.17.0
   Compiling toml_datetime v1.1.1+spec-1.1.0
   Compiling serde_spanned v1.1.1
   Compiling globset v0.4.18
   Compiling regex v1.12.3
   Compiling tracing-core v0.1.36
   Compiling notify-types v2.1.0
   Compiling anyhow v1.0.104
   Compiling strsim v0.11.1
   Compiling utf8-zero v0.8.1
   Compiling difflib v0.4.0
   Compiling assert_cmd v2.2.2
   Compiling arrayvec v0.7.6
   Compiling constant_time_eq v0.4.2
   Compiling rustix v1.1.5
   Compiling futures-executor v0.3.34
   Compiling streaming-iterator v0.1.9
   Compiling heck v0.5.0
   Compiling clap_lex v1.1.0
   Compiling toml_writer v1.1.2+spec-1.1.0
   Compiling termtree v0.5.1
   Compiling percent-encoding v2.3.2
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
   Compiling predicates-tree v1.0.13
   Compiling toml v1.1.6+spec-1.1.0
   Compiling clap_builder v4.6.7
   Compiling clap_derive v4.6.7
   Compiling futures v0.3.34
   Compiling predicates v3.1.4
   Compiling notify v8.2.0
   Compiling tracing v0.1.44
   Compiling ignore v0.4.33
   Compiling instant-distance v0.6.1
   Compiling ureq v3.4.2
   Compiling tokio v1.53.1
   Compiling async-trait v0.1.92
   Compiling errno v0.3.14
   Compiling wait-timeout v0.2.1
   Compiling clap v4.6.7
   Compiling fastrand v2.4.1
   Compiling tempfile v3.27.0
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
    Finished `release` profile [optimized] target(s) in 1m 28s
     Running unittests src/lib.rs (target/release/deps/deslop_core-240047566e698ddd)

running 1 test

thread 'live::watcher::tests::delivery::source_burst_preserves_every_changed_path' (2731233) panicked at crates/deslop-core/src/live/watcher.rs:221:5:
[LIVE-WATCHER] accuracy quarantine: source_burst_preserves_every_changed_path pins lost source edits
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test live::watcher::tests::delivery::source_burst_preserves_every_changed_path ... FAILED

failures:

failures:
    live::watcher::tests::delivery::source_burst_preserves_every_changed_path

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 327 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p deslop-core --lib`

```

## Real LSP source burst

- Measured: 2026-10-03T21:11:15.219743+00:00
- Command: `cargo test --release -p deslop-lsp --features deslop-core/live,deslop-lsp/profiling --test suite live_bursts -- --nocapture`
- Exit code: `0`
- Output: [idle-cpu-live-burst.log](../target/idle-cpu-live-burst.log)

## Missing-path filesystem work

Recorded: 2026-10-03T21:28:18.740980+00:00

### Original failing resolver budget

[idle-cpu-missing-path-red.log](../target/idle-cpu-missing-path-red.log)

```text
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
    Finished `release` profile [optimized] target(s) in 1m 08s
     Running unittests src/lib.rs (target/release/deps/deslop_core-240047566e698ddd)

running 1 test

thread 'pipeline::session::change::tests::missing_source_resolves_its_surviving_parent_once' (2900174) panicked at crates/deslop-core/src/pipeline/session/change/tests.rs:30:5:
assertion `left == right` failed: a missing leaf must not repeat its parent's realpath walk
  left: 2
 right: 1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test pipeline::session::change::tests::missing_source_resolves_its_surviving_parent_once ... FAILED

failures:

failures:
    pipeline::session::change::tests::missing_source_resolves_its_surviving_parent_once

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 334 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p deslop-core --lib`

```

### Three resolver correctness controls after replacement

[idle-cpu-missing-path-green.log](../target/idle-cpu-missing-path-green.log)

```text
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
    Finished `release` profile [optimized] target(s) in 1m 10s
     Running unittests src/lib.rs (target/release/deps/deslop_core-240047566e698ddd)

running 3 tests
test pipeline::session::change::tests::absent_parent_preserves_the_original_removal_reference ... ok
test pipeline::session::change::tests::missing_source_resolves_its_surviving_parent_once ... ok
test pipeline::session::change::tests::existing_source_resolves_the_complete_path_once ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 334 filtered out; finished in 0.00s


```
