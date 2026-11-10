# Retired metric configuration refusal

[METRICS-REPO-WEIGHTED] / [EXIT-CODES-WEIGHTED]. Root configuration and threshold tables reject unknown fields before analysis.

## weighted-config-red

SHA-256: `42b063c5a7fd6372b4ac0adfae2c7a3ce72c0aa0fd5f21a9759d975274e15ac2`.

```text
Compiling deslop v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop)
    Finished `release` profile [optimized] target(s) in 57.35s
     Running tests/suite.rs (target/release/deps/suite-b00d1cb49e41768e)

running 1 test
test metrics_weighted_wire_refusal::retired_metric_configuration_is_rejected_before_analysis ... FAILED

failures:

---- metrics_weighted_wire_refusal::retired_metric_configuration_is_rejected_before_analysis stdout ----

thread 'metrics_weighted_wire_refusal::retired_metric_configuration_is_rejected_before_analysis' (3884266) panicked at crates/deslop/tests/metrics_weighted_wire_refusal.rs:259:14:
Unexpected success
command=`"/Users/christianfindlay/Documents/Code/Deslop/target/release/deslop" "/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpEhebQs/repo" "--output" "/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpEhebQs/report" "--no-incremental" "--config" "/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpEhebQs/repo/.deslop.toml"`
code=0
stdout=""
stderr=```
deslop scanning /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpEhebQs/repo for duplicated code...
  report → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpEhebQs/report.{json,txt,html}
  log    → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpEhebQs/logs/deslop-1794270963.log

Found 0 groups of duplicated code across 2 file(s)
  ✔ no duplication detected — your codebase is clean.

✔ done
    report /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpEhebQs/report.json
    report /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpEhebQs/report.txt
    report /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpEhebQs/report.html
    log    /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpEhebQs/logs/deslop-1794270963.log
```


note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    metrics_weighted_wire_refusal::retired_metric_configuration_is_rejected_before_analysis

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 629 filtered out; finished in 0.78s

error: test failed, to rerun pass `-p deslop --test suite`
```

## weighted-config-green

SHA-256: `5ecfb4323bbb529bd138befaf7e7aa15966cd9a38af9e12bb2f153892942ec5a`.

```text
Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
   Compiling deslop v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop)
    Finished `release` profile [optimized] target(s) in 2m 15s
     Running tests/suite.rs (target/release/deps/suite-b00d1cb49e41768e)

running 3 tests
test metrics_weighted_wire_refusal::a_weighted_threshold_flag_is_a_usage_error ... ok
test metrics_weighted_wire_refusal::retired_metric_configuration_is_rejected_before_analysis ... ok
test metrics_weighted_wire_refusal::no_surface_carries_an_evidence_weighted_duplication_figure ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 627 filtered out; finished in 1.18s
```

## weighted-config-fmt

SHA-256: `411f30f903c3b28ef1b25b1af8839e47b3372cdccd7fdc3e575e863893e8806f`.

```text
==> _typediagram-gen: regenerating IPC models from docs/models/live-ipc.td
node scripts/typediagram/generate.mjs
typediagram-gen: unchanged /Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core/src/wire_generated.rs
typediagram-gen: unchanged /Users/christianfindlay/Documents/Code/Deslop/clients/vscode/src/types/wire-generated.ts
==> Formatting (check mode)...
```

## Workspace analysis

SHA-256: `062204908296ad06f1a2fb6de0870a69eefacc600b077d3e4513ffa9c5a75bf6`.

```text
==> _typediagram-gen: regenerating IPC models from docs/models/live-ipc.td
node scripts/typediagram/generate.mjs
typediagram-gen: unchanged /Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core/src/wire_generated.rs
typediagram-gen: unchanged /Users/christianfindlay/Documents/Code/Deslop/clients/vscode/src/types/wire-generated.ts
==> Linting...
cargo clippy --release --all-targets --workspace --features deslop-core/live,deslop-lsp/profiling -- -D warnings
    Blocking waiting for file lock on build directory
    Checking deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
    Checking deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
    Checking deslop-mcp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-mcp)
    Checking deslop-lsp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-lsp)
    Checking deslop v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop)
    Finished `release` profile [optimized] target(s) in 2m 17s
Taxonomy gate OK — every product-facing Type-N reference has a bucket label.
env/PATH write lint: 8 files clean
```

Full workspace tests are running separately; this receipt does not claim their result.
