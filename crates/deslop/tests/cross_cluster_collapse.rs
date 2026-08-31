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

use crate::common::signals::{assert_no_pair_surface_on_cluster, assert_structural_only_contract};
use crate::common::*;

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

/// Finds the exact two-member shared block or reports its disappearance.
fn expect_shared_logic<'a>(
    report: &'a serde_json::Value,
    root: &Path,
) -> Result<&'a serde_json::Value> {
    for cluster in clusters(report) {
        let texts = occurrence_texts(root, cluster)?;
        if texts.len() == 2 && texts.iter().all(|text| text == SHARED_LOGIC) {
            return Ok(cluster);
        }
    }
    Err(anyhow::anyhow!(
        "content-proven nested clone disappeared during cross-cluster subsumption: {report:#}"
    ))
}

/// Pins the evidence and verdict of the clone that subsumption must preserve.
fn assert_content_proven(cluster: &serde_json::Value) {
    assert_eq!(
        cluster_size(cluster),
        2,
        "the clone must span exactly two files"
    );
    assert_structural_only_contract(cluster, "cross-cluster collapse");
    assert_no_pair_surface_on_cluster(cluster, "cross-cluster collapse");
    assert_eq!(
        field(cluster, "signal_source"),
        &serde_json::json!({"left": 0, "right": 1}),
        "the rendered evidence must name the admitted pair it measured: {cluster:#}"
    );
}

/// A low-content enclosing shape must not delete a byte-proven inner clone
/// before [FUSED-CONTENT-GATE] can measure either view.
/// [REPAIR-SUBSUME-CONTENT-FIRST]
#[test]
fn content_proven_nested_clone_survives_content_poor_enclosing_view() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("corpus");
    write_content_subsumption_fixture(&scan_root)?;
    let report = run_report(tmp.path(), &scan_root)?;
    assert_content_proven(expect_shared_logic(&report, &scan_root)?);
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
/// and L17-21, 158 bytes each, byte-for-byte equal. Enclosing it is a
/// mis-scoped near-miss pairing the *whole* `ApplyStandard` body (L4-12,
/// 235 bytes) against only the prefix of `ApplyPremium` (L16-21, 189
/// bytes): two occurrences that are not the same code, routed
/// `structural_only` at `structural` 0.85 and demoted by the renderer.
///
/// The demoted encloser must not delete the byte-identical clone. Both
/// exceptions that let a nested view overturn its encloser require
/// `spans_multiple_files`, so a byte-proven clone confined to one file
/// has no route to survive: the report loses a Type-1 duplicate and
/// claims nine lines of `ApplyStandard` are duplicated where five are.
#[test]
fn byte_identical_clone_survives_a_demoted_enclosing_view_in_one_file() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = fixture("csharp-merge-readafter");
    let report = default_report(tmp.path(), &scan_root)?;
    let candidates = clusters_for_file(&report, "Prefix.cs");
    assert!(
        !candidates.is_empty(),
        "the fixture must report the duplicated prefix at all: {report:#}"
    );

    let mut byte_identical = Vec::new();
    for cluster in &candidates {
        let texts = occurrence_texts(&scan_root, cluster)?;
        if let [first, rest @ ..] = texts.as_slice() {
            if !rest.is_empty() && rest.iter().all(|text| text == first) {
                byte_identical.push(cluster.clone());
            }
        }
    }

    let rendered = rendered_clusters(&report, "Prefix.cs");
    assert_eq!(
        byte_identical.len(),
        1,
        "exactly one published cluster must be the byte-identical \
         five-statement run duplicated between ApplyStandard and \
         ApplyPremium; a demoted `structural_only` view enclosing it must \
         not delete it. Published: {rendered:#?}"
    );

    let clone = byte_identical
        .first()
        .ok_or_else(|| anyhow::anyhow!("length asserted to be exactly one above"))?;
    let occurrences = cluster_occurrences(clone);
    assert_eq!(
        occurrences.len(),
        2,
        "the byte-identical run occurs exactly twice: {clone:#}"
    );
    for occurrence in &occurrences {
        assert_eq!(
            occurrence.end.saturating_sub(occurrence.start),
            158,
            "each occurrence is the same 158-byte run; a differently-sized \
             occurrence means a mis-scoped view was elected: {clone:#}"
        );
    }
    assert_no_pair_surface_on_cluster(clone, "cross-cluster collapse");
    Ok(())
}
