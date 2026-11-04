# Report reuse baseline blockers

Quarantined Osprey ML implementation equals the feature branch before this bug task (`40b8863607067537b12ad2110ca817785a38de38`): `True`

The existing accuracy quarantine at `crates/deslop-core/src/lang/osprey.rs:115` deliberately panics on an ML tree containing an error. The report-cache change does not modify that implementation.

## First full release gate output

```text
```
command=`"/Users/christianfindlay/Documents/Code/Deslop/target/release/deslop" "/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpudovtw/src" "--output" "/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpudovtw/report" "--min-nodes" "10"`
code=101
stdout=""
stderr=```
deslop scanning /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpudovtw/src for duplicated code...
  report → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpudovtw/report.{json,txt,html}
  log    → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpudovtw/logs/deslop-1793750020.log

thread \'main\' (22104377) panicked at crates/deslop-core/src/lang/osprey.rs:115:21:
accuracy quarantine: Osprey ML whole-file error rejection removed because it hides intact duplicates; implement verified grammar recovery
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```


note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- cli::detection::osprey_flavors::malformed_osprey_ml_is_skipped_visibly_and_the_scan_continues stdout ----

thread 'cli::detection::osprey_flavors::malformed_osprey_ml_is_skipped_visibly_and_the_scan_continues' (22104945) panicked at crates/deslop/tests/cli/detection/osprey_flavors.rs:75:10:
Unexpected failure.
code=101
stderr=``````
deslop scanning /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpHqEyY6/src for duplicated code...
  report → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpHqEyY6/report.{json,txt,html}
  log    → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpHqEyY6/logs/deslop-1793750020.log

thread \'main\' (22104953) panicked at crates/deslop-core/src/lang/osprey.rs:115:21:
accuracy quarantine: Osprey ML whole-file error rejection removed because it hides intact duplicates; implement verified grammar recovery
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```
```
command=`"/Users/christianfindlay/Documents/Code/Deslop/target/release/deslop" "/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpHqEyY6/src" "--output" "/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpHqEyY6/report" "--min-nodes" "10"`
code=101
stdout=""
stderr=```
deslop scanning /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpHqEyY6/src for duplicated code...
  report → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpHqEyY6/report.{json,txt,html}
  log    → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpHqEyY6/logs/deslop-1793750020.log

thread \'main\' (22104953) panicked at crates/deslop-core/src/lang/osprey.rs:115:21:
accuracy quarantine: Osprey ML whole-file error rejection removed because it hides intact duplicates; implement verified grammar recovery
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```



---- cli::detection::osprey_recovery::intact_osprey_duplicates_survive_an_unfinished_neighboring_declaration stdout ----

thread 'cli::detection::osprey_recovery::intact_osprey_duplicates_survive_an_unfinished_neighboring_declaration' (22105044) panicked at crates/deslop/tests/cli/support.rs:202:46:
Unexpected failure.
code=101
stderr=``````
deslop scanning /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmp6HHxxL/src for duplicated code...
  report → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmp6HHxxL/report.{json,txt,html}
  log    → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmp6HHxxL/logs/deslop-1793750021.log

thread \'main\' (22105522) panicked at crates/deslop-core/src/lang/osprey.rs:115:21:
accuracy quarantine: Osprey ML whole-file error rejection removed because it hides intact duplicates; implement verified grammar recovery
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```
```
command=`"/Users/christianfindlay/Documents/Code/Deslop/target/release/deslop" "/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmp6HHxxL/src" "--output" "/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmp6HHxxL/report" "--min-nodes" "10"`
code=101
stdout=""
stderr=```
deslop scanning /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmp6HHxxL/src for duplicated code...
  report → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmp6HHxxL/report.{json,txt,html}
  log    → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmp6HHxxL/logs/deslop-1793750021.log

thread \'main\' (22105522) panicked at crates/deslop-core/src/lang/osprey.rs:115:21:
accuracy quarantine: Osprey ML whole-file error rejection removed because it hides intact duplicates; implement verified grammar recovery
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```



---- cli::embedding_ollama::ollama_incremental_plus_embeddings_second_run_hits_both_caches stdout ----

thread 'cli::embedding_ollama::ollama_incremental_plus_embeddings_second_run_hits_both_caches' (22105274) panicked at crates/deslop/tests/cli/embedding_ollama.rs:71:5:
assertion `left == right` failed: second run must hit the fingerprint cache for both files
  left: Some(0)
 right: Some(2)


failures:
    cli::detection::osprey_flavors::a_repository_of_only_malformed_osprey_ml_still_reports
    cli::detection::osprey_flavors::malformed_osprey_ml_is_skipped_visibly_and_the_scan_continues
    cli::detection::osprey_recovery::intact_osprey_duplicates_survive_an_unfinished_neighboring_declaration
    cli::embedding_ollama::ollama_incremental_plus_embeddings_second_run_hits_both_caches

test result: FAILED. 602 passed; 4 failed; 4 ignored; 0 measured; 0 filtered out; finished in 75.54s

error: test failed, to rerun pass `-p deslop --test suite`
make: *** [test] Error 101
```

## Installed server comparison discovery

Executable: `/Users/christianfindlay/.vscode/extensions/nimblesite.deslop-live-0.40.1-darwin-arm64/bin/darwin-arm64/deslop-lsp`

Command: `target/release/deps/suite-096f4ac716dba0e1 notifications::pair_compare::completed_discovery_is_reused_by_repeated_pair_comparisons --exact --nocapture`

Exit: `101`

```text

running 1 test

thread 'notifications::pair_compare::completed_discovery_is_reused_by_repeated_pair_comparisons' (22412074) panicked at crates/deslop-lsp/tests/notifications/pair_compare.rs:89:5:
assertion `left == right` failed: repeated pair comparisons must reuse completed discovery, including reversed endpoints
  left: 4
 right: 1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test notifications::pair_compare::completed_discovery_is_reused_by_repeated_pair_comparisons ... FAILED

failures:

failures:
    notifications::pair_compare::completed_discovery_is_reused_by_repeated_pair_comparisons

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 97 filtered out; finished in 0.11s

```

## Final release rerun

The embedding/fingerprint cache assertion remains intact and now passes after isolating the lower cache layer. The remaining failures are unchanged quarantined ML cases and the installed-server comparison-discovery case shown above.

```text
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 12 ignored; 0 measured; 0 filtered out; finished in 0.00s
    cli::detection::osprey_flavors::a_repository_of_only_malformed_osprey_ml_still_reports
    cli::detection::osprey_flavors::malformed_osprey_ml_is_skipped_visibly_and_the_scan_continues
    cli::detection::osprey_recovery::intact_osprey_duplicates_survive_an_unfinished_neighboring_declaration
test result: FAILED. 603 passed; 3 failed; 4 ignored; 0 measured; 0 filtered out; finished in 107.95s
error: test failed, to rerun pass `-p deslop --test suite`
make: *** [test] Error 101
```

## Remaining workspace run

```text
test result: ok. 354 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.81s
test result: ok. 222 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.14s
test result: ok. 74 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.02s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
    live_osprey_unparsed::a_repository_of_only_malformed_osprey_ml_still_boots
    live_osprey_unparsed::broken_osprey_ml_file_is_skipped_without_killing_the_live_server
    notifications::pair_compare::completed_discovery_is_reused_by_repeated_pair_comparisons
test result: FAILED. 94 passed; 3 failed; 1 ignored; 0 measured; 0 filtered out; finished in 22.45s
```
