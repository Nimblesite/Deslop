//! E2E coverage for [RANK-STRUCTURAL-ONLY]: clusters whose only
//! evidence is code shape (`bucket="structural_only"`) are weight-demoted
//! by default, configurable via `.deslop.toml` `[ranking] structural_only`.
//!
//! The fixture reproduces the geometry every previous fix missed
//! (#134 demoted ≥3-file scaffolding, #197 hid single-file declaration
//! families): a sibling-method family split across exactly **two** files
//! — the Dart `part`/extension idiom that kept full `NearlyIdentical`
//! weight and topped `top-offenders` on Flutter repos. A genuine
//! verbatim copy-paste pair rides along so the tests assert *relative*
//! ranking, the user-visible product.
//!
//! Black-box E2E: drive the CLI against generated fixture repos and
//! assert against the rendered JSON report only.

use std::{fmt::Write as _, fs, path::Path};

use anyhow::Result;
use assert_cmd::Command;
use serde_json::Value;

mod common;
use crate::common::*;

/// Generates one shape-identical API method. The method name, endpoint
/// literal, and every local identifier differ per call (normalisation
/// strips identifiers, so the family still fuses at `structural=1.00`
/// with no token or embedding support), and no statement run is
/// byte-identical across methods — otherwise the byte-equivalence
/// upgrade would correctly classify sub-windows as `Identical`.
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
/// files. Large enough that with the structural-only family demoted it
/// ranks first, yet small enough that an un-demoted seven-member family
/// out-weighs it — the exact inversion issues #134/#197 reported.
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
    let _assertion = Command::cargo_bin("deslop")?
        .arg(src)
        .arg("--min-nodes")
        .arg("30")
        .arg("--embeddings")
        .arg("off")
        .arg("--output")
        .arg(&output)
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

fn bucket_of(cluster: &Value) -> &str {
    cluster.get("bucket").and_then(Value::as_str).unwrap_or("?")
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
        bucket_of(cluster) == "structural_only" && cluster_touches(cluster, "inventory_api.dart")
    })
}

fn verbatim_pair_rank(report: &Value) -> usize {
    rank_where(report, |cluster| {
        bucket_of(cluster) == "identical" && cluster_touches(cluster, "sync_a.dart")
    })
}

/// [RANK-STRUCTURAL-ONLY] default policy: the two-file shape-only family
/// is surfaced and honestly labelled, but the genuine copy-paste pair
/// out-ranks it. Also pins the cross-surface contract: a `structural_only`
/// wire bucket with the structural-only interpretation sentence.
#[test]
fn structural_only_family_is_demoted_below_genuine_clone_by_default() -> Result<()> {
    let report = report_for_config(None)?;

    let family = family_rank(&report);
    let pair = verbatim_pair_rank(&report);
    assert!(
        family < usize::MAX,
        "the two-file sibling-method family must stay visible (demoted, not \
         hidden) under the default policy: {report:#}"
    );
    assert!(
        pair < usize::MAX,
        "the verbatim copy-paste pair must classify as `identical` — its raw \
         bytes are equal, so the unscored token signal must not drag it into \
         structural_only: {report:#}"
    );
    assert!(
        pair < family,
        "issues #134/#154/#197: with `structural_only = demote` (default), the \
         genuine copy-paste pair (rank {pair}) must out-rank the shape-only \
         method family (rank {family}): {report:#}"
    );

    let family_cluster = clusters(&report).get(family).cloned().unwrap_or_default();
    let interpretation = family_cluster
        .get("interpretation")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        interpretation.contains("Only the code shape matches"),
        "structural_only clusters must carry the structural-only \
         interpretation, not the nearly-identical one (issue #197 \
         inconsistency #3): {interpretation:?}"
    );
    Ok(())
}

/// `[ranking] structural_only = "keep"` restores full weight: the
/// seven-member family out-ranks the two-copy pair — the pre-fix order,
/// proving the multiplier (not a hide-filter) drives the default order.
#[test]
fn keep_policy_restores_full_weight_ranking() -> Result<()> {
    let report = report_for_config(Some("[ranking]\nstructural_only = \"keep\"\n"))?;
    let family = family_rank(&report);
    let pair = verbatim_pair_rank(&report);
    assert!(
        family < usize::MAX && pair < usize::MAX,
        "both clusters must stay visible under keep: {report:#}"
    );
    assert!(
        family < pair,
        "with structural_only = keep, the seven-member family (rank {family}) \
         must out-weigh the two-copy pair (rank {pair}) — node_count × (size−1) \
         with no demotion: {report:#}"
    );
    Ok(())
}

/// An explicit `structural_only_weight = 1.0` neutralises the demotion
/// the same way `keep` does, proving the weight knob feeds the ranking.
#[test]
fn unit_weight_neutralises_demotion() -> Result<()> {
    let report = report_for_config(Some("[ranking]\nstructural_only_weight = 1.0\n"))?;
    let family = family_rank(&report);
    let pair = verbatim_pair_rank(&report);
    assert!(
        family < pair,
        "structural_only_weight = 1.0 must restore the un-demoted order \
         (family rank {family}, pair rank {pair}): {report:#}"
    );
    Ok(())
}

/// `[ranking] structural_only = "ignore"` drops the family from the
/// report entirely and counts it in `clusters_hidden`; the genuine pair
/// is untouched.
#[test]
fn ignore_policy_hides_structural_only_clusters() -> Result<()> {
    let report = report_for_config(Some("[ranking]\nstructural_only = \"ignore\"\n"))?;
    assert!(
        !clusters(&report)
            .iter()
            .any(|cluster| bucket_of(cluster) == "structural_only"),
        "ignore must drop every structural_only cluster from the ranked \
         report: {report:#}"
    );
    assert!(
        verbatim_pair_rank(&report) < usize::MAX,
        "ignore must not touch the genuine identical pair: {report:#}"
    );
    let hidden = report
        .get("clusters_hidden")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    assert!(
        hidden >= 1,
        "the dropped family must be counted in clusters_hidden: {report:#}"
    );
    Ok(())
}

/// An out-of-range `structural_only_weight` fails the load with a
/// diagnostic naming the key, mirroring `data_clone_weight` validation.
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
            .arg("--min-nodes")
            .arg("30")
            .arg("--embeddings")
            .arg("off")
            .assert()
            .failure()
            .stderr(predicates::str::contains("structural_only_weight"))
            .stderr(predicates::str::contains(fragment));
    }
    Ok(())
}
