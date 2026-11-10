# CLI cluster identity verification

[UX-CLUSTER-IDENTITY] — issue #349.

The CLI prints the seven-character canonical ID prefix first and labels rank separately. The mutation test retains the unchanged family ID as another family gains a copy and moves ahead. The MCP test copies a handle from actual CLI output and resolves it through the real server.

## cli-summary-red

Log SHA-256: `6205b6c6ba5b66932e2988c78a77936bd7ff0b84218c0f5708cdb2bc60fc17cb`.

```text
Compiling deslop v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop)
    Finished `release` profile [optimized] target(s) in 1m 24s
     Running tests/suite.rs (target/release/deps/suite-b00d1cb49e41768e)

running 1 test
test cli::summary::summaries_keep_cluster_handles_when_another_group_changes_the_rank ... FAILED

failures:

---- cli::summary::summaries_keep_cluster_handles_when_another_group_changes_the_rank stdout ----
Error: summary must lead with 02bb5e8: deslop scanning /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpp2yLrK/src for duplicated code...
  report → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpp2yLrK/report.{json,txt,html}
  log    → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpp2yLrK/logs/deslop-1794270314.log

Found 2 groups of duplicated code across 7 file(s)
  diagnostic severity: 2 × warning
  Highest mass: 3792 from 5 occurrences × 948 canonical nodes in mid_0.ts, mid_1.ts, mid_2.ts (+2 more)

Top 2 findings
  #1  Nearly identical code — mass 3792, 5 occurrences in mid_0.ts, mid_1.ts, mid_2.ts (+2 more)
  #2  Identical code — mass 3513, 2 occurrences in big_a.ts, big_b.ts

Next: open the .html report to inspect occurrences or explicitly compare two endpoints.

✔ done
    report /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpp2yLrK/report.json
    report /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpp2yLrK/report.txt
    report /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpp2yLrK/report.html
    log    /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpp2yLrK/logs/deslop-1794270314.log



failures:
    cli::summary::summaries_keep_cluster_handles_when_another_group_changes_the_rank

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 628 filtered out; finished in 2.08s

error: test failed, to rerun pass `-p deslop --test suite`
```

## cli-summary-green

Log SHA-256: `d1e8d1d38467445639fad7ffe5e416c3343a9e5605160e6886e0716c9ecff34c`.

```text
Compiling deslop v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop)
    Finished `release` profile [optimized] target(s) in 1m 01s
     Running tests/suite.rs (target/release/deps/suite-b00d1cb49e41768e)

running 1 test
test cli::summary::summaries_keep_cluster_handles_when_another_group_changes_the_rank ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 628 filtered out; finished in 9.54s
```

## cli-mcp-slug

Log SHA-256: `9c7881e49f6ab1a3d856cf956fef487bc5d6b703388c441318227ee34e89595b`.

```text
Blocking waiting for file lock on build directory
   Compiling deslop-mcp v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-mcp)
    Finished `release` profile [optimized] target(s) in 1m 02s
     Running tests/suite.rs (target/release/deps/suite-a62a08c4bb2cd5ab)

running 2 tests
test issue_149_ui_mcp_agreement::ui_and_mcp_top_offenders_agree_on_worst_first_ids ... ok
test issue_149_ui_mcp_agreement::cluster_by_id_accepts_seven_hex_slug ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 147 filtered out; finished in 0.90s
```

## cli-summary-analyze

Log SHA-256: `086623642b98158eaa55f927f0a67201b57c188f19ec489caf0faf7fe3c1102c`.

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
    Finished `release` profile [optimized] target(s) in 54.33s
Taxonomy gate OK — every product-facing Type-N reference has a bucket label.
env/PATH write lint: 8 files clean
```
