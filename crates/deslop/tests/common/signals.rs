//! Final-contract assertion vocabulary shared by the accuracy suites
//! ([PIPELINE-CLUSTER-CLOSURE], [RANK-MASS-SUM]).
//!
//! Cluster-level signals, buckets, and evidence verdicts were removed
//! from the report wire by the mass-only cutover: admission evidence is
//! pair-scoped, and cluster surfaces carry cluster facts and mass only.
//! The honest shape-only/act-now bucket sets are gone with the routing
//! table; suites that still read a cluster signal fail fast on
//! [`super::signal`]'s quarantine panic until migrated to
//! admission/visibility + mass/membership.

use std::{collections::BTreeSet, path::Path, sync::Arc};

use anyhow::anyhow;
use deslop_core::{
    embedding::{test_support::StubProvider, EmbeddingMode},
    live::AnalysisSession,
    report::{PairComparison, PairComparisonParams, PairEndpoint},
};
use serde_json::Value;

use super::{
    cluster_file_set, cluster_id, clusters, field, occurrence_is_hidden, occurrence_texts,
    occurrences, Result,
};

/// Recomputes pair evidence for two explicitly supplied report occurrences.
pub(crate) fn compare_pair(
    scan_root: &Path,
    min_nodes: u32,
    left: &Value,
    right: &Value,
) -> Result<PairComparison> {
    compare_endpoints_with_mode(
        scan_root,
        min_nodes,
        pair_endpoint(left)?,
        pair_endpoint(right)?,
        EmbeddingMode::Off,
    )
}

/// Explicit pair comparison with the deterministic test embedding provider.
pub(crate) fn compare_pair_with_embeddings(
    scan_root: &Path,
    min_nodes: u32,
    left: &Value,
    right: &Value,
) -> Result<PairComparison> {
    compare_endpoints_with_mode(
        scan_root,
        min_nodes,
        pair_endpoint(left)?,
        pair_endpoint(right)?,
        EmbeddingMode::Required,
    )
}

/// Compares two exact endpoints without deriving either endpoint from a cluster.
pub(crate) fn compare_endpoints(
    scan_root: &Path,
    min_nodes: u32,
    left: PairEndpoint,
    right: PairEndpoint,
) -> Result<PairComparison> {
    compare_endpoints_with_mode(scan_root, min_nodes, left, right, EmbeddingMode::Off)
}

/// Returns the occurrence for the caller's explicit file choice.
pub(crate) fn occurrence_for_file<'a>(cluster: &'a Value, file: &str) -> Result<&'a Value> {
    occurrences(cluster)
        .iter()
        .find(|occurrence| {
            field(occurrence, "path")
                .as_str()
                .is_some_and(|path| path.ends_with(file))
        })
        .ok_or_else(|| anyhow!("cluster has no occurrence for {file}: {cluster:#}"))
}

/// Builds one analysis generation and compares only the requested endpoints.
fn compare_endpoints_with_mode(
    scan_root: &Path,
    min_nodes: u32,
    left: PairEndpoint,
    right: PairEndpoint,
    mode: EmbeddingMode,
) -> Result<PairComparison> {
    let provider = Arc::new(StubProvider::new());
    let session = AnalysisSession::new_with_mode(
        scan_root.to_path_buf(),
        min_nodes,
        false,
        None,
        provider,
        mode,
    )?;
    Ok(session.compare_pair(&PairComparisonParams { left, right })?)
}

/// Converts one report occurrence into an exact pair endpoint.
fn pair_endpoint(occurrence: &Value) -> Result<PairEndpoint> {
    let path = occurrence
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("occurrence has no path: {occurrence:#}"))?;
    Ok(PairEndpoint {
        path: path.into(),
        start_byte: endpoint_offset(occurrence, "start_byte")?,
        end_byte: endpoint_offset(occurrence, "end_byte")?,
    })
}

/// Reads one required byte offset from an occurrence.
fn endpoint_offset(occurrence: &Value, field_name: &str) -> Result<usize> {
    occurrence
        .get(field_name)
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("occurrence has no numeric {field_name}: {occurrence:#}"))
        .and_then(|value| usize::try_from(value).map_err(Into::into))
}

/// Asserts an exact pair metric without float-equality lint suppression.
pub(crate) fn assert_pair_metric(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= f64::EPSILON,
        "{label}: expected {expected}, got {actual}"
    );
}

