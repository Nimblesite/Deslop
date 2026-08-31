//! E2E coverage for [RANK-MASS-SUM] on the two-file sibling-method
//! family fixture: clusters are ordered by pure mass
//! `canonical_node_count × (occurrence_count − 1)`.
//!
//! The fixture reproduces the geometry every previous fix missed
//! (#134 demoted ≥3-file scaffolding, #197 hid single-file declaration
//! families): a sibling-method family split across exactly **two** files
//! — the Dart `part`/extension idiom that used to receive a
//! pair-classification weight and top `top-offenders` on Flutter repos. A
//! genuine verbatim copy-paste pair rides along so the tests assert relative
//! mass ranking, the user-visible product.
//!
//! [RANK-STRUCTURAL-ONLY] retired the `structural_only_weight`,
//! `data_clone_weight` and `demote` ranking modes: weight means mass and
//! nothing else. The legacy config keys still parse (backwards
//! compatibility) but must not change the ranking — that is what the
//! retired-knob test below pins.
//!
//! Black-box E2E: drive the CLI against generated fixture repos and
//! assert against the rendered JSON report only.

use std::{fmt::Write as _, fs, path::Path};

use anyhow::Result;
use assert_cmd::Command;
use serde_json::Value;

use crate::common::signals::{
    assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
};
use crate::common::*;

/// Generates one shape-identical API method. The method name, endpoint
/// literal, and every local identifier differ per call (normalisation
/// strips identifiers), and no statement run is byte-identical across
/// methods — otherwise the byte-equivalence upgrade would correctly
/// classify sub-windows as `Identical`.
fn api_method(name: &str, prefix: &str, endpoint: &str) -> String {
    format!(
        "  Future<List<String>> {name}() async {{\n    final {prefix}Response = await http\n        \
         .getMethod<List<Object?>>('/catalog/$uid/settings/{endpoint}');\n    \
         final {prefix}Rows = {prefix}Response.data ?? const <Object?>[];\n    \
         final {prefix}Labels = {prefix}Rows.map(({prefix}Row) => {prefix}Row.toString()).toList();\n    \
         {prefix}Labels.sort(({prefix}Left, {prefix}Right) => {prefix}Left.compareTo({prefix}Right));\n    \
         return {prefix}Labels.cast<String>();\n  }}\n\n"
    )
}

/// The genuine copy-pasted logic clone, duplicated verbatim across two
/// files. It must rank second: the seven-member family carries more
/// duplicated mass (7−1 copies × 101 nodes vs 1 copy × 143 nodes).
const DUPLICATED_FUNCTION: &str = "int mergeTotals(List<int> counts, List<int> offsets) {\n  \
     var total = 0;\n  var carry = 1;\n  for (final count in counts) {\n    \
     final scaled = count * carry;\n    final shifted = scaled + offsets.length;\n    \
     total = total + shifted;\n    carry = carry + 1;\n  }\n  \
     for (final offset in offsets) {\n    final damped = offset - carry;\n    \
     final folded = damped * 2;\n    total = total - folded;\n    \
     carry = carry * 1;\n  }\n  var checksum = 0;\n  for (final count in counts) {\n    \
     final mixed = count + carry;\n    final spun = mixed * total;\n    \
     checksum = checksum + spun;\n  }\n  return total + carry + checksum;\n}\n";

/// Writes the two-file sibling-method family plus the verbatim pair.
fn write_fixture(src: &Path) -> Result<()> {
    fs::create_dir_all(src)?;
    let shim = "class ApiResponse<T> {\n  ApiResponse(this.data);\n  final T? data;\n}\n\n\
         class HttpShim {\n  Future<ApiResponse<T>> getMethod<T>(String path) async =>\n      \
         throw UnsupportedError(path);\n}\n";
    fs::write(src.join("http_shim.dart"), shim)?;

    let mut inventory = String::from(
        "import 'http_shim.dart';\n\nclass InventoryApi {\n  InventoryApi(this.http, this.uid);\n\n  \
         final HttpShim http;\n  final String uid;\n\n",
    );
    for (name, prefix, endpoint) in [
        ("fetchStockLocations", "stock", "stock-locations"),
        ("fetchReorderPoints", "reorder", "reorder-points"),
        ("fetchSupplierCodes", "supplier", "supplier-codes"),
        ("fetchAuditTrails", "audit", "audit-trails"),
    ] {
        let _ = write!(inventory, "{}", api_method(name, prefix, endpoint));
    }
    inventory.push_str("}\n");
    fs::write(src.join("inventory_api.dart"), inventory)?;

    let mut catalog = String::from(
        "import 'http_shim.dart';\n\nclass CatalogApi {\n  CatalogApi(this.http, this.uid);\n\n  \
         final HttpShim http;\n  final String uid;\n\n",
    );
    for (name, prefix, endpoint) in [
        ("fetchDisplayBanners", "banner", "display-banners"),
        ("fetchPricingTiers", "pricing", "pricing-tiers"),
        ("fetchSeasonLabels", "season", "season-labels"),
    ] {
        let _ = write!(catalog, "{}", api_method(name, prefix, endpoint));
    }
    catalog.push_str("}\n");
    fs::write(src.join("catalog_api.dart"), catalog)?;

    fs::write(src.join("sync_a.dart"), DUPLICATED_FUNCTION)?;
    fs::write(src.join("sync_b.dart"), DUPLICATED_FUNCTION)?;
    Ok(())
}

