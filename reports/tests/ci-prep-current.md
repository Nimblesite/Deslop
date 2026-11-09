# Current CI preparation evidence

Source commit: `c79016b4fe1fdd0c2245d9a2a81294f6467e5cca`.

## Static analysis

Command: `CARGO_BUILD_JOBS=1 nice -n 15 make _ci-analyze`. Exit status: 0.

```text
==> _typediagram-gen: regenerating IPC models from docs/models/live-ipc.td
node scripts/typediagram/generate.mjs
typediagram-gen: unchanged /Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core/src/wire_generated.rs
typediagram-gen: unchanged /Users/christianfindlay/Documents/Code/Deslop/clients/vscode/src/types/wire-generated.ts
==> Linting...
cargo clippy --release --all-targets --workspace --features deslop-core/live,deslop-lsp/profiling -- -D warnings
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
    Checking tree-sitter-osprey-ml v0.1.0 (https://github.com/Nimblesite/osprey?rev=093f04242c7d8b2f56d8d5051dd813b9e9d280f3#093f0424)
    Checking tree-sitter-osprey v0.0.0-dev (https://github.com/Nimblesite/osprey?rev=093f04242c7d8b2f56d8d5051dd813b9e9d280f3#093f0424)
    Checking deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
    Checking deslop-mcp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-mcp)
    Checking deslop-lsp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-lsp)
    Checking deslop v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop)
    Finished `release` profile [optimized] target(s) in 28.13s
Taxonomy gate OK — every product-facing Type-N reference has a bucket label.
env/PATH write lint: 8 files clean
```

Remaining gates are tracked in [the CI plan](../../docs/plans/ci-prep-2026-11-10.md).

## Release workspace build

Command: `CARGO_BUILD_JOBS=1 nice -n 15 make build`. Exit status: 0.

```text
==> Building...
cargo build --release --workspace
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
   Compiling deslop-lsp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-lsp)
   Compiling deslop-mcp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-mcp)
   Compiling deslop v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop)
    Finished `release` profile [optimized] target(s) in 4m 59s
```

## Repository contract tests

Command: `NODE_V8_COVERAGE=coverage/repository/ci-prep-2026-11-10 nice -n 15 make _ci-contract-tests`. Exit status: 0.

Passed tests across TAP groups: 215. Fresh raw coverage: `coverage/repository/ci-prep-2026-11-10`; thresholds are not yet calculated.

```text
# tests 52
# pass 52
# fail 0
# skipped 0
# tests 12
# pass 12
# fail 0
# skipped 0
# tests 15
# pass 15
# fail 0
# skipped 0
# tests 3
# pass 3
# fail 0
# skipped 0
# tests 4
# pass 4
# fail 0
# skipped 0
# tests 5
# pass 5
# fail 0
# skipped 0
# tests 11
# pass 11
# fail 0
# skipped 0
# tests 8
# pass 8
# fail 0
# skipped 0
# tests 8
# pass 8
# fail 0
# skipped 0
# tests 5
# pass 5
# fail 0
# skipped 0
# tests 9
# pass 9
# fail 0
# skipped 0
# tests 5
# pass 5
# fail 0
# skipped 0
# tests 4
# pass 4
# fail 0
# skipped 0
# tests 15
# pass 15
# fail 0
# skipped 0
# tests 4
# pass 4
# fail 0
# skipped 0
# tests 3
# pass 3
# fail 0
# skipped 0
# tests 11
# pass 11
# fail 0
# skipped 0
# tests 20
# pass 20
# fail 0
# skipped 0
# tests 5
# pass 5
# fail 0
# skipped 0
# tests 9
# pass 9
# fail 0
# skipped 0
# tests 3
# pass 3
# fail 0
# skipped 0
# tests 4
# pass 4
# fail 0
# skipped 0
```