/// Asserts the admission + visibility + mass contract the wire still
/// exposes for a shape-only fixture ([RANK-MASS-SUM],
/// [PIPELINE-CLUSTER-CLOSURE]).
///
/// The former `structural_only` bucket contract died with the routing
/// table: clusters no longer carry a bucket, a category, or any
/// cluster-level signal. What a rendered cluster can still prove about
/// a shape-only fixture is that it was **admitted** (it is on the
/// report), that every occurrence is **visible**, and that its **mass**
/// is the wire formula `canonical_node_count × (occurrence_count − 1)`
/// with `occurrence_count` equal to the visible membership. A suite
/// that needs a stronger shape-only guarantee must assert it against
/// the pair wire (explicit `PairComparison`), not against the cluster.
pub(crate) fn assert_structural_only_contract(cluster: &Value, label: &str) {
    let id = cluster_id(cluster);
    let canonical_nodes = field(cluster, "canonical_node_count").as_u64().unwrap_or(0);
    let occurrence_count = field(cluster, "occurrence_count").as_u64().unwrap_or(0);
    let mass = field(cluster, "mass").as_u64().unwrap_or(0);
    assert!(
        canonical_nodes > 0 && occurrence_count >= 2,
        "{label}: an admitted cluster must carry canonical_node_count and \
         occurrence_count — {id} reports nodes={canonical_nodes} count={occurrence_count}: {cluster:#}"
    );
    assert_eq!(
        mass,
        canonical_nodes.saturating_mul(occurrence_count.saturating_sub(1)),
        "{label}: mass must be the wire formula canonical_node_count × \
         (occurrence_count − 1) — {id} reports mass={mass} nodes={canonical_nodes} \
         count={occurrence_count}: {cluster:#}"
    );
    assert!(
        !occurrences(cluster).iter().any(occurrence_is_hidden),
        "{label}: a reported cluster must not hide an occurrence behind \
         report_hide — {id}: {cluster:#}"
    );
}