/// Runs the CLI against `src` and parses the JSON report.
fn run_report(src: &Path, tmp: &Path) -> Result<Value> {
    let output = tmp.join("report");
    let _assertion = deslop_cmd(src, &output)?
        .args(["--min-nodes", "30", "--embeddings", "off"])
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

/// Builds the fixture (plus optional `.deslop.toml` body) and reports.
fn report_for_config(config: Option<&str>) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let src = tmp.path().join("src");
    write_fixture(&src)?;
    if let Some(body) = config {
        fs::write(src.join(".deslop.toml"), body)?;
    }
    let report = run_report(&src, tmp.path())?;
    Ok(report)
}

fn cluster_touches(cluster: &Value, file_name: &str) -> bool {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|occ| occ.get("path").and_then(Value::as_str))
        .any(|path| path.ends_with(file_name))
}

/// Zero-based rank of the first cluster matching `predicate`, or
/// [`usize::MAX`] when none does — callers assert presence with
/// `< usize::MAX`.
fn rank_where(report: &Value, predicate: impl Fn(&Value) -> bool) -> usize {
    clusters(report)
        .iter()
        .position(predicate)
        .unwrap_or(usize::MAX)
}

fn family_rank(report: &Value) -> usize {
    rank_where(report, |cluster| {
        cluster_touches(cluster, "inventory_api.dart")
    })
}

fn verbatim_pair_rank(report: &Value) -> usize {
    rank_where(report, |cluster| cluster_touches(cluster, "sync_a.dart"))
}

/// The cluster at `rank`, or `None`.
fn cluster_at(report: &Value, rank: usize) -> Option<&Value> {
    clusters(report).get(rank)
}

/// [RANK-MASS-SUM] default: both clusters are visible, ranked by pure
/// mass. The seven-member family out-masses the two-copy pair under the
/// rendered formula. The canonical node count is produced by the current
/// normaliser, so this test re-derives mass instead of freezing an obsolete
/// implementation count. The family spans exactly the seven sibling methods,
/// and the pair is byte-proven verbatim.
#[test]
fn mass_ranks_family_first_and_pair_second() -> Result<()> {
    let src = tempfile::tempdir()?;
    let root = src.path().join("src");
    write_fixture(&root)?;
    let report = run_report(&root, src.path())?;

    let family = family_rank(&report);
    let pair = verbatim_pair_rank(&report);
    assert!(
        family < usize::MAX && pair < usize::MAX,
        "both clusters must stay visible under the mass-only policy: {report:#}"
    );
    assert!(
        family < pair,
        "[RANK-MASS-SUM]: the seven-member family (rank {family}) must out-rank \
         the two-copy pair (rank {pair}) by pure mass: {report:#}"
    );

    let family_cluster =
        cluster_at(&report, family).ok_or_else(|| anyhow::anyhow!("family cluster missing"))?;
    // [PIPELINE-CLUSTER-SUBSUME]: the family IS seven duplicated methods.
    // A whole-class view would report two occurrences spanning each entire
    // class — losing five findings and counting non-duplicated members as
    // duplicated.
    assert_eq!(
        cluster_size(family_cluster),
        7,
        "the family must report all seven sibling methods, not a whole-class \
         view that encloses them: {family_cluster:#}"
    );
    let family_nodes = field(family_cluster, "canonical_node_count")
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("family has no canonical node count: {family_cluster:#}"))?;
    let family_mass = field(family_cluster, "mass")
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("family has no mass: {family_cluster:#}"))?;
    assert_eq!(
        family_mass,
        family_nodes.saturating_mul(cluster_size(family_cluster).saturating_sub(1)),
        "family mass must be canonical_node_count × (occurrence_count − 1): {family_cluster:#}"
    );
    assert_no_pair_surface_on_cluster(family_cluster, "rank-structural-only family");
    assert_structural_only_contract(family_cluster, "rank-structural-only family");

    let pair_cluster = cluster_at(&report, pair)
        .ok_or_else(|| anyhow::anyhow!("verbatim pair cluster missing"))?;
    assert_eq!(
        cluster_size(pair_cluster),
        2,
        "the verbatim pair spans exactly its two copies: {pair_cluster:#}"
    );
    assert!(
        has_verbatim_pair(&root, pair_cluster)?,
        "the copy-paste pair is byte-identical in source — the byte-proven \
         fact must hold on the mass-only wire: {pair_cluster:#}"
    );
    let pair_mass = field(pair_cluster, "mass")
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("verbatim pair has no mass: {pair_cluster:#}"))?;
    assert!(
        family_mass > pair_mass,
        "the rank order must follow larger family mass ({family_mass} > {pair_mass}): {report:#}"
    );
    Ok(())
}

