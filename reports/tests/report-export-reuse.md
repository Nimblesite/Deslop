# Report export reuse — test-first evidence

## [OUTPUT-REPORT-REUSE]

Unchanged report exports must reuse existing completed calculations across JSON, text and HTML; source changes must invalidate them.

- [x] Trace CLI and VS Code command paths
- [x] Add a black-box CLI scenario with populated duplicate output
- [x] Execute the new test and reproduce analysis on unchanged input
- [ ] Obtain the failing-test acknowledgment required by fix-bug Step 4
- [x] Verify the editor server export path with a focused real-binary scenario; extension-host reproduction remains pending
- [ ] Implement reuse and validate invalidation
- [ ] Run the required full suite

The CLI unconditionally initialises PipelineSession unless --from-report is supplied. The VS Code command forwards to the LSP renderHtmlReport command, which reads LiveService.report_get; its reported redundant analysis has not yet been independently reproduced.

Command: `cargo test -p deslop --test suite cli::from_report::reuse::unchanged_exports_reuse_calculations_and_source_changes_invalidate_them -- --exact --nocapture`

Exit code: `101`

## Test code

```rust
//! [OUTPUT-REPORT-REUSE] Export formats share completed, current calculations.

use super::super::support::*;

/// The trace emitted when the CLI starts analysis rather than rendering a snapshot.
const ANALYSIS_STARTED: &str = "pipeline session initialising";
/// Expected source-file population before deleting a copy.
const INITIAL_FILES: u64 = 2;
/// Expected source-file population after deleting a copy.
const REMAINING_FILES: u64 = 1;
/// The only duplicated group is ranked first.
const FIRST_RANK: u64 = 1;
/// The source removed to invalidate the report cache.
const DELETED_COPY: &str = "Beta.cs";
/// Rendering-only options must not invalidate completed calculations.
const FORMAT_OPTIONS: &[&[&str]] = &[&["--notext"], &["--nohtml"], &["--nojson"]];

#[test]
fn unchanged_exports_reuse_calculations_and_source_changes_invalidate_them() -> Result<()> {
    let (tmp, root) = seeded_fixture_root(CSHARP_SMALL_FIXTURE)?;
    let base = tmp.path().join(REPORT_OUTPUT_STEM);
    let cold_log = export(&root, &base, &[])?;
    assert!(
        cold_log.contains(ANALYSIS_STARTED),
        "cold scan must analyse"
    );
    let original = read_json_report(&base.with_extension("json"))?;
    assert_populated_report(&original);
    assert_unchanged_exports(&root, &base, &original)?;
    assert_deleted_copy_invalidates(&root, &base)
}

/// A changed source must replace the calculations, and that result is reusable too.
fn assert_deleted_copy_invalidates(root: &Path, base: &Path) -> Result<()> {
    fs::remove_file(root.join(DELETED_COPY))?;
    let changed_log = export(root, base, &[])?;
    assert!(
        changed_log.contains(ANALYSIS_STARTED),
        "deletion must invalidate calculations"
    );
    let changed = read_json_report(&base.with_extension("json"))?;
    assert_eq!(field(&changed, "files_analysed"), REMAINING_FILES);
    assert_eq!(field(&changed, "clusters"), &serde_json::json!([]));
    assert_unchanged_exports(root, base, &changed)
}

/// Drives the actual CLI and captures analysis traces without timing assertions.
fn export(root: &Path, base: &Path, formats: &[&str]) -> Result<String> {
    let mut command = deslop_command(root, base)?;
    let assertion = command
        .env("RUST_LOG", "info")
        .args([
            MIN_NODES_FLAG,
            MIN_NODES_VALUE,
            NO_COLOR_FLAG,
            "--log-to-console",
        ])
        .args(formats)
        .assert()
        .success();
    Ok(String::from_utf8(assertion.get_output().stderr.clone())?)
}

/// Pins non-empty detector output before testing reuse, so blindness cannot pass.
fn assert_populated_report(report: &Value) {
    assert_eq!(field(report, "files_analysed"), INITIAL_FILES);
    let clusters = field(report, "clusters").as_array().expect("cluster array");
    let first = clusters.first().expect("real duplicated C# methods");
    assert_eq!(field(first, "rank"), FIRST_RANK);
    let occurrences = field(first, "occurrences").as_array().expect("occurrences");
    assert_eq!(
        occurrences.len(),
        usize::try_from(INITIAL_FILES).expect("small count")
    );
    let paths: std::collections::BTreeSet<_> = occurrences
        .iter()
        .map(|occurrence| field(occurrence, "path").as_str().expect("source path"))
        .collect();
    assert_eq!(paths, ["Alpha.cs", DELETED_COPY].into_iter().collect());
}

/// Each format renders the same snapshot without another pipeline initialisation.
fn assert_unchanged_exports(root: &Path, base: &Path, expected: &Value) -> Result<()> {
    for formats in FORMAT_OPTIONS {
        let log = export(root, base, formats)?;
        assert!(
            !log.contains(ANALYSIS_STARTED),
            "unchanged report export must reuse completed calculations; analysis restarted:\n{log}"
        );
        let actual = read_json_report(&base.with_extension("json"))?;
        assert_eq!(
            &actual, expected,
            "all calculated report fields must be preserved"
        );
        assert!(fs::read_to_string(base.with_extension("html"))?.contains("Deslop report"));
        assert!(!fs::read_to_string(base.with_extension("txt"))?.is_empty());
    }
    Ok(())
}
```