/// One-line dump of everything an accuracy assertion needs to explain
/// itself, so a failure names the actual numbers instead of restating
/// the expectation. Wire fields only: cluster facts + mass
/// ([PIPELINE-CLUSTER-CLOSURE]).
pub(crate) fn signal_dump(cluster: &Value) -> String {
    format!(
        "id={id} rank={rank} mass={mass} nodes={nodes} count={count} total={total} \
         files={files:?}",
        id = cluster_id(cluster),
        rank = field(cluster, "rank").as_u64().unwrap_or(0),
        mass = field(cluster, "mass").as_u64().unwrap_or(0),
        nodes = field(cluster, "canonical_node_count").as_u64().unwrap_or(0),
        count = field(cluster, "occurrence_count").as_u64().unwrap_or(0),
        total = field(cluster, "occurrences_total").as_u64().unwrap_or(0),
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
/// — the proof [FUSED-CONTENT-GATE]'s verbatim guard relies on when it
/// vouches full agreement for a cluster that is not wholly identical.
pub(crate) fn has_verbatim_pair(scan_root: &Path, cluster: &Value) -> Result<bool> {
    let texts = occurrence_texts(scan_root, cluster)?;
    Ok(distinct_texts(scan_root, cluster)?.len() < texts.len())
}

/// Asserts the full **proven-rename** contract ([FUSED-CONTENT-GATE],
/// [TECH-PMATCH-BAKER], `[REPAIR-RENAME-ANCHOR-MASS]`) — the mirror of
/// [`assert_structural_only_contract`], and the reason both live here.
///
/// The two contracts described the same signal triple: a maximal Type-2
/// rename and an anchor-poor scaffolding family both rendered
/// `structural = 1.00, token_jaccard = 1.00`, and only the measured
/// content evidence separated them. That evidence, the routing bucket,
/// and the verdict sentence were cluster-surface facts; the mass-only
/// wire removed all of them ([PIPELINE-CLUSTER-CLOSURE]). What a
/// rendered cluster can still prove about a renamed fixture is that it
/// was **admitted** (reported at all — a demoted or dropped rename is
/// a false negative), that every occurrence is **visible**, that its
/// **mass** is the wire formula, and that **no pair-only surface** has
/// crept back onto the cluster to mislabel it. The byte-level half
/// ([`assert_rename_is_not_a_copy`]) survives unchanged: every
/// occurrence must differ in raw bytes, or the fixture proves nothing
/// about renames.
pub(crate) fn assert_proven_rename_contract(
    scan_root: &Path,
    cluster: &Value,
    label: &str,
) -> Result<()> {
    assert_admission_and_clean_surface(cluster, label);
    assert_rename_is_not_a_copy(scan_root, cluster, label)
}

/// The proven-rename contract for a fixture that is a rename **plus an
/// inserted statement**. Verdict and not-a-copy are identical; only the
/// shape half differs, because the reported view spans the whole
/// declaration and therefore includes the insertion
/// ([FUSED-SHARED-SUBTREE], gh #408). Demanding Merkle exactness there
/// demands the fragment view — the shared sub-range either side of the
/// insertion — which is the recall hole #408 is filed against. The
/// wire-visible contract is the same as [`assert_proven_rename_contract`].
pub(crate) fn assert_near_miss_rename_contract(
    scan_root: &Path,
    cluster: &Value,
    label: &str,
) -> Result<()> {
    assert_admission_and_clean_surface(cluster, label);
    assert_rename_is_not_a_copy(scan_root, cluster, label)
}

/// The admission + visibility + mass + no-pair-surface contract every
/// fixture-level suite can still assert on a rendered cluster
/// ([PIPELINE-CLUSTER-CLOSURE], [RANK-MASS-SUM]). Positive, readable
/// numbers first; the negative pin closes the mislabelling path the
/// routing table used to be able to fabricate.
fn assert_admission_and_clean_surface(cluster: &Value, label: &str) {
    assert_rename_verdict(cluster, label);
    assert_no_pair_surface_on_cluster(cluster, label);
}

/// Asserts a cluster carries none of the pair-only or presentation
/// fields the mass-only wire forbids. A cluster that grows a `signals`
/// block, a bucket, an evidence verdict, a weight, or a category is the
/// mislabelling/fabrication path reopening — the exact failure the old
/// bucket and verdict assertions existed to catch.
pub(crate) fn assert_no_pair_surface_on_cluster(cluster: &Value, label: &str) {
    for field_name in [
        "signals",
        "signal_source",
        "content",
        "evidence_verdict",
        "bucket",
        "category",
        "classification",
        "weight",
        "size",
        "summary",
        "interpretation",
        "language",
    ] {
        assert!(
            cluster.get(field_name).is_none(),
            "{label}: the mass-only wire forbids {field_name} on a cluster — a \
             routing table could fabricate a value through it again: {cluster:#}"
        );
    }
}

/// Verdict half of the rename contracts. On the mass-only wire the
/// cluster carries no bucket and no evidence verdict, so the honest
/// assertion is the admission + visibility + mass contract
/// ([PIPELINE-CLUSTER-CLOSURE], [RANK-MASS-SUM]) plus the byte-level
/// not-a-copy checks in [`assert_rename_is_not_a_copy`]. The old
/// `nearly_identical` verdict and content-gate routing were cluster-
/// surface facts; what a rendered cluster can still prove about a
/// fixture is that it was admitted with a consistent, visible
/// membership.
fn assert_rename_verdict(cluster: &Value, label: &str) {
    let dump = signal_dump(cluster);
    let canonical_nodes = field(cluster, "canonical_node_count").as_u64().unwrap_or(0);
    let occurrence_count = field(cluster, "occurrence_count").as_u64().unwrap_or(0);
    let mass = field(cluster, "mass").as_u64().unwrap_or(0);
    assert!(
        canonical_nodes > 0 && occurrence_count >= 2,
        "{label}: an admitted cluster must carry canonical_node_count and \
         occurrence_count — {dump}"
    );
    assert_eq!(
        mass,
        canonical_nodes.saturating_mul(occurrence_count.saturating_sub(1)),
        "{label}: mass must be canonical_node_count × (occurrence_count − 1) \
         — {dump}"
    );
}

/// Occurrence half: every occurrence must differ in raw bytes, or the
/// promotion proves nothing about renames, and none may be hidden — a
/// clone the report will not show is a false negative whatever its
/// bucket says.
fn assert_rename_is_not_a_copy(scan_root: &Path, cluster: &Value, label: &str) -> Result<()> {
    let dump = signal_dump(cluster);
    assert_eq!(
        distinct_texts(scan_root, cluster)?.len(),
        occurrences(cluster).len(),
        "{label}: every occurrence must differ in raw bytes, or this is a \
         Type-1 copy proving nothing about rename evidence — {dump}"
    );
    assert!(
        !occurrences(cluster).iter().any(occurrence_is_hidden),
        "{label}: a proven Type-2 clone may not have a hidden occurrence \
         — {dump}"
    );
    Ok(())
}
