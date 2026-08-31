//! Independent correctness checks for the committed report golden.

use std::fs;

use anyhow::{anyhow, Context as _};
use serde_json::Value;

use super::{corpus_dir, golden_report_path, BLESS, GOLDEN_MIN_NODES, PAIR_FILES, TRIO_FILES};
use crate::common::{golden::*, signals::assert_no_pair_surface_on_cluster, verdict::*, *};

// [PIPELINE-DETERMINISM] Half two: correct. Byte equality alone only
// proves the tool still agrees with a file the tool wrote; these
// invariants come from the authored corpus instead, so a wrongly-blessed
// golden fails here even when it matches the committed bytes exactly.
#[test]
fn committed_golden_satisfies_report_contract() -> Result<()> {
    let golden = load_golden(&golden_report_path(), BLESS)?;
    assert_cold_scan_header(&golden);
    assert_occurrences_are_real_type1_clones(&golden)?;
    assert_ranking_and_cluster_shape(&golden)?;
    assert_engine_derived_fields(&golden);
    assert_metrics_arithmetic(&golden)?;
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
            .ok_or_else(|| anyhow!("validated_cluster_slices asserts at least two occurrences"))?;
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
    let total = u64::try_from(slices.len())?;
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
    let start_line = field(occurrence, "start_line")
        .as_u64()
        .with_context(|| format!("occurrence in {path} is missing start_line"))?;
    let end_line = field(occurrence, "end_line")
        .as_u64()
        .with_context(|| format!("occurrence in {path} is missing end_line"))?;
    assert!(
        start_line >= 1 && start_line <= end_line,
        "occurrence lines {start_line}..{end_line} in {path} must be one-based and ordered"
    );
    assert_eq!(
        field(occurrence, "hidden").as_bool(),
        Some(false),
        "no occurrence in this corpus is hidden: {occurrence}"
    );
    source
        .get(start..end)
        .map(<[u8]>::to_vec)
        .ok_or_else(|| anyhow!("occurrence range [{start}..{end}) fell outside {path}"))
}

/// The two authored clusters and the ranking between them: the 3-copy
/// 72-node `settle_invoice` cluster must outrank the 2-copy 44-node
/// `merge_labels` cluster, masses must be positive and non-increasing
/// down the report, and every cluster must clear `--min-nodes`.
fn assert_ranking_and_cluster_shape(golden: &Value) -> Result<()> {
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
            .with_context(|| format!("cluster is missing canonical_node_count: {cluster}"))?;
        assert!(
            node_count >= GOLDEN_MIN_NODES,
            "cluster below --min-nodes {GOLDEN_MIN_NODES}: {cluster}"
        );
    }
    let trio_rank = rank_of_exact_file_set(golden, &TRIO_FILES)?;
    let pair_rank = rank_of_exact_file_set(golden, &PAIR_FILES)?;
    assert!(
        trio_rank < pair_rank,
        "the 3-copy cluster must outrank the 2-copy cluster: {golden}"
    );
    assert_authored_cluster(cluster_list, trio_rank, 3)?;
    assert_authored_cluster(cluster_list, pair_rank, 2)?;
    Ok(())
}

/// Masses down the report: strictly positive and non-increasing — the
/// mass-only ranking contract ([RANK-MASS-SUM]). `weight` was removed
/// with the bucket surface; mass is the wire figure consumers rank by.
fn assert_weights_ranked(cluster_list: &[Value], golden: &Value) {
    let masses: Vec<u64> = cluster_list
        .iter()
        .map(|cluster| field(cluster, "mass").as_u64().unwrap_or(0))
        .collect();
    assert!(
        masses.iter().all(|mass| *mass > 0),
        "every cluster mass must be positive: {masses:?} in {golden}"
    );
    assert!(
        masses
            .windows(2)
            .all(|pair| matches!(pair, [left, right] if left >= right)),
        "clusters must be ranked mass-non-increasing: {masses:?} in {golden}"
    );
}

/// One authored Type-1 cluster: exact occurrence count, byte-identical
/// occurrence slices (proven in [`assert_occurrences_are_real_type1_clones`])
/// and a clean cluster surface ([PIPELINE-CLUSTER-CLOSURE]) — the
/// mass-only wire carries cluster facts, never the deleted bucket or
/// signal block.
fn assert_authored_cluster(cluster_list: &[Value], rank: usize, copies: u64) -> Result<()> {
    let cluster = cluster_list
        .get(rank)
        .ok_or_else(|| anyhow!("rank {rank} is outside the cluster list"))?;
    assert_eq!(
        cluster_size(cluster),
        copies,
        "authored cluster must have exactly {copies} occurrences: {cluster}"
    );
    assert_no_pair_surface_on_cluster(cluster, "report-golden");
    Ok(())
}

