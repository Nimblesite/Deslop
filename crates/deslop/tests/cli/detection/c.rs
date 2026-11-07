//! [LANG-CAND-C] C goes through the same CLI as every language.

use super::*;

const SMALL_FIXTURE: &str = "c-small";
const SMALL_MIN_NODES: &str = "10";
const SMALL_FIRST: &str = "alpha.c";
const SMALL_SECOND: &str = "beta.c";
const TYPE3_FIXTURE: &str = "c-type3";
const TYPE3_MIN_NODES: &str = "8";
const TYPE3_FIRST: &str = "delta.c";
const TYPE3_SECOND: &str = "epsilon.c";
const DISSIMILAR_FIXTURE: &str = "c-dissimilar-functions";
const DISSIMILAR_MIN_NODES: &str = "8";
const PROLOGUE_FIXTURE: &str = "c-prologue-false-positive";
const PROLOGUE_MIN_NODES: &str = "15";
const PROLOGUE_FILE_COUNT: u64 = 6;
/// Six files of 31–39 rows each; the parser must have consumed them all.
const PROLOGUE_MIN_ANALYSED_LOC: u64 = 150;
const HEADERS_FIXTURE: &str = "c-headers";
const HEADERS_MIN_NODES: &str = "8";
/// `util.h`, `math_shim.h`, `clock_shim.h`, `util.c` and `main.c`: every
/// header counts as an analysed file.
const HEADERS_FILE_COUNT: u64 = 5;
const HEADER_FILE: &str = "util.h";
const CLEANUP_FIXTURE: &str = "c-error-cleanup";
const CLEANUP_MIN_NODES: &str = "12";
const CLEANUP_FIRST: &str = "frame_reader.c";
const CLEANUP_SECOND: &str = "sample_loader.c";
/// `frame_open` and `sample_load` share a copied "cannot open" error path:
/// log, free the struct, return `NULL`.
const CLEANUP_OPEN_FAILURE_FIRST: (u64, u64) = (17, 22);
const CLEANUP_OPEN_FAILURE_SECOND: (u64, u64) = (20, 25);
/// The short-read error path and `frame_close` free the same resources in
/// the same order inside one file.
const CLEANUP_TAIL_FIRST: (u64, u64) = (32, 34);
const CLEANUP_TAIL_SECOND: (u64, u64) = (44, 46);
const POINTERS_FIXTURE: &str = "c-function-pointers";
const POINTERS_MIN_NODES: &str = "8";
const POINTERS_FIRST: &str = "hooks_alpha.c";
const POINTERS_SECOND: &str = "hooks_beta.c";
const POINTERS_HEADER: &str = "hooks.h";
/// The four function-pointer variables both files copied, by row.
const POINTERS_TABLE: (u64, u64) = (3, 6);
const LANGUAGE_LABEL: &str = "C";
const LANGUAGE_ID: &str = "c";
const NO_CLUSTERS: &[Value] = &[];

// [LANG-CAND-C] Type-2: the same function with every identifier renamed.
#[test]
fn detects_type2_clone_in_c_fixture() -> Result<()> {
    let json = run_min_nodes(SMALL_FIXTURE, SMALL_MIN_NODES)?;
    let cluster = require_type2_cluster(&json, SMALL_FIRST, SMALL_SECOND)?;
    for occurrence in require_array(&cluster, "/occurrences", "C Type-2 cluster")? {
        assert_eq!(
            occurrence.get("language").and_then(Value::as_str),
            Some(LANGUAGE_ID),
            "every occurrence carries the C parser identity: {occurrence}"
        );
    }
    Ok(())
}

// [FUSED-SIGNALS-THREE-LAYER] Type-3: `delta.c` runs two accumulator
// updates per iteration, `epsilon.c` one. The shared loop body surfaces
// as a cross-file cluster that is not byte-identical.
#[test]
fn detects_type3_clone_in_c_fixture() -> Result<()> {
    let json = run_min_nodes(TYPE3_FIXTURE, TYPE3_MIN_NODES)?;
    let scan_root = fixture(TYPE3_FIXTURE);
    let clusters = report_clusters(&json)?;
    assert_eq!(
        files_analysed(&json)?,
        TYPE2_EXPECTED_FILES_ANALYSED,
        "the c-type3 fixture is a two-file pair"
    );
    let cluster = require_cluster_spanning(&clusters, TYPE3_FIRST, TYPE3_SECOND)?;
    assert_type3_signals(&scan_root, cluster, LANGUAGE_LABEL)
}

// Zero-false-positive guard: `tally()` counts into an array inside a
// loop; `describe()` is a chain of early returns. Never paired.
#[test]
fn dissimilar_c_functions_across_files_stay_in_separate_clusters() -> Result<()> {
    let json = run_min_nodes(DISSIMILAR_FIXTURE, DISSIMILAR_MIN_NODES)?;
    assert_every_cluster_single_file(&json, LANGUAGE_LABEL)
}

