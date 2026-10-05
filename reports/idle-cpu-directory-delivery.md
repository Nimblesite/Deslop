# Watcher directory and overflow delivery evidence

Mechanically generated from captured Cargo output for [LIVE-WATCHER-DIRECTORIES] and [LIVE-WATCHER-RESCAN].

## watcher-directories-red.log

[target/idle-cpu-measurement/watcher-directories-red.log](../target/idle-cpu-measurement/watcher-directories-red.log)

```text
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.41s
     Running unittests src/lib.rs (target/debug/deps/deslop_core-e210a06444ca108c)

running 5 tests

thread 'live::watcher::tests::delivery::directories::rescan_without_paths_preserves_the_workspace_reconciliation_request' (3471981) panicked at crates/deslop-core/src/live/watcher/tests/delivery.rs:66:5:
assertion `left == right` failed: the first source edit must arrive
  left: None
 right: Some("workspace")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'live::watcher::tests::delivery::directories::directory_creation_keeps_sources_and_rejects_excluded_build_output' (3471977) panicked at crates/deslop-core/src/live/watcher/tests/delivery.rs:66:5:
assertion `left == right` failed: the first source edit must arrive
  left: None
 right: Some("/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpxL2utW/created")

thread 'live::watcher::tests::delivery::directories::rename_source_eviction_survives_exclusions_but_new_excluded_targets_do_not' (3471980) panicked at crates/deslop-core/src/live/watcher/tests/delivery.rs:66:5:
assertion `left == right` failed: the first source edit must arrive
  left: None
 right: Some("/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpxNwukM/created")

thread 'live::watcher::tests::delivery::directories::rename_events_keep_missing_old_directories_and_existing_new_directories' (3471979) panicked at crates/deslop-core/src/live/watcher/tests/delivery.rs:66:5:
assertion `left == right` failed: the first source edit must arrive
  left: None
 right: Some("/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpgIyXJC/previous")
test live::watcher::tests::delivery::directories::rescan_without_paths_preserves_the_workspace_reconciliation_request ... FAILED
test live::watcher::tests::delivery::directories::rename_events_keep_missing_old_directories_and_existing_new_directories ... FAILED
test live::watcher::tests::delivery::directories::rename_source_eviction_survives_exclusions_but_new_excluded_targets_do_not ... FAILED
test live::watcher::tests::delivery::directories::directory_creation_keeps_sources_and_rejects_excluded_build_output ... FAILED
test live::watcher::tests::delivery::directories::file_creations_keep_sources_without_discovering_regular_build_files ... ok

failures:

failures:
    live::watcher::tests::delivery::directories::directory_creation_keeps_sources_and_rejects_excluded_build_output
    live::watcher::tests::delivery::directories::rename_events_keep_missing_old_directories_and_existing_new_directories
    live::watcher::tests::delivery::directories::rename_source_eviction_survives_exclusions_but_new_excluded_targets_do_not
    live::watcher::tests::delivery::directories::rescan_without_paths_preserves_the_workspace_reconciliation_request

test result: FAILED. 1 passed; 4 failed; 0 ignored; 0 measured; 339 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p deslop-core --lib`
```

## watcher-directories-green.log

[target/idle-cpu-measurement/watcher-directories-green.log](../target/idle-cpu-measurement/watcher-directories-green.log)

```text
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.58s
     Running unittests src/lib.rs (target/debug/deps/deslop_core-e210a06444ca108c)

running 11 tests
test live::watcher::tests::delivery::directories::rescan_without_paths_preserves_the_workspace_reconciliation_request ... ok
test live::watcher::tests::delivery::removals::removed_directories_and_unknown_kinds_keep_every_path ... ok
test live::watcher::tests::delivery::admitted_event_classes_drain_in_order_after_sender_closes ... ok
test live::watcher::tests::delivery::callback_deduplication_preserves_later_edits_to_the_same_source ... ok
test live::watcher::tests::delivery::removals::removed_files_keep_source_config_and_ignore_changes ... ok
test live::watcher::tests::delivery::directories::rename_source_eviction_survives_exclusions_but_new_excluded_targets_do_not ... ok
test live::watcher::tests::delivery::directories::directory_creation_keeps_sources_and_rejects_excluded_build_output ... ok
test live::watcher::tests::delivery::directories::file_creations_keep_sources_without_discovering_regular_build_files ... ok
test live::watcher::tests::delivery::directories::rename_events_keep_missing_old_directories_and_existing_new_directories ... ok
test live::watcher::tests::delivery::closed_receiver_allows_callbacks_to_finish ... ok
test live::watcher::tests::delivery::source_burst_preserves_every_changed_path ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 333 filtered out; finished in 0.01s
```

## watcher-rename-noise-red.log

[target/idle-cpu-measurement/watcher-rename-noise-red.log](../target/idle-cpu-measurement/watcher-rename-noise-red.log)

```text
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5.19s
     Running unittests src/lib.rs (target/debug/deps/deslop_core-e210a06444ca108c)