/// Every figure the report states about a cluster that a consumer would
/// otherwise have to derive: the worst-first rank and its severity band
/// ([SEVERITY-BAND]), the display occurrence count, and the byte-proven
/// clone fact with a clean cluster surface.
///
/// Every one of these is carried precisely so no client recomputes it,
/// so a report that omits one — or states one the rest of the report
/// contradicts — is a defect here, not in the client that renders it.
fn assert_engine_derived_fields(golden: &Value) {
    let cluster_list = clusters(golden);
    let total = cluster_list.len();
    for (index, cluster) in cluster_list.iter().enumerate() {
        let expected_rank = u64::try_from(index.saturating_add(1)).ok();
        assert_eq!(
            field(cluster, "rank").as_u64(),
            expected_rank,
            "clusters must carry their one-based worst-first rank: {cluster}"
        );
        assert_eq!(
            field(cluster, "occurrence_count").as_u64(),
            Some(cluster_size(cluster)),
            "the stated occurrence count must equal the occurrences the report carries: {cluster}"
        );
        assert!(
            cluster.pointer("/signals/fused").is_none()
                && cluster.get("meets_fused_gate").is_none(),
            "no cluster-level fused or fused-gate field may survive on the wire \
             ([FUSED-SCOPE]): {cluster}"
        );
        assert_no_pair_surface_on_cluster(cluster, "report-golden");
    }
    // Two clusters: the worse of them tops the percentile, the other
    // closes it ([SEVERITY-BAND]).
    assert_eq!(
        total, 2,
        "the corpus authors exactly two clusters: {golden}"
    );
    let bands: Vec<&str> = cluster_list
        .iter()
        .map(|cluster| field(cluster, "rank_band").as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        bands,
        vec!["worst", "faint"],
        "the engine bands the ranking; a client never re-derives it: {golden}"
    );
}

/// Zero-based rank of the cluster whose occurrences span exactly
/// `files`, failing with the full report when absent.
fn rank_of_exact_file_set(golden: &Value, files: &[&str]) -> Result<usize> {
    let expected: std::collections::BTreeSet<String> =
        files.iter().map(|name| (*name).to_owned()).collect();
    clusters(golden)
        .iter()
        .position(|cluster| cluster_file_set(cluster) == expected)
        .ok_or_else(|| anyhow!("no cluster spans exactly {files:?}: {golden}"))
}

/// Repo metrics must be transparent arithmetic over the cluster list:
/// duplicated LOC recomputed from the visible occurrence spans, the
/// percentage recomputed from the two LOC totals, per-file rows summing
/// to the repo totals, and the fixed-flag run carrying no threshold.
fn assert_metrics_arithmetic(golden: &Value) -> Result<()> {
    let analysed = metric_field(golden, "analysed_loc")
        .as_u64()
        .with_context(|| format!("metrics is missing analysed_loc: {golden}"))?;
    let duplicated = metric_field(golden, "duplicated_loc")
        .as_u64()
        .with_context(|| format!("metrics is missing duplicated_loc: {golden}"))?;
    assert!(
        analysed > 0,
        "five source files must analyse to LOC: {golden}"
    );
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
        .with_context(|| format!("metrics is missing duplication_percent: {golden}"))?;
    let recomputed = 100.0 * loc_as_f64(duplicated)? / loc_as_f64(analysed)?;
    assert!(
        (percent - recomputed).abs() <= 1e-6,
        "duplication_percent {percent} must equal 100*{duplicated}/{analysed} = {recomputed} within rendering precision"
    );
    assert_eq!(
        metric_field(golden, "duplicated_files").as_u64(),
        Some(5),
        "every corpus file carries a clone: {golden}"
    );
    assert_per_file_rows(golden, analysed, duplicated)?;
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
    Ok(())
}

/// Per-file metric rows: one per corpus file, each internally
/// consistent, and both LOC columns summing to the repo totals.
fn assert_per_file_rows(golden: &Value, analysed: u64, duplicated: u64) -> Result<()> {
    let rows = per_file_metrics(golden);
    assert_eq!(rows.len(), 5, "one metric row per corpus file: {golden}");
    let mut analysed_sum = 0_u64;
    let mut duplicated_sum = 0_u64;
    for row in rows {
        let path = field(row, "path")
            .as_str()
            .with_context(|| format!("per-file metric row is missing path: {row}"))?;
        assert!(
            corpus_dir().join(path).is_file(),
            "per-file metric row {path} must reference a corpus file"
        );
        let row_analysed = field(row, "analysed_loc")
            .as_u64()
            .with_context(|| format!("{path}: per-file row is missing analysed_loc"))?;
        let row_duplicated = field(row, "duplicated_loc")
            .as_u64()
            .with_context(|| format!("{path}: per-file row is missing duplicated_loc"))?;
        assert!(
            row_duplicated <= row_analysed,
            "{path}: per-file duplicated LOC exceeds its analysed LOC: {row}"
        );
        analysed_sum = analysed_sum.saturating_add(row_analysed);
        duplicated_sum = duplicated_sum.saturating_add(row_duplicated);
    }
    assert_eq!(
        analysed_sum, analysed,
        "per-file analysed LOC must sum to the repo total: {golden}"
    );
    assert_eq!(
        duplicated_sum, duplicated,
        "per-file duplicated LOC must sum to the repo total: {golden}"
    );
    Ok(())
}
