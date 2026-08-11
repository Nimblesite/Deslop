//! Fused-signal assertion vocabulary shared by the golden confidence
//! suites ([FUSION-STRATEGY-MAX-SUM], [FUSION-CONTENT-GATE],
//! [FUSED-THRESHOLD]).
//!
//! The bands, the honest shape-only bucket set, and the diagnostic dump
//! are the same contract in every suite that asserts on `signals.fused`,
//! so they live here rather than being restated per binary.

use std::{collections::BTreeSet, path::Path};

use anyhow::anyhow;
use serde_json::Value;

use super::{
    cluster_bucket, cluster_file_set, cluster_size, clusters, field, occurrence_texts, signal,
    Result,
};

/// The agent-facing act-now line ([FUSED-THRESHOLD]): at or above this a
/// `find-similar` consumer must refuse to write the copy, so nothing but
/// real duplication may reach it.
pub(crate) const ACT_NOW_FUSED: f64 = 0.85;

/// The agent-facing reuse-bias line (`docs/snippets/agents-md-recipe.md`):
/// below this the recipe tells the agent to author the code outright, so
/// a genuine clone that lands here is a false negative at the agent
/// surface even when the report still lists it.
pub(crate) const REUSE_FUSED: f64 = 0.6;

/// Buckets that describe a shape-only match without claiming the
/// *content* is duplicated ([RANK-STRUCTURAL-ONLY]).
pub(crate) const HONEST_SHAPE_ONLY_BUCKETS: [&str; 2] = ["structural_only", "loosely_similar"];

/// Buckets a cluster may carry once it has reached the act-now line.
pub(crate) const ACT_NOW_BUCKETS: [&str; 3] = ["identical", "nearly_identical", "same_behavior"];

/// One-line dump of everything an accuracy assertion needs to explain
/// itself, so a failure names the actual numbers instead of restating
/// the expectation.
pub(crate) fn signal_dump(cluster: &Value) -> String {
    format!(
        "id={id} bucket={bucket} category={category} size={size} weight={weight:.3} \
         structural={structural:.4} token_jaccard={token:.4} embedding_cos={embedding:.4} \
         fused={fused:.4} files={files:?}",
        id = field(cluster, "id").as_str().unwrap_or("?"),
        bucket = cluster_bucket(cluster),
        category = field(cluster, "category").as_str().unwrap_or("?"),
        size = cluster_size(cluster),
        weight = field(cluster, "weight").as_f64().unwrap_or_default(),
        structural = signal(cluster, "structural"),
        token = signal(cluster, "token_jaccard"),
        embedding = signal(cluster, "embedding_cos"),
        fused = signal(cluster, "fused"),
        files = cluster_file_set(cluster),
    )
}

/// Zero-based rank of `cluster` inside the ranked report, resolved by
/// identity so two clusters with equal weights can never be confused.
pub(crate) fn rank_of(report: &Value, cluster: &Value) -> Result<usize> {
    clusters(report)
        .iter()
        .position(|candidate| std::ptr::eq(candidate, cluster))
        .ok_or_else(|| anyhow!("cluster is missing from the ranked report: {report:#}"))
}

/// The distinct source slices a cluster's occurrences point at. A
/// single-element set proves byte-identical duplication; anything larger
/// proves the raw content differs.
pub(crate) fn distinct_texts(scan_root: &Path, cluster: &Value) -> Result<BTreeSet<String>> {
    Ok(occurrence_texts(scan_root, cluster)?.into_iter().collect())
}

/// True when at least two of a cluster's occurrences are byte-identical
/// — the proof [FUSION-CONTENT-GATE]'s verbatim guard relies on when it
/// vouches full agreement for a cluster that is not wholly identical.
pub(crate) fn has_verbatim_pair(scan_root: &Path, cluster: &Value) -> Result<bool> {
    let texts = occurrence_texts(scan_root, cluster)?;
    Ok(distinct_texts(scan_root, cluster)?.len() < texts.len())
}
