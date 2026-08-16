//! [PIPELINE-DETERMINISM] Cold-run report golden — Phase 0 of
//! `docs/plans/incremental-analysis-plan.md`.
//!
//! One fixed corpus (`tests/fixtures/report-golden/src`), one fixed flag
//! set (`--no-incremental --embeddings off --min-nodes 16 --notext
//! --nohtml`), one committed rendering
//! (`tests/fixtures/report-golden/expected-report.json`). The pipeline
//! must render bit-identical reports over an unchanged corpus, and every
//! later incremental phase (warm cache, delta re-analysis, live
//! sessions) must reproduce this exact cold report — so any drift in
//! ranking, spans, cluster ids, metrics arithmetic, or serialisation
//! order fails here first.
//!
//! Two halves, mirroring the AST golden guard in
//! `tests/cli/cache_and_debug.rs`: **unchanged** — the rendered bytes
//! must equal the committed golden byte-for-byte; **correct** — the
//! committed golden must independently satisfy invariants derived from
//! the authored fixture sources, so a wrongly-blessed golden cannot
//! self-certify.
//!
//! Regenerate with `DESLOP_BLESS=1 cargo test -p deslop --test
//! report_golden`, then review the diff — see
//! `tests/fixtures/report-golden/README.md`.

use std::{fs, path::PathBuf};

use anyhow::Context as _;
use serde_json::Value;

mod common;
use crate::common::*;

/// Fixed `--min-nodes` the golden is rendered at. 16 sits above the
/// 12-node `settle_invoice` signature subtree (which would otherwise
/// surface as a third cluster) and below both authored clone bodies
/// (58 and 38 nodes), so exactly the two authored clusters render.
const GOLDEN_MIN_NODES: u64 = 16;

/// The three corpus files carrying the byte-identical `settle_invoice`
/// clone — the larger, higher-ranked cluster.
const TRIO_FILES: [&str; 3] = ["alpha.rs", "beta.rs", "gamma.rs"];

/// The two corpus files carrying the byte-identical `merge_labels`
/// clone — the smaller, lower-ranked cluster.
const PAIR_FILES: [&str; 2] = ["delta.rs", "epsilon.rs"];

/// `tests/fixtures/report-golden`.
fn golden_dir() -> PathBuf {
    fixture("report-golden")
}

/// The authored corpus the golden report describes.
fn corpus_dir() -> PathBuf {
    golden_dir().join("src")
}

/// The committed golden rendering.
fn golden_report_path() -> PathBuf {
    golden_dir().join("expected-report.json")
}

/// Copies the corpus into a throwaway scan root and renders the cold
/// report with the fixed flag set, returning the raw JSON bytes. The
/// checked-in fixture is never scanned in place, and `deslop_cmd`
/// carries `--no-incremental`, so no run can seed a cache anywhere.
fn render_cold_report() -> Result<Vec<u8>> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed(&corpus_dir(), &scan_root)?;
    let output = tmp.path().join("out").join("report");
    let mut cmd = deslop_cmd(&scan_root, &output)?;
    let _assertion = cmd
        .args(["--embeddings", "off", "--min-nodes"])
        .arg(GOLDEN_MIN_NODES.to_string())
        .args(["--notext", "--nohtml"])
        .assert()
        .success();
    Ok(fs::read(output.with_extension("json"))?)
}

// [PIPELINE-DETERMINISM] Half one: unchanged. Two fresh scan roots must
// render byte-identical reports, and those bytes must equal the
// committed golden exactly. `tool_version` is embedded in the report,
// so a workspace version bump legitimately lands here and requires a
// reviewed re-bless.
#[test]
fn cold_report_matches_committed_golden_byte_for_byte() -> Result<()> {
    let first = render_cold_report()?;
    let second = render_cold_report()?;
    let rendered = String::from_utf8(first)?;
    assert_eq!(
        rendered,
        String::from_utf8(second)?,
        "two cold renders over identical corpora must be bit-identical [PIPELINE-DETERMINISM]"
    );
    if std::env::var_os("DESLOP_BLESS").is_some() {
        fs::write(golden_report_path(), rendered.as_bytes())?;
        anyhow::bail!(
            "golden re-blessed at {}; review the diff, then re-run without DESLOP_BLESS to verify",
            golden_report_path().display()
        );
    }
    let expected = fs::read_to_string(golden_report_path()).with_context(|| {
        format!(
            "missing golden {} — generate it with `DESLOP_BLESS=1 cargo test -p deslop --test report_golden`",
            golden_report_path().display()
        )
    })?;
    assert_eq!(
        rendered,
        expected,
        "cold report drifted from {} [PIPELINE-DETERMINISM]. Regenerating is NOT the default \
         remedy — ranking, spans, ids, and metrics all change user-visible output, so prove the \
         drift is intended, then re-bless with `DESLOP_BLESS=1 cargo test -p deslop --test \
         report_golden` and review the diff.",
        golden_report_path().display()
    );
    Ok(())
}