## Failure output

```text
   Compiling deslop v0.0.0-dev (/Users/christianfindlay/Documents/Code/Deslop/crates/deslop)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.56s
     Running tests/suite.rs (target/debug/deps/suite-e70fa3a35af2edb1)

running 1 test

thread 'cli::from_report::reuse::unchanged_exports_reuse_calculations_and_source_changes_invalidate_them' (21563359) panicked at crates/deslop/tests/cli/from_report/reuse.rs:86:9:
unchanged report export must reuse completed calculations; analysis restarted:
deslop scanning /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpVQgsBM/src for duplicated code...
  report → /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpVQgsBM/report.{json,txt,html}
  log    → stderr (--log-to-console)
2026-11-03T23:01:50.608248Z  INFO deslop invoked path=/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpVQgsBM/src min_nodes=8 json=true text=false html=true embeddings="off" incremental=true
2026-11-03T23:01:50.608369Z  INFO pipeline session initialising root=/private/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpVQgsBM/src root_exists=true root_is_dir=true min_nodes=8 incremental=true supported_extensions=["php", "ospml", "fsx", "osp", "cs", "jsx", "cjs", "mjs", "dart", "rs", "tsx", "go", "fs", "ts", "py", "js"]
2026-11-03T23:01:50.611537Z  INFO file discovery complete root=/private/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpVQgsBM/src total_files=2 by_language={"csharp": 2}
2026-11-03T23:01:50.618613Z  INFO fingerprint corpus built files_processed=2 fingerprints=58 cache_hits=2 cache_misses=0 signatures_built=0 signatures_reused=58 exact_fingerprints=0 sibling_fingerprints=0 read_ms=0 parse_ms=0 fingerprint_ms=0 signature_ms=0 elapsed_ms=4 rss_mib=15 signature_mib=0 source_mib=0
2026-11-03T23:01:50.618774Z  INFO fingerprint store swept other_version_blobs=0 orphan_blobs=0 evicted_blobs=0 store_bytes=66996
2026-11-03T23:01:50.621195Z  INFO pairs: pre-structural rss_mib=15
2026-11-03T23:01:50.623762Z  INFO pairs: post-structural rss_mib=15 evidence=29
2026-11-03T23:01:50.629351Z  INFO pairs: post-lsh rss_mib=16 kept=71 lsh_scanned=1466
2026-11-03T23:01:50.631366Z  INFO pairs: post-resolve rss_mib=16 resolved=71
2026-11-03T23:01:50.633551Z  INFO cluster stage complete stage="candidate_pairs" input=58 output=71 elapsed_ms=12 rss_mib=16
2026-11-03T23:01:50.634149Z  INFO normalised trees materialised files=2 elapsed_ms=0
2026-11-03T23:01:50.638906Z  INFO scanned=71 eligible=42 cross_file=42 same_file=0 measured=1 rescued_pairs=1 content_gate_rejected=0 container_echo_rejected=0 echo_bound_skipped=41 hard_content_skipped=0 core_preflight_skipped=0 alignments=1 credit_fallbacks=0 hash_equal=0 exact_hits=0 bound_hits=0 bound_skips=0 order_skips=0 unresolved=0 elapsed_ms=4 shared-subtree rescue overlaps measured
2026-11-03T23:01:50.642537Z  INFO cluster stage complete stage="shared_subtree_rescue" input=71 output=71 elapsed_ms=4 rss_mib=18
2026-11-03T23:01:50.642651Z  INFO pair survival outcome total=71 survived=50 survived_shared_subtree=0 dropped_below_fused=13 dropped_lsh_only_jaccard=8 dropped_lsh_only_node_count=0 dropped_size_mismatch=0
2026-11-03T23:01:50.642908Z  INFO copy recovery workload families=16 eligible=15 comparisons=15 max_members=2
2026-11-03T23:01:50.678303Z  INFO cluster stage complete stage="pair_content_gate" input=71 output=41 elapsed_ms=15 rss_mib=18
2026-11-03T23:01:50.678346Z  INFO pair survival outcome total=41 survived=20 survived_shared_subtree=0 dropped_below_fused=13 dropped_lsh_only_jaccard=8 dropped_lsh_only_node_count=0 dropped_size_mismatch=0
2026-11-03T23:01:50.684720Z  INFO cluster stage complete stage="transitive_closure" input=41 output=20 elapsed_ms=0 rss_mib=18
2026-11-03T23:01:50.687564Z  INFO cluster stage complete stage="structural_family_split" input=20 output=20 elapsed_ms=0 rss_mib=18
2026-11-03T23:01:50.689679Z  INFO cluster stage complete stage="noise_verbatim_split" input=20 output=20 elapsed_ms=0 rss_mib=19
2026-11-03T23:01:50.689833Z  INFO informational count-bound workload families=16 eligible=16 skipped=0
2026-11-03T23:01:50.712842Z  INFO cross-cluster subsumption complete stage="cross_cluster_subsume" views=20 file_sets=1 evaluated=190 same_region=187 absorbed=19 cycles=0 straddle_rounds=0 straddled=0 survivors=1 elapsed_ms=0
2026-11-03T23:01:50.712866Z  INFO ranked clusters built total=1 input_total=20 dropped_below_min_members=0 largest_mass=58
2026-11-03T23:01:50.723494Z  INFO cross-cluster subsumption complete stage="cross_cluster_subsume" views=0 file_sets=0 evaluated=0 same_region=0 absorbed=0 cycles=0 straddle_rounds=0 straddled=0 survivors=0 elapsed_ms=0
2026-11-03T23:01:50.723511Z  INFO ranked clusters built total=0 input_total=16 dropped_below_min_members=16 largest_mass=0
2026-11-03T23:01:50.726600Z  INFO cluster stage complete stage="ranked_build" input=20 output=1 elapsed_ms=33 rss_mib=19
2026-11-03T23:01:50.726636Z  INFO render complete ranked_clusters=1 fingerprints=58
2026-11-03T23:01:50.726646Z  INFO pipeline stage stage="candidate_pairs" input=58 output=71 elapsed_ms=12
2026-11-03T23:01:50.726651Z  INFO pipeline stage stage="shared_subtree_rescue" input=71 output=71 elapsed_ms=4
2026-11-03T23:01:50.726655Z  INFO pipeline stage stage="pair_content_gate" input=71 output=41 elapsed_ms=15
2026-11-03T23:01:50.726660Z  INFO pipeline stage stage="transitive_closure" input=41 output=20 elapsed_ms=0
2026-11-03T23:01:50.726664Z  INFO pipeline stage stage="structural_family_split" input=20 output=20 elapsed_ms=0
2026-11-03T23:01:50.726668Z  INFO pipeline stage stage="noise_verbatim_split" input=20 output=20 elapsed_ms=0
2026-11-03T23:01:50.726673Z  INFO pipeline stage stage="ranked_build" input=20 output=1 elapsed_ms=33
2026-11-03T23:01:50.727551Z  INFO cluster noise filter totals stage="run_cumulative_after_report_render" filter="constant_table" calls=1 members=2 fired=0 micros=2
2026-11-03T23:01:50.727577Z  INFO cluster noise filter totals stage="run_cumulative_after_report_render" filter="data_rows" calls=1 members=2 fired=0 micros=74
2026-11-03T23:01:50.727591Z  INFO cluster noise filter totals stage="run_cumulative_after_report_render" filter="language_specific" calls=1 members=2 fired=0 micros=0
2026-11-03T23:01:50.727604Z  INFO cluster noise filter totals stage="run_cumulative_after_report_render" filter="literal_calls" calls=1 members=2 fired=0 micros=238
2026-11-03T23:01:50.727617Z  INFO cluster noise filter totals stage="run_cumulative_after_report_render" filter="polymorphic" calls=1 members=2 fired=0 micros=17
2026-11-03T23:01:50.727629Z  INFO cluster noise filter totals stage="run_cumulative_after_report_render" filter="signature_only" calls=1 members=2 fired=0 micros=5
2026-11-03T23:01:50.727772Z  INFO mass-ranked report built stage="report_build" visible_clusters=1 clusters_hidden=0 highest_mass=58 elapsed_ms=1
2026-11-03T23:01:50.728385Z  INFO wrote report file path=/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpVQgsBM/report.json bytes=2334
2026-11-03T23:01:50.729007Z  INFO wrote report file path=/var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpVQgsBM/report.html bytes=57770

Found 1 groups of duplicated code across 2 file(s)
  skipped 2 unchanged file(s) using the cache
  diagnostic severity: 1 × warning
  Highest mass: 58 from 2 occurrences × 58 canonical nodes in Alpha.cs, Beta.cs

Top 1 findings
  #1  Nearly identical code — mass 58, 2 occurrences in Alpha.cs, Beta.cs

Next: open the .html report to inspect occurrences or explicitly compare two endpoints.

✔ done
    report /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpVQgsBM/report.json
    report /var/folders/v3/49bmt6f55b1csb6snk8fr1mw0000gn/T/.tmpVQgsBM/report.html

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test cli::from_report::reuse::unchanged_exports_reuse_calculations_and_source_changes_invalidate_them ... FAILED

failures:

failures:
    cli::from_report::reuse::unchanged_exports_reuse_calculations_and_source_changes_invalidate_them

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 1.37s

error: test failed, to rerun pass `-p deslop --test suite`
```

## Editor server follow-up

The current source build and a separate instance of the installed VS Code server both preserve report fields and generation on repeated HTML/JSON exports; the scenario also deletes a source and checks reuse of the changed snapshot. Commands, test code, and captured outputs are in [live-report-export-reuse.md](live-report-export-reuse.md). This narrows the reported editor symptom without claiming extension-host reproduction.

## Archived reproduction

This file preserves the failing state captured before implementation. The user acknowledged it before production changes. Current results are in [report-reuse-verification.md](report-reuse-verification.md).