/// The retired `structural_only` / `data_clone_weight` knobs still parse
/// for backwards compatibility but [RANK-STRUCTURAL-ONLY] forbids them
/// from changing weight: every legacy body must render the identical
/// report (same ids, same masses, same order).
#[test]
fn retired_structural_only_knobs_do_not_change_the_ranking() -> Result<()> {
    let baseline = report_for_config(None)?;
    let ranked_baseline = rankable(&baseline);
    for (body, label) in [
        ("[ranking]\nstructural_only = \"keep\"\n", "keep"),
        ("[ranking]\nstructural_only_weight = 1.0\n", "unit weight"),
        ("[ranking]\nstructural_only = \"ignore\"\n", "ignore"),
        ("[ranking]\ndata_clone_weight = 0.5\n", "data clone weight"),
    ] {
        let report = report_for_config(Some(body))?;
        assert_eq!(
            rankable(&report),
            ranked_baseline,
            "{label}: the retired {label} knob must not change mass or order — \
             weight means mass and nothing else ([RANK-STRUCTURAL-ONLY]): {report:#}"
        );
        assert_eq!(
            field(&report, "clusters_hidden").as_u64(),
            Some(0),
            "{label}: the retired knob must not hide the family: {report:#}"
        );
    }
    assert!(
        !ranked_baseline.is_empty(),
        "the mass-only ranking must be non-empty"
    );
    Ok(())
}

/// The stable, order-insensitive fingerprint of a report's ranking:
/// `(rank, id, mass)` per cluster, so a retired knob cannot reorder or
/// re-mass without the assertion seeing it.
fn rankable(report: &Value) -> Vec<(u64, &str, u64)> {
    let mut rows: Vec<(u64, &str, u64)> = clusters(report)
        .iter()
        .map(|cluster| {
            (
                field(cluster, "rank").as_u64().unwrap_or(0),
                cluster_id(cluster),
                field(cluster, "mass").as_u64().unwrap_or(0),
            )
        })
        .collect();
    rows.sort_unstable();
    rows
}

/// An out-of-range `structural_only_weight` still fails the load with a
/// diagnostic naming the key — the legacy keys are parsed and validated
/// even though they no longer feed ranking.
#[test]
fn invalid_structural_only_weight_is_rejected_with_a_clear_error() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let src = tmp.path().join("src");
    write_fixture(&src)?;
    for (body, fragment) in [
        (
            "[ranking]\nstructural_only_weight = 2.5\n",
            "range (0.0, 1.0]",
        ),
        (
            "[ranking]\nstructural_only_weight = 0.0\n",
            "range (0.0, 1.0]",
        ),
        (
            "[ranking]\nstructural_only_weight = nan\n",
            "must be finite",
        ),
    ] {
        fs::write(src.join(".deslop.toml"), body)?;
        let _assertion = Command::cargo_bin("deslop")?
            .arg(&src)
            .args(["--min-nodes", "30", "--embeddings", "off"])
            .assert()
            .failure()
            .stderr(predicates::str::contains("structural_only_weight"))
            .stderr(predicates::str::contains(fragment));
    }
    Ok(())
}