// [PIPELINE-DETERMINISM] Half two: correct. Byte equality alone only
// proves the tool still agrees with a file the tool wrote; these
// invariants come from the authored corpus instead, so a wrongly-blessed
// golden fails here even when it matches the committed bytes exactly.
#[test]
fn committed_golden_satisfies_report_contract() -> Result<()> {
    let golden = load_json(&golden_report_path()).with_context(|| {
        format!(
            "unreadable golden {} — generate it with `DESLOP_BLESS=1 cargo test -p deslop --test report_golden`",
            golden_report_path().display()
        )
    })?;
    assert_cold_scan_header(&golden);
    assert_occurrences_are_real_type1_clones(&golden)?;
    assert_ranking_and_cluster_shape(&golden);
    assert_metrics_arithmetic(&golden);
    Ok(())
}

/// The fixed-flag scan header: five files analysed, the pinned
/// `min_nodes`, nothing hidden, embeddings off, and a cache that was
/// never consulted — `--no-incremental` defines the cold golden, so both
/// counters must be zero.
fn assert_cold_scan_header(golden: &Value) {
    assert_eq!(
        field(golden, "files_analysed").as_u64(),
        Some(5),
        "the corpus has exactly five source files: {golden}"
    );
    assert_eq!(
        field(golden, "min_nodes").as_u64(),
        Some(GOLDEN_MIN_NODES),
        "golden must be rendered at --min-nodes {GOLDEN_MIN_NODES}: {golden}"
    );
    assert_eq!(
        clusters_hidden(golden),
        0,
        "no cluster in this corpus is suppressible: {golden}"
    );
    let cache = field(golden, "cache_stats");
    assert_eq!(
        field(cache, "hits").as_u64(),
        Some(0),
        "a cold --no-incremental run can never hit a cache: {golden}"
    );
    assert_eq!(
        field(cache, "misses").as_u64(),
        Some(0),
        "a cold --no-incremental run never consults a cache, so it cannot miss either: {golden}"
    );
    assert!(
        field(golden, "tool_version")
            .as_str()
            .is_some_and(|version| !version.is_empty()),
        "report must carry the embedding tool_version: {golden}"
    );
    assert!(
        field(golden, "embedding_provenance").is_null(),
        "--embeddings off must render null provenance: {golden}"
    );
    assert_eq!(
        array_len(golden, "boilerplate_hints"),
        0,
        "the authored corpus contains no boilerplate: {golden}"
    );
}

/// Every occurrence must point into the authored corpus — a real fixture
/// file, a byte range inside that file, ordered one-based lines, not
/// hidden — and, because the corpus is authored as pure Type-1 clones,
/// every occurrence in a cluster must slice to source bytes identical to
/// its siblings'.
fn assert_occurrences_are_real_type1_clones(golden: &Value) -> Result<()> {
    assert!(
        !clusters(golden).is_empty(),
        "golden must report clusters: {golden}"
    );
    for cluster in clusters(golden) {
        let slices = validated_cluster_slices(cluster)?;
        let (first, rest) = slices
            .split_first()
            .expect("validated_cluster_slices asserts at least two occurrences");
        assert!(
            !first.is_empty(),
            "an occurrence slice can never be empty: {cluster}"
        );
        for slice in rest {
            assert_eq!(
                slice, first,
                "Type-1 corpus: every occurrence in a cluster must slice to identical source bytes: {cluster}"
            );
        }
    }
    Ok(())
}

/// Validates one cluster's occurrence bookkeeping (`size`,
/// `occurrences_total`, `occurrences_truncated`) and returns the
/// validated source slice behind each occurrence, read straight from the
/// checked-in corpus.
fn validated_cluster_slices(cluster: &Value) -> Result<Vec<Vec<u8>>> {
    let mut slices = Vec::new();
    for occurrence in occurrences(cluster) {
        slices.push(validated_occurrence_slice(occurrence)?);
    }
    let total = u64::try_from(slices.len()).expect("occurrence count fits u64");
    assert!(
        total >= 2,
        "a cluster below two occurrences is not a duplicate: {cluster}"
    );
    assert_eq!(
        cluster_size(cluster),
        total,
        "cluster size must equal its occurrence count: {cluster}"
    );
    assert_eq!(
        field(cluster, "occurrences_total").as_u64(),
        Some(total),
        "occurrences_total must equal the rendered occurrence count: {cluster}"
    );
    assert_eq!(
        field(cluster, "occurrences_truncated").as_bool(),
        Some(false),
        "this corpus is far below any truncation limit: {cluster}"
    );
    Ok(slices)
}

