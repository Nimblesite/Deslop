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
//! nothing else. A `.deslop.toml` that still carries a `[ranking]` section
//! fails the load and says why, rather than being read and ignored; the
//! `[visibility]` choices that remain change no clone's rank, mass or
//! percentage.
//!
//! Black-box E2E: drive the CLI against generated fixture repos and
//! assert against the rendered JSON report only.

use std::{fmt::Write as _, fs, path::Path};

use anyhow::Result;
use serde_json::Value;

use crate::common::{
    scan_dir::temp_scan_dir,
    signals::{
        assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
    },
    *,
};

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
    let (tmp, src) = temp_scan_dir("src")?;
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

/// What a config that still carries `[ranking]` is told.
const RANKING_RETIRED: &str = "the [ranking] section is retired";
const CONFIG_FILE: &str = ".deslop.toml";
/// Every body the section used to accept, valid or not.
const RETIRED_RANKING_BODIES: [&str; 8] = [
    "[ranking]\nstructural_only = \"keep\"\n",
    "[ranking]\nstructural_only = \"ignore\"\n",
    "[ranking]\nstructural_only = \"demote\"\n",
    "[ranking]\nstructural_only_weight = 1.0\n",
    "[ranking]\nstructural_only_weight = 2.5\n",
    "[ranking]\nstructural_only_weight = nan\n",
    "[ranking]\ndata_clone_weight = 0.5\n",
    "[ranking]\n",
];

/// A retired setting is refused, never accepted and ignored: the run fails,
/// names the section and the file, and writes no report.
#[test]
fn retired_ranking_settings_fail_the_load_and_say_why() -> Result<()> {
    for body in RETIRED_RANKING_BODIES {
        let (tmp, src) = temp_scan_dir("src")?;
        write_fixture(&src)?;
        fs::write(src.join(CONFIG_FILE), body)?;
        let output = tmp.path().join("report");
        let _assertion = deslop_cmd(&src, &output)?
            .args(["--min-nodes", "30", "--embeddings", "off"])
            .assert()
            .failure()
            .stderr(predicates::str::contains(RANKING_RETIRED))
            .stderr(predicates::str::contains(CONFIG_FILE));
        assert!(
            !output.with_extension("json").exists(),
            "{body:?}: a refused config must not produce a report"
        );
    }
    Ok(())
}

/// The clone-only figures a visibility choice must leave untouched.
#[derive(Debug, PartialEq)]
struct CloneFigures<'report> {
    /// Every cluster's rank, id and mass.
    ranked: Vec<(u64, &'report str, u64)>,
    duplicated_loc: Option<u64>,
    duplication_percent: Option<f64>,
}

fn clone_figures(report: &Value) -> CloneFigures<'_> {
    let metrics = field(report, "metrics");
    CloneFigures {
        ranked: rankable(report),
        duplicated_loc: field(metrics, "duplicated_loc").as_u64(),
        duplication_percent: field(metrics, "duplication_percent").as_f64(),
    }
}

/// [EXCLUSION-CONFIG] Neither `[visibility]` choice moves a clone: the same
/// ids, ranks, masses, duplicated lines and percentage as with no config.
#[test]
fn visibility_choices_leave_clone_rank_mass_and_percentage_untouched() -> Result<()> {
    let baseline = report_for_config(None)?;
    let expected = clone_figures(&baseline);
    assert!(
        !expected.ranked.is_empty(),
        "the mass-only ranking must be non-empty"
    );
    assert!(
        expected.duplicated_loc > Some(0),
        "the fixture holds real duplicated lines"
    );
    for choice in ["keep", "ignore"] {
        let body = format!("[visibility]\ndata_clones = \"{choice}\"\n");
        let report = report_for_config(Some(&body))?;
        assert_eq!(
            clone_figures(&report),
            expected,
            "data_clones = {choice}: no clone may change rank, mass or percentage: {report:#}"
        );
        assert_eq!(
            field(&report, "clusters_hidden").as_u64(),
            field(&baseline, "clusters_hidden").as_u64(),
            "data_clones = {choice}: the family holds no data table to hide"
        );
    }
    Ok(())
}
