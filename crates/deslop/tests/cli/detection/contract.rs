//! Shared detection contract assertions.

use super::*;

pub(super) const TYPE2_EXPECTED_FILES_ANALYSED: u64 = 2;
pub(super) const TYPE2_EXPECTED_OCCURRENCES: usize = 2;
pub(super) const VALID_DIAGNOSTIC_LEVELS: [&str; 5] =
    ["error", "warning", "information", "hint", "none"];

/// [LANG-CAND-GO] [PIPELINE-CLUSTER-EXACT-SCOPE] The Go fixtures this suite
/// drives, with the `--min-nodes` each is driven at. Every report they
/// produce must satisfy the authored-window contract in `common::go_scope`,
/// so the fixture names are constants rather than literals repeated at each
/// call site.
pub(super) const GO_SMALL_FIXTURE: &str = "go-small";
pub(super) const GO_SMALL_MIN_NODES: &str = "10";
pub(super) const GO_SMALL_FIRST: &str = "alpha.go";
pub(super) const GO_SMALL_SECOND: &str = "beta.go";
pub(super) const GO_TYPE3_FIXTURE: &str = "go-type3";
pub(super) const GO_TYPE3_MIN_NODES: &str = "8";
pub(super) const GO_CLOSURE_FIXTURE: &str = "go-closure-signature-only";
pub(super) const GO_CLOSURE_MIN_NODES: &str = "8";
pub(super) const GO_PROLOGUE_FIXTURE: &str = "go-prologue-false-positive";
pub(super) const GO_PROLOGUE_MIN_NODES: &str = "15";
pub(super) const GO_DISSIMILAR_FIXTURE: &str = "go-dissimilar-functions";
pub(super) const GO_DISSIMILAR_MIN_NODES: &str = "8";

/// Runs the CLI against `fixture(fixture_name)` with `--min-nodes
/// <min_nodes>`, asserts the process succeeded, and returns the raw
/// JSON report text. Shared by every detection test that drives a
/// fixture with an explicit `--min-nodes` and then asserts on the
/// rendered report.
pub(super) fn run_min_nodes(fixture_name: &str, min_nodes: &str) -> Result<String> {
    let (_tmp, out, mut cmd) = fixture_run(fixture_name)?;
    let _assertion = cmd.args(["--min-nodes", min_nodes]).assert().success();
    Ok(fs::read_to_string(&out.json)?)
}

/// Runs the CLI against `fixture(fixture_name)` with `extra_args`,
/// asserts success, and returns the scan root plus the parsed JSON
/// report. Shared by the issue-#34 prologue regressions, which need
/// the scan root to read back the byte slices the report claims are
/// clones.
pub(super) fn run_with_args(
    fixture_name: &str,
    extra_args: &[&str],
) -> Result<(PathBuf, serde_json::Value)> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let scan_root = fixture(fixture_name);
    let mut cmd = fixture_command(fixture_name, &tmp.path().join("report"))?;
    let _assertion = cmd.args(extra_args).assert().success();
    let json = fs::read_to_string(&out.json)?;
    let report: serde_json::Value = serde_json::from_str(&json)?;
    Ok((scan_root, report))
}

/// Asserts the canonical Type-2 report shape shared by every
/// per-language `*-small` fixture: both files analysed, one component spans
/// exactly both source files, and its only cluster-level measures are
/// mass-derived ([RANK-MASS-SUM], [FUSED-PAIR-SIGNALS]). The old
/// `structural: 1.0` cluster signal is retired from the wire; no pair-only
/// evidence may silently return.
pub(super) fn assert_type2_report(json: &str, first_file: &str, second_file: &str) -> Result<()> {
    let report: serde_json::Value = serde_json::from_str(json)?;
    assert_eq!(
        require_u64(&report, "/files_analysed", "report")?,
        TYPE2_EXPECTED_FILES_ANALYSED,
        "the Type-2 fixture must analyse both authored source files"
    );
    let clusters = require_array(&report, "/clusters", "report")?;
    let clones = assert_finding_weights(clusters)?;
    assert!(
        !clones.is_empty(),
        "a Type-2 fixture must surface a genuine clone: {json}"
    );
    let clone = require_cluster_spanning(clusters, first_file, second_file)?;
    let typed: deslop_core::report::ReportCluster = serde_json::from_value(clone.clone())?;
    assert!(
        typed.kind.is_clone(),
        "the pair must be a known clone kind: {clone}"
    );
    assert_eq!(
        cluster_file_basenames(clone),
        std::collections::BTreeSet::from([first_file.to_owned(), second_file.to_owned()]),
        "the Type-2 component must contain exactly its two fixture files"
    );
    assert_eq!(
        require_array(clone, "/occurrences", "Type-2 cluster")?.len(),
        TYPE2_EXPECTED_OCCURRENCES,
        "the Type-2 component must preserve its two exact occurrences"
    );
    assert_no_pair_surface_on_cluster(clone, "Type-2 cluster");
    for cluster in clusters {
        let severity = cluster
            .get("severity")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("cluster carries no diagnostic level: {cluster}"))?;
        assert!(
            VALID_DIAGNOSTIC_LEVELS.contains(&severity),
            "cluster {} carries no valid diagnostic level: {cluster}",
            cluster_id(cluster)
        );
    }
    Ok(())
}