running 1 test

thread 'live::watcher::tests::delivery::directories::rename_events_skip_regular_build_files_and_keep_source_or_structural_paths' (3515662) panicked at crates/deslop-core/src/live/watcher/tests/delivery.rs:72:5:
assertion `left == right` failed: a full watcher queue must not discard source edits
  left: 4
 right: 3
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test live::watcher::tests::delivery::directories::rename_events_skip_regular_build_files_and_keep_source_or_structural_paths ... FAILED

failures:

failures:
    live::watcher::tests::delivery::directories::rename_events_skip_regular_build_files_and_keep_source_or_structural_paths

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 344 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p deslop-core --lib`
```

## watcher-rename-noise-green.log

[target/idle-cpu-measurement/watcher-rename-noise-green.log](../target/idle-cpu-measurement/watcher-rename-noise-green.log)

```text
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.06s
     Running unittests src/lib.rs (target/debug/deps/deslop_core-e210a06444ca108c)

running 13 tests
test live::watcher::tests::delivery::directories::rescan_without_paths_preserves_the_workspace_reconciliation_request ... ok
test live::watcher::tests::delivery::removals::removed_directories_and_unknown_kinds_keep_every_path ... ok
test live::watcher::tests::delivery::admitted_event_classes_drain_in_order_after_sender_closes ... ok
test live::watcher::tests::delivery::callback_deduplication_preserves_later_edits_to_the_same_source ... ok
test live::watcher::tests::delivery::removals::removed_files_keep_source_config_and_ignore_changes ... ok
test live::watcher::tests::delivery::directories::rename_events_keep_missing_old_directories_and_existing_new_directories ... ok
test live::watcher::tests::delivery::directories::directory_creation_keeps_sources_and_rejects_excluded_build_output ... ok
test live::watcher::tests::delivery::directories::file_creations_keep_sources_without_discovering_regular_build_files ... ok
test live::watcher::tests::delivery::directories::rename_events_skip_regular_build_files_and_keep_source_or_structural_paths ... ok
test live::watcher::tests::delivery::directories::rename_source_eviction_survives_exclusions_but_new_excluded_targets_do_not ... ok
test live::watcher::tests::delivery::directories::rename_events_keep_leaf_and_directory_symlink_aliases ... ok
test live::watcher::tests::delivery::closed_receiver_allows_callbacks_to_finish ... ok
test live::watcher::tests::delivery::source_burst_preserves_every_changed_path ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 333 filtered out; finished in 0.00s
```

## live-external-alias-red.log

[target/idle-cpu-measurement/live-external-alias-red.log](../target/idle-cpu-measurement/live-external-alias-red.log)

```text
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.70s
     Running tests/suite.rs (target/debug/deps/suite-1faeda2511223a1e)

running 1 test

thread 'live::aliases::renamed_external_source_alias_preserves_the_complete_cold_report' (3640701) panicked at crates/deslop-core/tests/live/directories.rs:84:5:
assertion `left == right` failed
  left: 3
 right: 2
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test live::aliases::renamed_external_source_alias_preserves_the_complete_cold_report ... FAILED

failures:

failures:
    live::aliases::renamed_external_source_alias_preserves_the_complete_cold_report

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 214 filtered out; finished in 0.15s

error: test failed, to rerun pass `-p deslop-core --test suite`
```

## live-external-alias-green.log

[target/idle-cpu-measurement/live-external-alias-green.log](../target/idle-cpu-measurement/live-external-alias-green.log)

