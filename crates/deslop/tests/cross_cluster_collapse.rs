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
    collections::BTreeSet,
    fs,
    ops::RangeInclusive,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::common::signals::assert_no_pair_surface_on_cluster;
use crate::common::*;

/// The bytes of the wider authored view inside `ApplyStandard`.
const STANDARD_VIEW_BYTES: u64 = 190;
/// The same view inside `ApplyPremium`, one literal shorter.
const PREMIUM_VIEW_BYTES: u64 = 189;
/// The 1-based lines that view covers in each method.
const STANDARD_VIEW_LINES: RangeInclusive<u64> = 5..=10;
/// The same view's lines in the premium copy.
const PREMIUM_VIEW_LINES: RangeInclusive<u64> = 16..=21;
/// The statement run both methods carry byte for byte after their label.
const SHARED_PREFIX_RUN: &str = "policy.Stage(ticket);\n        policy.Validate(ticket);\n        policy.Record(ticket);\n        policy.Publish(ticket);";
/// The 1-based lines `SHARED_LOGIC` occupies in both wrappers.
const SHARED_LOGIC_LINES: RangeInclusive<u64> = 8..=13;

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

/// [PIPELINE-CLUSTER-SUBSUME-STRADDLE] The two wrappers agree on nothing
/// but the block they share: every other statement keeps its shape and
/// changes its names and numbers, so the whole functions fail the content
/// floor ([FUSED-CONTENT-GATE]) and the block plus one neighbouring
/// statement clears it — on either side. Those two padded windows straddle
/// the block; neither may be published, and the byte-identical block is
/// the one finding, at its own extent, in both files.
#[test]
fn padded_windows_straddling_a_verbatim_block_publish_the_block() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("corpus");
    write_content_subsumption_fixture(&scan_root)?;
    let report = run_report(tmp.path(), &scan_root)?;
    let candidates = clusters(&report);
    assert_eq!(
        candidates.len(),
        1,
        "one shared block must be published once, not once per padded window: {report:#}"
    );
    let clone = candidates
        .first()
        .ok_or_else(|| anyhow::anyhow!("candidate count asserted to be one above"))?;
    let occurrences = cluster_occurrences(clone);
    assert_eq!(cluster_size(clone), 2, "the block must span both files");
    assert_eq!(occurrences.len(), 2, "both visible occurrences must render");
    let paths: Vec<&str> = occurrences
        .iter()
        .map(|occurrence| occurrence.path.as_str())
        .collect();
    assert_eq!(
        paths,
        vec!["alpha.ts", "beta.ts"],
        "the finding must preserve file coverage"
    );
    let block_bytes = u64::try_from(SHARED_LOGIC.len())?;
    let spans: Vec<u64> = occurrences
        .iter()
        .map(|occurrence| occurrence.end.saturating_sub(occurrence.start))
        .collect();
    assert_eq!(
        spans,
        vec![block_bytes, block_bytes],
        "each occurrence is the block and nothing around it: {clone:#}"
    );
    let block_lines: BTreeSet<u64> = SHARED_LOGIC_LINES.collect();
    let published = visible_duplicated_lines(&report);
    for file in ["alpha.ts", "beta.ts"] {
        assert_eq!(
            published.get(file),
            Some(&block_lines),
            "{file} must publish the block's lines alone: {report:#}"
        );
    }
    assert_eq!(
        occurrence_texts(&scan_root, clone)?,
        vec![SHARED_LOGIC.to_owned(), SHARED_LOGIC.to_owned()],
        "the finding is the shared block, byte for byte, in both files"
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

/// [PIPELINE-CLUSTER-EXACT-SCOPE] / [PIPELINE-CLUSTER-SUBSUME]: one
/// physical duplication publishes one canonical view.
///
/// `csharp-merge-readafter` holds `ApplyStandard` (L3-12) and
/// `ApplyPremium` (L14-26) in one class. Both open with a label
/// declaration and five byte-identical statements. The larger authored
/// view runs from that declaration to `Publish` — 190 bytes and 189,
/// consistently renamed only at the label literal — and is selected
/// before pair admission, so the exact fingerprint nested inside it must
/// not displace it. The methods themselves cluster in neither this
/// version nor 0.32.0: the rescue that would admit them is cross-file
/// only (gh #492).
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
    let views = cluster_occurrences(clone);
    assert_eq!(
        views.len(),
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
        "the premium method grew an archive branch, so the two methods stay byte-distinct"
    );
    for text in &texts {
        assert!(
            text.contains(SHARED_PREFIX_RUN),
            "each method carries the byte-identical run the near-miss is built on: {text}"
        );
    }
    let lines: Vec<(u64, u64)> = occurrences(clone)
        .iter()
        .map(|occurrence| {
            Ok((
                field(occurrence, "start_line")
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("start_line missing: {occurrence:#}"))?,
                field(occurrence, "end_line")
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("end_line missing: {occurrence:#}"))?,
            ))
        })
        .collect::<Result<_>>()?;
    assert_eq!(
        lines,
        vec![
            (*STANDARD_VIEW_LINES.start(), *STANDARD_VIEW_LINES.end()),
            (*PREMIUM_VIEW_LINES.start(), *PREMIUM_VIEW_LINES.end()),
        ],
        "each occurrence is the wider authored view, not the exact run inside it: {clone:#}"
    );
    let spans: Vec<u64> = views
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
