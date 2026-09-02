//! End-to-end regression coverage for issue #50: nested fingerprints
//! over the same physical code produce two distinct fused clusters
//! whose occurrence byte ranges fully overlap inside the same files.
//! `collapse_overlapping_per_file` deduplicates *within* a single
//! cluster; without a cross-cluster pass, the `[Fact] + method`
//! subtree and the bare `method` subtree (one line below the
//! attribute) survive as siblings and the user sees the same
//! dozen-occurrence clone twice with different cluster ids.
//!
//! Spec: [PIPELINE-CLUSTER-EXACT] commits to one canonical cluster
//! per duplicated region.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::common::signals::assert_no_pair_surface_on_cluster;
use crate::common::*;

const STANDARD_VIEW_BYTES: u64 = 190;
const PREMIUM_VIEW_BYTES: u64 = 189;
const ALPHA_WIDE_VIEW_BYTES: u64 = 493;
const BETA_WIDE_VIEW_BYTES: u64 = 532;

fn report_path(tmp: &Path) -> PathBuf {
    let mut path = tmp.join("report");
    let _replaced = path.set_extension("json");
    path
}

fn run_report(tmp: &Path, scan_root: &Path) -> Result<serde_json::Value> {
    report_with(tmp, scan_root, &["--min-nodes", "8", "--embeddings", "off"])
}

/// One CLI run with `extra_args`, parsed from the JSON report.
fn report_with(tmp: &Path, scan_root: &Path, extra_args: &[&str]) -> Result<serde_json::Value> {
    let mut cmd = deslop_cmd(scan_root, &tmp.join("report"))?;
    let _assertion = cmd.args(extra_args).assert().success();
    let body = fs::read_to_string(report_path(tmp))?;
    Ok(serde_json::from_str(&body)?)
}

#[derive(Clone, Debug)]
struct Occurrence {
    path: String,
    start: u64,
    end: u64,
}