/// One occurrence: the path names a corpus file, `[start_byte,
/// end_byte)` lies within it, lines are one-based and ordered, and the
/// occurrence is visible. Returns the source slice it points at.
fn validated_occurrence_slice(occurrence: &Value) -> Result<Vec<u8>> {
    let path = occurrence_path(occurrence)?;
    let file = corpus_dir().join(path);
    assert!(
        file.is_file(),
        "occurrence path {path} does not exist in the corpus"
    );
    let source = fs::read(&file)?;
    let start = occurrence_byte(occurrence, "start_byte")?;
    let end = occurrence_byte(occurrence, "end_byte")?;
    assert!(
        start < end && end <= source.len(),
        "occurrence [{start}..{end}) must lie within {path}'s {} bytes",
        source.len()
    );
    let start_line = field(occurrence, "start_line").as_u64().expect("start_line");
    let end_line = field(occurrence, "end_line").as_u64().expect("end_line");
    assert!(
        start_line >= 1 && start_line <= end_line,
        "occurrence lines {start_line}..{end_line} in {path} must be one-based and ordered"
    );
    assert_eq!(
        field(occurrence, "hidden").as_bool(),
        Some(false),
        "no occurrence in this corpus is hidden: {occurrence}"
    );
    Ok(source
        .get(start..end)
        .expect("range asserted in bounds")
        .to_vec())
}

/// The two authored clusters and the ranking between them: the 3-copy
/// 58-node `settle_invoice` cluster must outrank the 2-copy 38-node
/// `merge_labels` cluster, weights must be finite, positive, and
/// non-increasing down the report, and every cluster must clear
/// `--min-nodes`.
fn assert_ranking_and_cluster_shape(golden: &Value) {
    let cluster_list = clusters(golden);
    assert_eq!(
        metric_field(golden, "clusters_total").as_u64(),
        u64::try_from(cluster_list.len()).ok(),
        "metrics.clusters_total must equal the rendered cluster list: {golden}"
    );
    assert!(
        cluster_list.len() >= 2,
        "the corpus authors two distinct duplicate clusters: {golden}"
    );
    assert_weights_ranked(cluster_list, golden);
    for cluster in cluster_list {
        let node_count = field(cluster, "canonical_node_count")
            .as_u64()
            .expect("canonical_node_count");
        assert!(
            node_count >= GOLDEN_MIN_NODES,
            "cluster below --min-nodes {GOLDEN_MIN_NODES}: {cluster}"
        );
    }
    let trio_rank = rank_of_exact_file_set(golden, &TRIO_FILES);
    let pair_rank = rank_of_exact_file_set(golden, &PAIR_FILES);
    assert!(
        trio_rank < pair_rank,
        "the 3-copy cluster must outrank the 2-copy cluster: {golden}"
    );
    assert_authored_cluster(cluster_list, trio_rank, 3);
    assert_authored_cluster(cluster_list, pair_rank, 2);
}

/// Weights down the report: finite, strictly positive, non-increasing.
fn assert_weights_ranked(cluster_list: &[Value], golden: &Value) {
    let weights: Vec<f64> = cluster_list
        .iter()
        .map(|cluster| field(cluster, "weight").as_f64().unwrap_or(f64::NAN))
        .collect();
    assert!(
        weights
            .iter()
            .all(|weight| weight.is_finite() && *weight > 0.0),
        "every cluster weight must be finite and positive: {weights:?} in {golden}"
    );
    assert!(
        weights
            .windows(2)
            .all(|pair| matches!(pair, [left, right] if left >= right)),
        "clusters must be ranked weight-non-increasing: {weights:?} in {golden}"
    );
}

/// One authored Type-1 cluster: exact occurrence count, `identical`
/// bucket, and full-strength structural and token signals with the
/// embedding channel switched off.
fn assert_authored_cluster(cluster_list: &[Value], rank: usize, copies: u64) {
    let cluster = cluster_list
        .get(rank)
        .expect("rank_of_exact_file_set returned an in-range index");
    assert_eq!(
        cluster_size(cluster),
        copies,
        "authored cluster must have exactly {copies} occurrences: {cluster}"
    );
    assert_eq!(
        cluster_bucket(cluster),
        "identical",
        "byte-identical clones must land in the identical bucket: {cluster}"
    );
    assert!(
        approx(signal(cluster, "structural"), 1.0),
        "byte-identical clones must reach structural identity: {cluster}"
    );
    assert!(
        approx(signal(cluster, "token_jaccard"), 1.0),
        "byte-identical clones must reach token identity: {cluster}"
    );
    assert!(
        approx(signal(cluster, "embedding_cos"), 0.0),
        "--embeddings off must zero the embedding signal: {cluster}"
    );
}