/// The array `owner` carries at `pointer`, or an error dumping the whole
/// value. A missing array is a *malformed* report, not an empty one —
/// defaulting to `Vec::new()` there would let every "no cluster does X"
/// guard below pass vacuously on a run that produced nothing at all.
pub(super) fn require_array<'a>(
    value: &'a serde_json::Value,
    pointer: &str,
    owner: &str,
) -> Result<&'a Vec<serde_json::Value>> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            anyhow::anyhow!("{owner} carries no `{pointer}` array — malformed: {value:#?}")
        })
}

/// The count `owner` carries at `pointer`, or an error when it is absent.
/// A count a guard depends on must be present, never defaulted.
pub(super) fn require_u64(value: &serde_json::Value, pointer: &str, owner: &str) -> Result<u64> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("{owner} carries no `{pointer}` count: {value:#?}"))
}

/// The array at `pointer`, or an empty vector. Only for callers that have
/// already proved the surrounding report is well formed.
pub(super) fn array_or_empty(value: &serde_json::Value, pointer: &str) -> Vec<serde_json::Value> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// Parses a JSON report and returns its cluster array.
pub(super) fn report_clusters(json: &str) -> Result<Vec<serde_json::Value>> {
    let report: serde_json::Value = serde_json::from_str(json)?;
    Ok(require_array(&report, "/clusters", "report")?.clone())
}

/// Number of files the run parsed. Every "no cluster does X" guard pairs
/// with this so a run that silently discovered nothing cannot masquerade
/// as a clean result.
pub(super) fn files_analysed(json: &str) -> Result<u64> {
    require_u64(&serde_json::from_str(json)?, "/files_analysed", "report")
}

/// The cluster's reported id, or `<unknown>` when the report omits it.
/// Every cross-file guard names it in its failure message.
pub(super) fn cluster_id(cluster: &serde_json::Value) -> &str {
    cluster
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("<unknown>")
}

/// Returns the set of occurrence paths carried by `cluster`, as reported.
pub(super) fn cluster_file_paths(
    cluster: &serde_json::Value,
) -> std::collections::BTreeSet<String> {
    array_or_empty(cluster, "/occurrences")
        .iter()
        .filter_map(|occurrence| occurrence.get("path").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
        .collect()
}

/// Returns the set of occurrence file basenames carried by `cluster`.
pub(super) fn cluster_file_basenames(
    cluster: &serde_json::Value,
) -> std::collections::BTreeSet<String> {
    cluster_file_paths(cluster)
        .iter()
        .map(|path| {
            Path::new(path)
                .file_name()
                .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned())
        })
        .collect()
}

/// Finds the cluster spanning both `first_file` and `second_file`, or
/// fails with the full cluster dump so the report shape is visible.
pub(super) fn require_cluster_spanning<'a>(
    clusters: &'a [serde_json::Value],
    first_file: &str,
    second_file: &str,
) -> Result<&'a serde_json::Value> {
    clusters
        .iter()
        .find(|cluster| {
            let files = cluster_file_basenames(cluster);
            files.contains(first_file) && files.contains(second_file)
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "expected a cross-file cluster spanning {first_file} and {second_file} \
                 (genuine Type-3 body near-miss); the signature-only match must not be \
                 the only cluster; got clusters: {clusters:#?}"
            )
        })
}

/// Asserts the Type-3 near-miss contract on a cross-file cluster:
/// at least two occurrences and the mass-only wire fields. The
/// admission-floor and not-Merkle-exact bounds this helper used to
/// pin via `signals.structural` moved to the pair surface when the
/// cluster wire went mass-only: a cluster names components, pair
/// evidence names edges ([FUSED-PAIR-SIGNALS]). The not-verbatim half
/// is proven by the byte truth — a one-statement Type-3 near-miss can
/// never be Merkle-exact by construction (gh #408) — and the clean
/// surface keeps pair-only fields off the cluster.
pub(super) fn assert_type3_signals(
    scan_root: &Path,
    cluster: &serde_json::Value,
    language: &str,
) -> Result<()> {
    let occurrences = cluster
        .pointer("/occurrences")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    assert!(
        occurrences >= 2,
        "a clone cluster must have at least two occurrences, got {occurrences}",
    );
    assert!(
        !has_verbatim_pair(scan_root, cluster)?,
        "the reported view must be the near-miss itself, not a byte-identical pair: \
         a one-statement Type-3 near-miss cannot be Merkle-exact by construction (gh #408); \
         got {cluster:#}",
    );
    assert_no_pair_surface_on_cluster(cluster, &format!("{language} type-3 near-miss"));
    Ok(())
}

/// Zero-false-positive guard shared by every `*-dissimilar-functions`
/// fixture: every cluster's occurrences stay within a single file, so
/// two structurally unrelated functions are never paired as duplicates.
pub(super) fn assert_every_cluster_single_file(json: &str, language_label: &str) -> Result<()> {
    assert_eq!(
        files_analysed(json)?,
        2,
        "the {language_label} dissimilar-functions fixture has two source files; a run \
         that analysed a different number never exercised the guard below",
    );
    for (index, cluster) in report_clusters(json)?.iter().enumerate() {
        let files = cluster_file_basenames(cluster);
        assert_eq!(
            files.len(),
            1,
            "cluster #{index} ({}) spans multiple files {files:?}; the two \
             {language_label} functions are structurally unrelated and must not be \
             reported as duplicates",
            cluster_id(cluster),
        );
    }
    Ok(())
}