fn cluster_occurrences(cluster: &serde_json::Value) -> Vec<Occurrence> {
    cluster
        .get("occurrences")
        .and_then(serde_json::Value::as_array)
        .map(|occurrences| {
            occurrences
                .iter()
                .filter_map(|occurrence| {
                    Some(Occurrence {
                        path: occurrence.get("path")?.as_str()?.to_owned(),
                        start: occurrence.get("start_byte")?.as_u64()?,
                        end: occurrence.get("end_byte")?.as_u64()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn ranges_overlap(left: &Occurrence, right: &Occurrence) -> bool {
    left.path == right.path && left.start < right.end && right.start < left.end
}

fn every_occurrence_overlaps_some(inner: &[Occurrence], outer: &[Occurrence]) -> bool {
    !inner.is_empty()
        && inner
            .iter()
            .all(|candidate| outer.iter().any(|other| ranges_overlap(candidate, other)))
}

fn first_subsumed_pair(report: &serde_json::Value) -> Option<String> {
    let clusters = report.get("clusters")?.as_array()?;
    let occurrence_sets: Vec<(String, Vec<Occurrence>)> = clusters
        .iter()
        .map(|cluster| (cluster_id(cluster).to_owned(), cluster_occurrences(cluster)))
        .collect();
    for (outer_index, (outer_id, outer)) in occurrence_sets.iter().enumerate() {
        for (inner_id, inner) in occurrence_sets.iter().skip(outer_index.saturating_add(1)) {
            if every_occurrence_overlaps_some(inner, outer)
                && every_occurrence_overlaps_some(outer, inner)
            {
                return Some(format!(
                    "clusters {outer_id} and {inner_id} cover the same physical \
                     bytes — every occurrence in one overlaps with some \
                     occurrence in the other"
                ));
            }
        }
    }
    None
}

fn clusters_for_file(report: &serde_json::Value, needle: &str) -> Vec<serde_json::Value> {
    report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .map(|clusters| {
            clusters
                .iter()
                .filter(|cluster| {
                    cluster_occurrences(cluster)
                        .iter()
                        .any(|occurrence| occurrence.path == needle)
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

const SHARED_LOGIC: &str = r"if (sharedGate()) {
        const sharedValue = 7;
        emitShared(sharedValue);
        persistShared(sharedValue);
        auditShared(sharedValue);
    }";

const ALPHA_SOURCE: &str = r"export function calculateAlpha(alphaSeed: number): number {
    const alphaOne = alphaSeed + 11;
    const alphaTwo = alphaOne * 13;
    const alphaThree = alphaTwo - 17;
    const alphaFour = alphaThree / 19;
    const alphaFive = alphaFour + 23;
    const alphaSix = alphaFive * 29;
    if (sharedGate()) {
        const sharedValue = 7;
        emitShared(sharedValue);
        persistShared(sharedValue);
        auditShared(sharedValue);
    }
    const alphaSeven = alphaSix - 31;
    const alphaEight = alphaSeven + 37;
    return alphaEight;
}
";

const BETA_SOURCE: &str = r"export function calculateBeta(betaSeed: number): number {
    const betaOne = betaSeed + 41;
    const betaTwo = betaOne * 43;
    const betaThree = betaTwo - 47;
    const betaFour = betaThree / 53;
    const betaFive = betaFour + 59;
    const betaSix = betaFive * 61;
    if (sharedGate()) {
        const sharedValue = 7;
        emitShared(sharedValue);
        persistShared(sharedValue);
        auditShared(sharedValue);
    }
    const betaSeven = betaSix - 67;
    const betaEight = betaSeven + 71;
    return betaEight;
}
";

/// Writes two content-divergent wrappers around one byte-identical clone.
fn write_content_subsumption_fixture(root: &Path) -> Result<()> {
    fs::create_dir_all(root)?;
    fs::write(root.join("alpha.ts"), ALPHA_SOURCE)?;
    fs::write(root.join("beta.ts"), BETA_SOURCE)?;
    Ok(())
}

/// [PIPELINE-CLUSTER-SUBSUME] selects the enclosing cross-file view by
/// physical enclosure and duplicated mass; it never promotes pair evidence
/// or preserves a nested view merely because that view is byte-identical.
#[test]
fn wider_cross_file_view_survives_subsumption() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("corpus");
    write_content_subsumption_fixture(&scan_root)?;
    let report = run_report(tmp.path(), &scan_root)?;
    let candidates = clusters(&report);
    assert_eq!(
        candidates.len(),
        1,
        "one cross-file duplication must retain one canonical view: {report:#}"
    );
    let clone = candidates
        .first()
        .ok_or_else(|| anyhow::anyhow!("candidate count asserted to be one above"))?;
    let occurrences = cluster_occurrences(clone);
    assert_eq!(cluster_size(clone), 2, "the clone must span both files");
    assert_eq!(occurrences.len(), 2, "both visible occurrences must render");
    let paths: Vec<&str> = occurrences
        .iter()
        .map(|occurrence| occurrence.path.as_str())
        .collect();
    assert_eq!(
        paths,
        vec!["alpha.ts", "beta.ts"],
        "the canonical view must preserve file coverage"
    );
    let spans: Vec<u64> = occurrences
        .iter()
        .map(|occurrence| occurrence.end.saturating_sub(occurrence.start))
        .collect();
    assert_eq!(spans, vec![ALPHA_WIDE_VIEW_BYTES, BETA_WIDE_VIEW_BYTES]);
    let texts = occurrence_texts(&scan_root, clone)?;
    assert_eq!(
        texts.len(),
        2,
        "both occurrence ranges must resolve to source bytes"
    );
    assert!(
        texts.iter().all(|text| text.len() > SHARED_LOGIC.len()),
        "the wider cross-file view must enclose the nested shared block"
    );
    assert_ne!(
        texts.first(),
        texts.last(),
        "the nested exact block must not become the selected view"
    );
    assert_no_pair_surface_on_cluster(clone, "cross-cluster collapse");
    Ok(())
}

// Issue #50 acceptance: a small C# file with two [Fact]-decorated
// near-identical test methods must produce exactly one cluster covering
// the test-method region. Pre-fix, the `attribute_list +
// method_declaration` subtree and the bare `method_declaration`
// subtree each form a separate fused cluster, so the user sees the
// same occurrences reported twice. The fixture is a two-method pair so
// the cluster stays visible: a three-or-more sibling-method family is a
// single-file `structural_only` pattern suppressed by #197.
#[test]
fn fact_decorated_identical_methods_produce_one_cluster() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = fixture("csharp-fact-cross-cluster");
    let report = run_report(tmp.path(), &scan_root)?;
    let candidates = clusters_for_file(&report, "CodeLookupTests.cs");
    assert!(
        !candidates.is_empty(),
        "fixture must produce at least one clone cluster covering the test \
         methods: {report:#}"
    );
    assert!(
        candidates.len() <= 3,
        "nested-fingerprint clusters must collapse: expected at most 3 clusters \
         (method body, attribute, possible sibling window) covering the test \
         methods, got {} (was 25 before the fix): ids = {:?}",
        candidates.len(),
        candidates.iter().map(cluster_id).collect::<Vec<_>>(),
    );
    Ok(())
}

// Issue #50 invariant: no two clusters may have mutually-subsuming
// occurrence sets. If every occurrence in cluster B overlaps some
// occurrence in cluster A *and* vice versa, they describe the same
// physical bytes at different AST depths and must collapse to one.
#[test]
fn no_two_clusters_cover_the_same_physical_bytes() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = fixture("csharp-fact-cross-cluster");
    let report = run_report(tmp.path(), &scan_root)?;
    assert!(
        first_subsumed_pair(&report).is_none(),
        "cross-cluster overlap collapse missing: {}",
        first_subsumed_pair(&report).unwrap_or_default()
    );
    Ok(())
}

/// The default-settings report for a fixture directory, with embeddings
/// off so the assertion turns on deterministic signals only.
fn default_report(tmp: &Path, scan_root: &Path) -> Result<serde_json::Value> {
    report_with(tmp, scan_root, &["--embeddings", "off"])
}

/// One line per published cluster covering `needle`, for failure output.
fn rendered_clusters(report: &serde_json::Value, needle: &str) -> Vec<String> {
    clusters_for_file(report, needle)
        .iter()
        .map(|cluster| {
            let spans: Vec<String> = cluster_occurrences(cluster)
                .iter()
                .map(|occurrence| format!("{}..{}", occurrence.start, occurrence.end))
                .collect();
            format!("{} {}", cluster_id(cluster), spans.join(","))
        })
        .collect()
}

/// [REPAIR-SUBSUME-CONTENT-FIRST] / [PIPELINE-CLUSTER-SUBSUME]: the
/// single-file half of the contract
/// `content_proven_nested_clone_survives_content_poor_enclosing_view`
/// holds across files.
///
/// `csharp-merge-readafter` holds one byte-identical five-statement run
/// duplicated between two methods of the same class — `Prefix.cs` L6-10
/// and L17-21, 158 bytes each, byte-for-byte equal. The larger authored view
/// starts at the local `label` declaration and ends after `Publish`: it is
/// 190 bytes in both methods, consistently renamed only at the label literal.
/// [PIPELINE-CLUSTER-EXACT-SCOPE] selects that wider view before pair
/// admission; a nested exact fingerprint must not override the authored scope.
#[test]
fn widest_same_declaration_view_is_the_published_finding() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = fixture("csharp-merge-readafter");
    let report = default_report(tmp.path(), &scan_root)?;
    let candidates = clusters_for_file(&report, "Prefix.cs");
    assert!(
        !candidates.is_empty(),
        "the fixture must report the duplicated prefix at all: {report:#}"
    );

    let rendered = rendered_clusters(&report, "Prefix.cs");
    assert_eq!(
        candidates.len(),
        1,
        "one physical duplication must publish one canonical view: {rendered:#?}"
    );
    let clone = candidates
        .first()
        .ok_or_else(|| anyhow::anyhow!("candidate count asserted to be one above"))?;
    let occurrences = cluster_occurrences(clone);
    assert_eq!(
        occurrences.len(),
        2,
        "the canonical same-file view must retain both method occurrences: {clone:#}"
    );
    let texts = occurrence_texts(&scan_root, clone)?;
    assert_eq!(
        texts.len(),
        2,
        "each occurrence must resolve to source bytes"
    );
    assert_ne!(
        texts.first(),
        texts.last(),
        "the smaller byte-identical fingerprint must not displace the wider authored view"
    );
    let spans: Vec<u64> = occurrences
        .iter()
        .map(|occurrence| occurrence.end.saturating_sub(occurrence.start))
        .collect();
    assert_eq!(
        spans,
        vec![STANDARD_VIEW_BYTES, PREMIUM_VIEW_BYTES],
        "the two wider authored ranges differ only by their literal byte length: {clone:#}"
    );
    assert_no_pair_surface_on_cluster(clone, "cross-cluster collapse");
    Ok(())
}