/// Zero-based rank of the cluster whose occurrences span exactly
/// `files`, panicking with the full report when absent.
fn rank_of_exact_file_set(golden: &Value, files: &[&str]) -> usize {
    let expected: std::collections::BTreeSet<String> =
        files.iter().map(|name| (*name).to_owned()).collect();
    clusters(golden)
        .iter()
        .position(|cluster| cluster_file_set(cluster) == expected)
        .unwrap_or_else(|| panic!("no cluster spans exactly {files:?}: {golden}"))
}

/// Repo metrics must be transparent arithmetic over the cluster list:
/// duplicated LOC recomputed from the visible occurrence spans, the
/// percentage recomputed from the two LOC totals, per-file rows summing
/// to the repo totals, and the fixed-flag run carrying no threshold.
fn assert_metrics_arithmetic(golden: &Value) {
    let analysed = metric_field(golden, "analysed_loc")
        .as_u64()
        .expect("analysed_loc");
    let duplicated = metric_field(golden, "duplicated_loc")
        .as_u64()
        .expect("duplicated_loc");
    assert!(analysed > 0, "five source files must analyse to LOC: {golden}");
    assert!(
        duplicated <= analysed,
        "duplicated LOC can never exceed analysed LOC: {golden}"
    );
    assert_eq!(
        duplicated,
        visible_duplicated_loc(golden),
        "metrics.duplicated_loc must equal the line set covered by visible occurrences: {golden}"
    );
    let percent = metric_field(golden, "duplication_percent")
        .as_f64()
        .expect("duplication_percent");
    let recomputed = 100.0 * loc_as_f64(duplicated) / loc_as_f64(analysed);
    assert!(
        (percent - recomputed).abs() <= 1e-6,
        "duplication_percent {percent} must equal 100*{duplicated}/{analysed} = {recomputed} within rendering precision"
    );
    assert_eq!(
        metric_field(golden, "duplicated_files").as_u64(),
        Some(5),
        "every corpus file carries a clone: {golden}"
    );
    assert_per_file_rows(golden, analysed, duplicated);
    let threshold = metric_field(golden, "threshold");
    assert_eq!(
        field(threshold, "source").as_str(),
        Some("none"),
        "the fixed flag set configures no threshold: {golden}"
    );
    assert_eq!(
        field(threshold, "breached").as_bool(),
        Some(false),
        "with no threshold nothing can breach: {golden}"
    );
}

/// Per-file metric rows: one per corpus file, each internally
/// consistent, and both LOC columns summing to the repo totals.
fn assert_per_file_rows(golden: &Value, analysed: u64, duplicated: u64) {
    let rows = per_file_metrics(golden);
    assert_eq!(rows.len(), 5, "one metric row per corpus file: {golden}");
    let mut analysed_sum = 0_u64;
    let mut duplicated_sum = 0_u64;
    for row in rows {
        let path = field(row, "path").as_str().expect("per_file.path");
        assert!(
            corpus_dir().join(path).is_file(),
            "per-file metric row {path} must reference a corpus file"
        );
        let row_analysed = field(row, "analysed_loc").as_u64().expect("per-file analysed_loc");
        let row_duplicated = field(row, "duplicated_loc")
            .as_u64()
            .expect("per-file duplicated_loc");
        assert!(
            row_duplicated <= row_analysed,
            "{path}: per-file duplicated LOC exceeds its analysed LOC: {row}"
        );
        analysed_sum += row_analysed;
        duplicated_sum += row_duplicated;
    }
    assert_eq!(
        analysed_sum, analysed,
        "per-file analysed LOC must sum to the repo total: {golden}"
    );
    assert_eq!(
        duplicated_sum, duplicated,
        "per-file duplicated LOC must sum to the repo total: {golden}"
    );
}

/// Lossless `u64 → f64` for the line counts in this corpus (all far
/// below `2^32`), mirroring the renderer's own clamp-then-widen order.
fn loc_as_f64(value: u64) -> f64 {
    f64::from(u32::try_from(value).expect("corpus LOC fits in u32"))
}