```text
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.47s
     Running tests/suite.rs (target/debug/deps/suite-1faeda2511223a1e)

running 30 tests
test live::directories::directory_event_honours_ancestor_directory_ignore_rules ... ok
test live::aliases::renamed_external_source_alias_preserves_the_complete_cold_report ... ok
test live::directories::directory_event_recovers_every_preexisting_descendant ... ok
test live::directories::excluded_directory_event_preserves_the_existing_report ... ok
test live::directories::root_event_reconciles_missing_and_new_source_paths ... ok
test live::directories::directory_event_honours_new_nested_ignore_overrides ... ok
test live::directories::deleted_directory_releases_its_ignore_rules ... ok
test live::directories::root_rescan_reloads_missed_config_and_ignore_edits ... ok
test live::embedding_running_progress_reaches_total_with_duplicate_snippets ... ok
test live::analysis_session_new_surfaces_error_for_unreadable_config_path ... ok
test live::deeply_nested_dart_change_is_skipped_without_crashing_the_session ... ok
test live::embedding_list_models_returns_empty_when_ollama_unreachable ... ok
test live::debouncer_coalesces_burst_and_flushes_at_cap ... ok
test live::embedding_refresh_keeps_latest_report_readable_while_provider_is_blocked ... ok
test live::find_similar_on_below_min_nodes_snippet_returns_below_min_nodes_flag ... ok
test live::find_similar_on_known_range_returns_expected_cluster ... ok
test live::find_similar_on_unparseable_snippet_returns_unparseable_error ... ok
test live::issue_222_agent_worktree_copies_never_enter_live_report ... ok
test live::live_analysis_session_honors_scan_root_relative_report_hide ... ok
test live::live_loop_evicts_newly_gitignored_tree_after_gitignore_edit ... ok
test live::live_loop_hides_cluster_after_deslop_toml_report_hide_edit ... ok
test live::live_service_round_trip_covers_the_query_surface ... ok
test live::live_session_first_report_matches_batch_run ... ok
test live::live_session_initial_report_does_not_run_embeddings ... ok
test live::removing_a_directory_evicts_every_occurrence_under_it ... ok
test live::removing_a_directory_spares_a_sibling_with_a_shared_name_prefix ... ok
test live::removing_a_single_file_evicts_its_occurrence ... ok
test live::update_files_produces_non_empty_delta_when_a_file_changes ... ok
test live::watcher_emits_event_for_every_modification_of_the_same_path ... ok
test live::watcher_forwards_directory_removal_without_a_source_extension ... ok

test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 185 filtered out; finished in 7.89s
```

## live-external-alias-clippy.log

[target/idle-cpu-measurement/live-external-alias-clippy.log](../target/idle-cpu-measurement/live-external-alias-clippy.log)

```text
    Checking deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
    Checking deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.97s
```

## Missing-path native alias ordering

Failure excerpt mechanically extracted from [target/idle-cpu-directory-ci.log](../target/idle-cpu-directory-ci.log).

```text
thread 'rerun::removals::unknown_build_removal_preserves_report_with_bounded_path_lookup' (3799067) panicked at crates/deslop/tests/rerun/removals.rs:100:5:
assertion `left == right` failed: the removal lookup must publish its work count: 2026-10-03T23:07:30.875985Z  INFO deslop invoked path=/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpD1g3E6/src min_nodes=8 json=true text=true html=true embeddings="off" incremental=false
  left: 0
 right: 1
```

## missing-alias-removal-final-green.log

[target/idle-cpu-measurement/missing-alias-removal-final-green.log](../target/idle-cpu-measurement/missing-alias-removal-final-green.log)

```text
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
   Compiling deslop v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.64s
     Running tests/suite.rs (target/debug/deps/suite-527e3a836c4477d4)

running 2 tests
test rerun::removals::unknown_build_removal_preserves_report_with_bounded_path_lookup ... ok
test rerun::removals::removing_an_empty_source_updates_membership_and_preserves_the_clone ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 544 filtered out; finished in 1.39s
```

## external-alias-order-green.log

[target/idle-cpu-measurement/external-alias-order-green.log](../target/idle-cpu-measurement/external-alias-order-green.log)

```text
   Compiling deslop-core v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-core)
   Compiling deslop-test-support v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop-test-support)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.84s
     Running tests/suite.rs (target/debug/deps/suite-1faeda2511223a1e)

running 1 test
test live::aliases::renamed_external_source_alias_preserves_the_complete_cold_report ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 214 filtered out; finished in 0.06s
```