// [LANG-CAND-C-PREPROC] [PIPELINE-BOILERPLATE-FILTER] Six files share the
// same `#include` block and `#define` constants and nothing else. The
// honest report has no clusters; the metrics prove the scan was live.
#[test]
fn c_include_and_define_prologue_never_becomes_a_cluster() -> Result<()> {
    let (scan_root, report) =
        run_with_args(PROLOGUE_FIXTURE, &[MIN_NODES_FLAG, PROLOGUE_MIN_NODES])?;
    assert_eq!(
        require_u64(&report, "/files_analysed", "report")?,
        PROLOGUE_FILE_COUNT,
        "all six C files must be analysed; report={report:#?}"
    );
    let analysed_loc = require_u64(&report, "/metrics/analysed_loc", "report")?;
    assert!(
        analysed_loc >= PROLOGUE_MIN_ANALYSED_LOC,
        "the six divergent C files must all be parsed (analysed_loc >= \
         {PROLOGUE_MIN_ANALYSED_LOC}, got {analysed_loc})"
    );
    assert_eq!(
        require_array(&report, "/clusters", "report")?.as_slice(),
        NO_CLUSTERS,
        "a shared preprocessor prologue is scaffolding, not duplication"
    );
    assert_no_cross_file_prologue_cluster(&report, &scan_root, "c prologue");
    Ok(())
}

// [LANG-CAND-C] [LANG-CAND-C-PROTOTYPE] `.h` is C: every header is
// analysed, and an include guard plus a run of prototypes — the same
// shape in `math_shim.h`, `clock_shim.h` and `util.h` — is never a cluster.
#[test]
fn c_headers_are_analysed_without_manufacturing_clusters() -> Result<()> {
    let (_scan_root, report) =
        run_with_args(HEADERS_FIXTURE, &[MIN_NODES_FLAG, HEADERS_MIN_NODES])?;
    assert_eq!(
        require_u64(&report, "/files_analysed", "report")?,
        HEADERS_FILE_COUNT,
        "the header must count as an analysed C file; report={report:#?}"
    );
    let rows = per_file_metrics(&report);
    let header = row_for_path(rows, HEADER_FILE)
        .ok_or_else(|| anyhow::anyhow!("{HEADER_FILE} missing from per-file rows: {rows:#?}"))?;
    assert_eq!(
        header.get("language").and_then(Value::as_str),
        Some(LANGUAGE_ID),
        "the header row carries the C parser identity: {header}"
    );
    assert_eq!(
        require_array(&report, "/clusters", "report")?.as_slice(),
        NO_CLUSTERS,
        "an include guard and prototypes are not duplication"
    );
    Ok(())
}

/// Every `(start_line, end_line)` span the cluster reports for `file`.
fn spans_in(cluster: &Value, file: &str) -> Vec<(u64, u64)> {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|occurrence| {
            occurrence
                .get("path")
                .and_then(Value::as_str)
                .is_some_and(|path| path.ends_with(file))
        })
        .map(occurrence_line_span)
        .collect()
}

/// Fails unless some cluster reports both spans, each in its named file.
fn require_cluster_with_spans(
    clusters: &[Value],
    first: (&str, (u64, u64)),
    second: (&str, (u64, u64)),
) -> Result<()> {
    let found = clusters.iter().any(|cluster| {
        spans_in(cluster, first.0).contains(&first.1)
            && spans_in(cluster, second.0).contains(&second.1)
    });
    anyhow::ensure!(
        found,
        "expected a cluster pairing {}:{}-{} with {}:{}-{}; got {clusters:#?}",
        first.0,
        first.1 .0,
        first.1 .1,
        second.0,
        second.1 .0,
        second.1 .1
    );
    Ok(())
}

// [LANG-CAND-C] Osprey's runtime maintainers asked for copied error-path
// and cleanup blocks: `if (x == NULL) { fprintf(...); free(...); return
// NULL; }` ladders that grow by copy and drift apart one fix at a time.
// Two loaders share the ladder across files; inside one file the
// short-read path and the destructor release resources identically.
#[test]
fn copied_c_error_paths_and_cleanup_tails_are_reported() -> Result<()> {
    let json = run_min_nodes(CLEANUP_FIXTURE, CLEANUP_MIN_NODES)?;
    assert_eq!(
        files_analysed(&json)?,
        TYPE2_EXPECTED_FILES_ANALYSED,
        "the c-error-cleanup fixture is a two-file pair"
    );
    let clusters = report_clusters(&json)?;
    let cross_file = require_cluster_spanning(&clusters, CLEANUP_FIRST, CLEANUP_SECOND)?;
    assert_type3_signals(&fixture(CLEANUP_FIXTURE), cross_file, LANGUAGE_LABEL)?;
    require_cluster_with_spans(
        &clusters,
        (CLEANUP_FIRST, CLEANUP_OPEN_FAILURE_FIRST),
        (CLEANUP_SECOND, CLEANUP_OPEN_FAILURE_SECOND),
    )?;
    require_cluster_with_spans(
        &clusters,
        (CLEANUP_FIRST, CLEANUP_TAIL_FIRST),
        (CLEANUP_FIRST, CLEANUP_TAIL_SECOND),
    )
}

// [LANG-CAND-C-PROTOTYPE] `int (*on_open)(const char *, int);` declares a
// variable that holds a function pointer, not a function: the declarator is
// parenthesised. Two files that copied the same hook table are a real clone,
// while `hooks.h` (prototypes only) still contributes nothing.
#[test]
fn copied_c_function_pointer_tables_are_reported_but_prototypes_are_not() -> Result<()> {
    let json = run_min_nodes(POINTERS_FIXTURE, POINTERS_MIN_NODES)?;
    let clusters = report_clusters(&json)?;
    require_cluster_with_spans(
        &clusters,
        (POINTERS_FIRST, POINTERS_TABLE),
        (POINTERS_SECOND, POINTERS_TABLE),
    )?;
    for cluster in &clusters {
        assert!(
            !cluster_file_basenames(cluster).contains(POINTERS_HEADER),
            "a header of prototypes is scaffolding, never a clone: {cluster:#}"
        );
    }
    Ok(())
}
