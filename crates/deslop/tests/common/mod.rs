//! Shared helpers for the standalone (non-`cli`) end-to-end regression
//! tests. Each `tests/<name>.rs` integration binary is its own crate, so
//! this module is pulled in with `mod common;` and used through
//! `use crate::common::*;`. It centralises the fixture-path lookup, the
//! `deslop` invocation, and the report-walking helpers that every
//! per-issue false-positive test would otherwise copy verbatim.
//!
//! Each integration binary pulls in only the subset of helpers it needs,
//! so the unused-symbol lint is silenced for this shared module (matching
//! the `deslop-core` and `deslop-mcp` test commons).

#![allow(dead_code)]

/// Legacy pair-signal assertion vocabulary pending explicit compare migration.
/// Suites that assert on reported evidence import it explicitly with
/// `use crate::common::signals::*;` — a glob re-export here would be an
/// unused import in every binary that never touches that vocabulary.
/// The two-sided contract every noise-family pin is judged by: the
/// family stays hidden while a real clone in the same run stays visible.
pub(crate) mod negative_pin;

pub(crate) mod signals;

pub(crate) mod findings;
pub(crate) use findings::clone_findings;

/// The deterministic mock-embedder runner. Imported explicitly with
/// `use crate::common::embeddings::*;`, for the same reason as
/// `signals`.
pub(crate) mod embeddings;

/// Report-verdict assertions: metric totals ([METRICS-REPO]) and the
/// shape of an expected cluster. Imported explicitly with
/// `use crate::common::verdict::*;`, for the same reason as `signals`.
pub(crate) mod verdict;

/// Exact `cache_stats` assertions and the strip-and-compare view of
/// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]. Imported explicitly
/// with `use crate::common::incremental::*;`, for the same reason as
/// `signals`.
pub(crate) mod incremental;

/// The authored clone corpus, its cold ground truth, and the positive
/// report-shape assertions both equivalence suites are judged by —
/// the batch-process one and the live-session one. Imported explicitly
/// with `use crate::common::clone_corpus::*;`, for the same reason as
/// `signals`.
pub(crate) mod clone_corpus;

/// The `--rerun-add SRC=DST` spec vocabulary shared by every suite that
/// mutates a tree between the initial analysis and the rerun. Imported
/// explicitly with `use crate::common::rerun_ops::*;`, for the same
/// reason as `signals`.
pub(crate) mod rerun_ops;

/// The six-language `incremental-multilang` fixture vocabulary. Imported
/// explicitly with `use crate::common::multilang::*;`, for the same
/// reason as `signals`.
pub(crate) mod multilang;

/// Warm-store scenarios over that fixture — the baseline, the targeted
/// mutation, and the reuse accounting each mutation must produce.
/// Imported explicitly with `use crate::common::multilang_warm::*;`, for
/// the same reason as `signals`.
pub(crate) mod multilang_warm;

/// The GH #119 role-gate contract, asserted once for every language
/// ([CLONE-NOISE-EMBEDDING-ROLE-MISMATCH]). Imported explicitly with
/// `use crate::common::role_gate::*;`, for the same reason as `signals`.
pub(crate) mod role_gate;

/// The committed `diff-scope` fixture's vocabulary — what the patch
/// adds, and how to drive the CLI over it. Imported explicitly with
/// `use crate::common::diff_scope::*;`, for the same reason as
/// `signals`.
pub(crate) mod diff_scope;

/// The committed-golden comparison and its `DESLOP_BLESS` regeneration
/// path ([PIPELINE-DETERMINISM]). Imported explicitly with
/// `use crate::common::golden::*;`, for the same reason as `signals`.
pub(crate) mod golden;

/// The three-file seeded Rust corpus the store-accounting suites share.
/// Imported explicitly with `use crate::common::seeded::*;`, for the
/// same reason as `signals`.
pub(crate) mod seeded;

/// The temp-workspace scaffold: a bound [`tempfile::TempDir`] plus an
/// empty scan root inside it. Imported explicitly with
/// `use crate::common::scan_dir::*;`, for the same reason as
/// `signals`.
pub(crate) mod scan_dir;
pub(crate) use scan_dir::temp_scan_dir;

/// Reading the on-disk parse store and a run's tracing log. Imported
/// explicitly with `use crate::common::store::*;`, for the same reason
/// as `signals`.
pub(crate) mod store;

/// Building a corpus on disk before the tool runs. Imported explicitly
/// with `use crate::common::corpora::*;`, for the same reason as
/// `signals`.
pub(crate) mod corpora;

/// The `verbatim-subgroup` fixture vocabulary, shared by the suite
/// pinning that a copy survives an unrelated cluster member and the one
/// pinning the price the cross-file arbitration accepts. Imported
/// explicitly with `use crate::common::verbatim_subgroup::*;`, for the
/// same reason as `signals`.
/// One [CLONE-NOISE-POLYMORPHIC-SIGNATURE] scan proving a contract
/// pair stays hidden while a rename clone beside it still surfaces.
pub(crate) mod contract_boundary;
pub(crate) mod verbatim_subgroup;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    ops::RangeInclusive,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
pub(crate) use anyhow::Result;
use assert_cmd::Command;
/// Appends `.<ext>` to a path's file name. The suite's single spelling of
/// "the sibling output the run wrote"; four suites carried their own
/// three-line delegate to it ([CI-DESLOP] ledger).
pub(crate) use deslop_test_support::with_ext;
use serde_json::Value;

/// Absolute path to the named directory under `tests/fixtures`, falling
/// back to the `deslop-mcp` crate's fixture tree.
///
/// The fallback is the mirror of `deslop-mcp`'s `copied_fixture_named`,
/// which resolves the other way. A corpus that proves a detection defect
/// through the MCP surface proves the same defect through the CLI, and
/// the two suites must read the *same* bytes: a second copy of the
/// fixture would let one suite go green while the code it pins is still
/// broken under the other.
pub(crate) fn fixture(name: &str) -> PathBuf {
    let local = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    if local.is_dir() {
        return local;
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../deslop-mcp/tests/fixtures")
        .join(name)
}

/// Builds `deslop <scan_root> --output <output_prefix> --no-incremental`,
/// ready for the caller to append scenario-specific flags before
/// `.assert()`. Centralises the `cargo_bin` lookup + scan-root +
/// output-prefix prefix that every invocation shares.
///
/// `--no-incremental` is part of the shared prefix because the fingerprint
/// cache is on by default ([PIPELINE-INCREMENTAL]) and always lands in the
/// *scan root* ([OUTPUT-DIR]) — `--output` cannot redirect it. These
/// binaries assert on detection output, and most point straight at a
/// checked-in `tests/fixtures/` directory, so without the opt-out every
/// run would write into the fixture tree and concurrent binaries sharing a
/// fixture would race on the same cache. Cache behaviour itself is covered
/// in `tests/cli/cache_and_debug.rs`, against scan roots it owns.
pub(crate) fn deslop_cmd(scan_root: &Path, output_prefix: &Path) -> Result<Command> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _cmd = cmd
        .arg(scan_root)
        .arg("--output")
        .arg(output_prefix)
        .arg("--no-incremental");
    Ok(cmd)
}

/// Writes two byte-identical source files (`a.<extension>`, `b.<extension>`)
/// into a freshly created `dir`: the minimal corpus for a fully-duplicated
/// repo, used to prove the duplication metric is language-agnostic.
pub(crate) fn write_identical_pair(dir: &Path, extension: &str, source: &str) -> Result<()> {
    fs::create_dir_all(dir)?;
    for stem in ["a", "b"] {
        fs::write(dir.join(format!("{stem}.{extension}")), source)?;
    }
    Ok(())
}

/// Runs `deslop <scan_root> <extra_args...> --output <tmp>/report` into a
/// throwaway temp dir and returns the parsed JSON report after asserting the
/// process exits successfully. The flexible-flag core behind [`run_report`].
pub(crate) fn run_report_args(scan_root: &Path, extra_args: &[&str]) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let mut cmd = deslop_cmd(scan_root, &output)?;
    let _assertion = cmd.args(extra_args).assert().success();
    load_json(&output.with_extension("json"))
}

/// Runs `deslop <scan_root> --min-nodes <min_nodes> --embeddings off` into
/// a throwaway temp dir and returns the parsed JSON report. Asserts the
/// process exits successfully before the report is read.
pub(crate) fn run_report(scan_root: &Path, min_nodes: u32) -> Result<Value> {
    let min_nodes = min_nodes.to_string();
    run_report_args(
        scan_root,
        &["--min-nodes", min_nodes.as_str(), "--embeddings", "off"],
    )
}

/// Runs `deslop` over the named fixture at `min_nodes`, leaving the
/// embedding pass at its default so the signature suites drive all three
/// layers ([FUSED-SIGNALS-THREE-LAYER]).
pub(crate) fn run_fixture_report(fixture_name: &str, min_nodes: u32) -> Result<Value> {
    let min_nodes = min_nodes.to_string();
    run_report_args(&fixture(fixture_name), &["--min-nodes", min_nodes.as_str()])
}

/// A writable copy of `fixture_name` at `<tmp>/src`, returned with the
/// temp dir that owns it so the caller controls its lifetime. The shape
/// every suite needs before it can mutate a scan root.
pub(crate) fn seeded_fixture_root(fixture_name: &str) -> Result<(tempfile::TempDir, PathBuf)> {
    let (tmp, scan_root) = temp_scan_dir("src")?;
    seed(&fixture(fixture_name), &scan_root)?;
    Ok((tmp, scan_root))
}

/// Asserts `haystack` contains `needle`, failing with `context` plus the
/// needle and the whole haystack.
///
/// Every suite that reads a rendered report,
/// stderr stream or log body asks the same question — "is this marker
/// present?" — and every one of them used to hand-roll the same
/// `assert!(x.contains(y), "…: {x}")` shape with its own decision about
/// whether to print the haystack at all. The check and its diagnostic
/// live here once, so a failure always names the marker AND shows the
/// text that was searched.
pub(crate) fn assert_contains(haystack: &str, needle: &str, context: &str) {
    assert!(
        haystack.contains(needle),
        "{context}\n  expected to contain: {needle:?}\n  actual output:\n{haystack}"
    );
}

/// The negative half of [`assert_contains`]: `haystack` must NOT contain
/// `needle`. Same diagnostic, so a suppression that silently stopped
/// suppressing names the marker it let through.
pub(crate) fn assert_not_contains(haystack: &str, needle: &str, context: &str) {
    assert!(
        !haystack.contains(needle),
        "{context}\n  must not contain: {needle:?}\n  actual output:\n{haystack}"
    );
}

/// Parses the JSON document at `path` into a [`Value`].
pub(crate) fn load_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

/// Reads a named field off `value`, returning [`Value::Null`] when absent so
/// callers get a deterministic `!=` instead of a panic.
pub(crate) fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    value.get(name).unwrap_or(&Value::Null)
}

/// The first row in `rows` whose reported `path` ends with `suffix`.
/// Suffix matching, so a suite names the bare file wherever the fixture
/// nests it.
pub(crate) fn row_for_path<'a>(rows: &'a [Value], suffix: &str) -> Option<&'a Value> {
    rows.iter().find(|row| {
        field(row, "path")
            .as_str()
            .is_some_and(|path| path.ends_with(suffix))
    })
}

/// The `duplicated_loc` the per-file metrics report for `file`, or zero
/// when the file has no duplication row at all.
pub(crate) fn duplicated_loc_for(report: &Value, file: &str) -> u64 {
    row_for_path(per_file_metrics(report), file).map_or(0, |row| {
        field(row, "duplicated_loc").as_u64().unwrap_or_default()
    })
}

/// The negative-control contract: `fixture_name` must analyse exactly
/// `files` sources and report no clone at all. The file count guards
/// against a vacuous pass — "no clusters" from a silently-broken parser
/// that produced zero fingerprints proves nothing. Returns the report so
/// a caller can pin what was suppressed on top.
pub(crate) fn assert_no_clone_reported(
    fixture_name: &str,
    min_nodes: u32,
    files: u64,
) -> Result<Value> {
    let report = run_report(&fixture(fixture_name), min_nodes)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(files),
        "{fixture_name}: all {files} file(s) must be analysed: {report:#}"
    );
    assert!(
        clusters(&report).is_empty(),
        "{fixture_name}: unrelated code must not be reported as a clone: {report:#}"
    );
    Ok(report)
}

/// [CLONE-KIND-LABELS] The wire spelling of the cluster kind every member
/// of which is byte-identical to the canonical occurrence.
pub(crate) const IDENTICAL_KIND: &str = "identical";
/// [CLONE-KIND-LABELS] The title every surface renders for that kind.
pub(crate) const IDENTICAL_TITLE: &str = "Identical code";
/// [CLONE-KIND-LABELS] The wire spelling of the renamed / near-copy kind.
pub(crate) const NEARLY_IDENTICAL_KIND: &str = "nearly_identical";
/// [CLONE-KIND-LABELS] The title every surface renders for that kind.
pub(crate) const NEARLY_IDENTICAL_TITLE: &str = "Nearly identical code";
/// [CLONE-KIND-LABELS] The wire spelling of the equivalent-behaviour kind:
/// the same work implemented with different code (Type-4).
pub(crate) const SAME_BEHAVIOR_KIND: &str = "same_behavior";
/// [CLONE-KIND-LABELS] The title every surface renders for that kind.
pub(crate) const SAME_BEHAVIOR_TITLE: &str = "Same behavior, different code";
/// [CLONE-KIND-LABELS] The wire spelling of the informational non-clone
/// kind: matching shape carrying no or negligible shared content.
pub(crate) const STRUCTURAL_ONLY_KIND: &str = "structural_only";
/// [CLONE-KIND-LABELS] The title every surface renders for that kind.
pub(crate) const STRUCTURAL_ONLY_TITLE: &str = "Same shape, different content";
/// [CLONE-KIND-LABELS] The wire spelling of the more heavily edited
/// established-copy kind. The machine key keeps the legacy spelling; the
/// human title is "Similar code".
pub(crate) const LOOSELY_SIMILAR_KIND: &str = "loosely_similar";
/// [CLONE-KIND-LABELS] The title every surface renders for that kind.
pub(crate) const LOOSELY_SIMILAR_TITLE: &str = "Similar code";

/// [CLONE-KIND-FOLD] The kind the engine folded for `cluster`, or `""`
/// when the report omits it so the assertion trips with the JSON printed.
pub(crate) fn cluster_kind(cluster: &Value) -> &str {
    field(cluster, "kind").as_str().unwrap_or_default()
}

/// Length of a named array-valued field, or `0` when missing / non-array (so
/// the assertion trips with the full JSON printed rather than panicking).
pub(crate) fn array_len(value: &Value, name: &str) -> usize {
    field(value, name).as_array().map_or(0, Vec::len)
}

/// Copies every top-level entry in `src` into a freshly created `dst`. Used
/// by tests that need a mutable scan root seeded from an immutable fixture.
pub(crate) fn seed(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        // Source files only. A fixture directory can pick up a nested
        // `.deslop/` output directory ([OUTPUT-DIR]) from a stray run, and
        // `fs::copy` on a directory fails outright — seeding must not be
        // hostage to whatever else happens to be sitting there.
        if !entry.file_type()?.is_file() {
            continue;
        }
        let _bytes = fs::copy(entry.path(), dst.join(entry.file_name()))?;
    }
    Ok(())
}

/// The `clusters` array of a report, or an empty slice when absent.
pub(crate) fn clusters(report: &Value) -> &[Value] {
    report
        .get("clusters")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// The `occurrences` array of a cluster, or an empty slice when absent.
pub(crate) fn occurrences(cluster: &Value) -> &[Value] {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// A cluster's stable `id`, or `"?"` when absent.
/// The 1-indexed `(start_line, end_line)` one reported occurrence
/// covers. Line numbers, not byte offsets, so a failing assertion names
/// the physical rows a reader can open.
pub(crate) fn occurrence_line_span(occurrence: &Value) -> (u64, u64) {
    (
        field(occurrence, "start_line").as_u64().unwrap_or(0),
        field(occurrence, "end_line").as_u64().unwrap_or(0),
    )
}

/// Every occurrence's line span within one cluster, in report order.
pub(crate) fn cluster_line_spans(cluster: &Value) -> Vec<(u64, u64)> {
    occurrences(cluster)
        .iter()
        .map(occurrence_line_span)
        .collect()
}

/// Every occurrence's line span across every visible cluster, in report
/// order.
pub(crate) fn report_line_spans(report: &Value) -> Vec<(u64, u64)> {
    clusters(report)
        .iter()
        .flat_map(occurrences)
        .map(occurrence_line_span)
        .collect()
}

pub(crate) fn cluster_id(cluster: &Value) -> &str {
    field(cluster, "id").as_str().unwrap_or("?")
}

/// A cluster's occurrence count, or `0` when absent. The wire carries
/// the count as `occurrence_count` (visible membership) with
/// `occurrences_total` alongside; `size` was removed with the bucket
/// surface.
pub(crate) fn cluster_size(cluster: &Value) -> u64 {
    field(cluster, "occurrence_count").as_u64().unwrap_or(0)
}

/// Number of clusters the report rendered — the [METRICS-REPO] visible set.
pub(crate) fn cluster_count(report: &Value) -> usize {
    array_len(report, "clusters")
}

/// True when `value` lies within `1e-9` of `target`. Lets a test pin an
/// exact signal value (typically `0.0` or `1.0`) without tripping
/// the float-equality lint or carrying its own epsilon.
pub(crate) fn approx(value: f64, target: f64) -> bool {
    (value - target).abs() <= 1e-9
}

/// The sorted, de-duplicated bare file names a cluster's occurrences span.
pub(crate) fn cluster_file_set(cluster: &Value) -> BTreeSet<String> {
    occurrence_files(cluster).into_iter().collect()
}

/// True when `cluster` has an occurrence in every one of `files`, matched
/// by bare file name. The one place a suite asks "is this the cross-file
/// clone I mean?".
pub(crate) fn cluster_covers_files(cluster: &Value, files: &[&str]) -> bool {
    let present = cluster_file_set(cluster);
    files.iter().all(|name| present.contains(*name))
}

/// The first cluster whose occurrences cover every name in `files`, or
/// `None`. Lets a test target the specific cross-file clone it cares about
/// regardless of report ordering or unrelated clusters.
pub(crate) fn cluster_spanning<'a>(report: &'a Value, files: &[&str]) -> Option<&'a Value> {
    clusters(report)
        .iter()
        .find(|cluster| cluster_covers_files(cluster, files))
}

/// True when any occurrence of `cluster` sits in a file whose reported
/// path ends with `file_name`. Suffix matching, so a suite naming the
/// bare file matches it wherever the fixture nests it.
pub(crate) fn cluster_touches(cluster: &Value, file_name: &str) -> bool {
    occurrence_paths(cluster)
        .iter()
        .any(|path| path.ends_with(file_name))
}

/// True when any *visible* cluster in the report touches `file_name` —
/// the question every "this file must (not) be reported" pin asks.
pub(crate) fn report_touches(report: &Value, file_name: &str) -> bool {
    clusters(report)
        .iter()
        .any(|cluster| cluster_touches(cluster, file_name))
}

/// True when one cluster's occurrences cover both files — the shape a
/// cross-file clone has, and the shape a suppressed family must not.
pub(crate) fn cluster_spans(cluster: &Value, left: &str, right: &str) -> bool {
    cluster_touches(cluster, left) && cluster_touches(cluster, right)
}

/// True when any visible cluster spans both files. Hidden clusters are
/// dropped before serialisation, so every cluster considered here is one
/// a human is actually shown.
pub(crate) fn report_spans(report: &Value, left: &str, right: &str) -> bool {
    clusters(report)
        .iter()
        .any(|cluster| cluster_spans(cluster, left, right))
}

/// Like [`cluster_spanning`] but fails the test with the full report when
/// no matching cluster exists, so the failure message is actionable.
pub(crate) fn expect_cluster_spanning<'a>(report: &'a Value, files: &[&str]) -> Result<&'a Value> {
    cluster_spanning(report, files)
        .ok_or_else(|| anyhow!("expected a clone spanning {files:?}: {report:#}"))
}

/// Drives `deslop` over `fixture_dir` at `min_nodes` and asserts the
/// cross-file clone spanning `files` exists with `structural == 1.0`, the
/// expected `bucket`, and the token signal that bucket implies: a full
/// token signal for `identical` / `nearly_identical`, a near-zero one for
/// the structural-only routing (#134). Centralises the renamed-clone
/// assertion every per-feature E2E test would otherwise repeat.
///
/// The bucket labels are gone from the wire; what remains provable is
/// the byte-level truth the labels used to proxy: `byte_identical`
/// asserts the clone's occurrences are byte-identical text, `false`
/// asserts they are not (a rename/near-miss). Either way the cluster is
/// asserted to be admitted, visible, mass-honest, and free of any
/// pair-only surface ([PIPELINE-CLUSTER-CLOSURE]).
pub(crate) fn assert_bucketed_clone(
    fixture_dir: &str,
    min_nodes: u32,
    files: &[&str],
    byte_identical: bool,
) -> Result<()> {
    let scan_root = fixture(fixture_dir);
    let report = run_report(&scan_root, min_nodes)?;
    let clone = expect_cluster_spanning(&report, files)?;
    signals::assert_structural_only_contract(clone, fixture_dir);
    signals::assert_no_pair_surface_on_cluster(clone, fixture_dir);
    assert_eq!(
        signals::has_verbatim_pair(&scan_root, clone)?,
        byte_identical,
        "{fixture_dir}: the fixture bytes determine whether the clone is a \
         verbatim copy (identical) or a byte-distinct rename — the report \
         must carry the clone either way, and the byte truth must match: \
         {report:#}"
    );
    Ok(())
}

/// Every visible cluster as `id mass=N [files]`, in report order — the
/// whole published surface as one comparable list. Two suites pinning
/// that an operator change (or a rename with no anchor) publishes
/// nothing carried byte-identical copies of this rendering.
pub(crate) fn published_with_mass(report: &Value) -> Vec<String> {
    clusters(report)
        .iter()
        .map(|cluster| {
            format!(
                "{id} mass={mass} {files:?}",
                id = cluster_id(cluster),
                mass = field(cluster, "mass").as_u64().unwrap_or(0),
                files = occurrence_files(cluster),
            )
        })
        .collect()
}

/// Every visible cluster as `(rank, id, mass)`, sorted — the ranking a
/// report commits to ([RANK-MASS-SUM]), in a form two suites compare
/// against an expected order.
pub(crate) fn rankable(report: &Value) -> Vec<(u64, &str, u64)> {
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

/// The occurrence texts of every cluster that quotes `needle`, so a suite
/// can assert on what the report actually published for a marker rather
/// than on cluster identity.
///
/// # Errors
///
/// Returns an error when an occurrence's source slice cannot be read.
pub(crate) fn clusters_touching(
    report: &Value,
    scan_root: &Path,
    needle: &str,
) -> Result<Vec<Vec<String>>> {
    let mut hits = Vec::new();
    for cluster in clusters(report) {
        let texts = occurrence_texts(scan_root, cluster)?;
        if texts.iter().any(|text| text.contains(needle)) {
            hits.push(texts);
        }
    }
    Ok(hits)
}

/// The first visible cluster whose occurrences reach every path in
/// `sides`, or an error carrying `missing` — the shape both rename pins
/// look for, where absence is the false negative they exist to catch.
///
/// # Errors
///
/// Returns an error spelled `missing` when no visible cluster spans them.
pub(crate) fn cluster_spanning_sides<'a>(
    report: &'a Value,
    sides: &[&str],
    missing: &str,
) -> Result<&'a Value> {
    clusters(report)
        .iter()
        .find(|cluster| {
            sides.iter().all(|side| {
                occurrences(cluster).iter().any(|occurrence| {
                    field(occurrence, "path")
                        .as_str()
                        .unwrap_or_default()
                        .ends_with(side)
                })
            })
        })
        .ok_or_else(|| anyhow!("{missing}"))
}

/// The report's `clusters_hidden` count (suppressed-cluster telemetry), or
/// `0` when absent.
pub(crate) fn clusters_hidden(report: &Value) -> u64 {
    field(report, "clusters_hidden").as_u64().unwrap_or(0)
}

/// The relative path of a single occurrence, erroring when absent.
pub(crate) fn occurrence_path(occurrence: &Value) -> Result<&str> {
    field(occurrence, "path")
        .as_str()
        .ok_or_else(|| anyhow!("reported occurrence is missing path"))
}

/// True when an occurrence is rendered but marked hidden — present in a
/// cluster's `size`, absent from the report a human reads and from every
/// line metric. A count alone cannot see the difference, so assertions
/// that care about *shown* occurrences ask this instead.
pub(crate) fn occurrence_is_hidden(occurrence: &Value) -> bool {
    field(occurrence, "hidden").as_bool().unwrap_or(false)
}

/// The relative paths of every occurrence in `cluster`, in report order.
pub(crate) fn occurrence_paths(cluster: &Value) -> Vec<String> {
    occurrences(cluster)
        .iter()
        .filter_map(|occurrence| field(occurrence, "path").as_str().map(ToOwned::to_owned))
        .collect()
}

/// The bare file names (no directory) of every occurrence in `cluster`.
pub(crate) fn occurrence_files(cluster: &Value) -> Vec<String> {
    occurrences(cluster)
        .iter()
        .filter_map(|occurrence| {
            field(occurrence, "path")
                .as_str()
                .and_then(|path| Path::new(path).file_name())
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect()
}

/// The source slice each occurrence in `cluster` points at, in report order.
pub(crate) fn occurrence_texts(scan_root: &Path, cluster: &Value) -> Result<Vec<String>> {
    occurrences(cluster)
        .iter()
        .map(|occurrence| occurrence_text(scan_root, occurrence))
        .collect()
}

/// Reads the source slice an occurrence points at, resolving its `path`
/// relative to `scan_root` and slicing `[start_byte, end_byte)`.
pub(crate) fn occurrence_text(scan_root: &Path, occurrence: &Value) -> Result<String> {
    let path = occurrence_path(occurrence)?;
    let source = fs::read_to_string(scan_root.join(path))?;
    let start = occurrence_byte(occurrence, "start_byte")?;
    let end = occurrence_byte(occurrence, "end_byte")?;
    source
        .get(start..end)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("reported occurrence range is invalid"))
}

/// Reads a `usize` byte-offset field (`start_byte` / `end_byte`) off an
/// occurrence.
pub(crate) fn occurrence_byte(occurrence: &Value, field: &str) -> Result<usize> {
    occurrence
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| anyhow!("reported occurrence is missing {field}"))
}

/// Collapses every reported occurrence whose source text satisfies
/// `predicate` into a one-line summary. Tests assert the returned list is
/// empty to prove a benign pattern was not surfaced as a duplicate.
pub(crate) fn summaries_where(
    report: &Value,
    scan_root: &Path,
    predicate: impl Fn(&str) -> bool,
) -> Result<Vec<String>> {
    let mut summaries = Vec::new();
    for cluster in clusters(report) {
        for occurrence in occurrences(cluster) {
            let text = occurrence_text(scan_root, occurrence)?;
            if predicate(&text) {
                summaries.push(text.replace('\n', " "));
            }
        }
    }
    Ok(summaries)
}

/// Reads a scalar field off the report's `metrics` block ([METRICS-REPO]).
pub(crate) fn metric_field<'a>(report: &'a Value, name: &str) -> &'a Value {
    field(field(report, "metrics"), name)
}

/// `metrics.per_file` rows, or an empty slice when absent.
pub(crate) fn per_file_metrics(report: &Value) -> &[Value] {
    metric_field(report, "per_file")
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// Line-set cardinality as the `u64` the wire metric uses.
pub(crate) fn line_count(lines: &BTreeSet<u64>) -> u64 {
    u64::try_from(lines.len()).unwrap_or(u64::MAX)
}

/// [METRICS-REPO] Source lines covered by visible clones, excluding informational matches.
pub(crate) fn visible_duplicated_lines(report: &Value) -> BTreeMap<String, BTreeSet<u64>> {
    let mut per_file: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
    for cluster in clone_findings(report) {
        for occurrence in occurrences(&cluster) {
            if occurrence_is_hidden(occurrence) {
                continue;
            }
            let Some(path) = field(occurrence, "path").as_str() else {
                continue;
            };
            let start = field(occurrence, "start_line").as_u64().unwrap_or(0);
            let end = field(occurrence, "end_line").as_u64().unwrap_or(0);
            let entry = per_file.entry(path.to_owned()).or_default();
            for line in start..=end {
                let _inserted = entry.insert(line);
            }
        }
    }
    per_file
}

/// Total lines the report's visible clusters cover across every file — the
/// repo-level `duplicated_loc` the metric must report ([METRICS-REPO]).
pub(crate) fn visible_duplicated_loc(report: &Value) -> u64 {
    visible_duplicated_lines(report)
        .values()
        .map(line_count)
        .sum()
}

/// One `id [bucket] path Lstart-end, ...` line per visible cluster — the
/// complete published surface a full-set regression asserts against, so a
/// mis-scoped view cannot hide behind a marker-based spot check.
pub(crate) fn visible_cluster_lines(report: &Value) -> Vec<String> {
    clusters(report)
        .iter()
        .map(|cluster| {
            let spans: Vec<String> = occurrences(cluster)
                .iter()
                .map(|occurrence| {
                    format!(
                        "{} L{}-{}",
                        field(occurrence, "path").as_str().unwrap_or("?"),
                        field(occurrence, "start_line").as_u64().unwrap_or(0),
                        field(occurrence, "end_line").as_u64().unwrap_or(0),
                    )
                })
                .collect();
            format!(
                "{} [{}] {}",
                field(cluster, "id").as_str().unwrap_or("?"),
                cluster_kind(cluster),
                spans.join(", ")
            )
        })
        .collect()
}

/// [PIPELINE-CLUSTER-EXACT-SCOPE] The Go authored-window contract, shared
/// by every Go suite so a padded occurrence is caught wherever it appears
/// rather than only in the fixture that first exposed it. Imported
/// explicitly with `use crate::common::go_scope::*;`, for the same reason
/// as `signals`.
pub(crate) mod go_scope;

/// Asserts the cluster's occurrences are exactly `spans` in `file`, in line
/// order. Anything wider has lumped in code that is not duplicated;
/// anything narrower has published a fragment of the authored declaration
/// instead of the declaration ([PIPELINE-CLUSTER-EXACT-SCOPE]).
pub(crate) fn assert_occurrence_extents(
    cluster: &Value,
    file: &str,
    spans: &[RangeInclusive<u64>],
) -> Result<()> {
    let mut extents: Vec<(String, u64, u64)> = occurrences(cluster)
        .iter()
        .map(|occurrence| {
            Ok((
                occurrence_path(occurrence)?.to_owned(),
                field(occurrence, "start_line")
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("start_line missing: {occurrence:#}"))?,
                field(occurrence, "end_line")
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("end_line missing: {occurrence:#}"))?,
            ))
        })
        .collect::<Result<_>>()?;
    extents.sort();
    let expected: Vec<(String, u64, u64)> = spans
        .iter()
        .map(|lines| (file.to_owned(), *lines.start(), *lines.end()))
        .collect();
    assert_eq!(
        extents, expected,
        "each occurrence is the authored declaration, never its container \
         and never a fragment of it: {cluster:#}"
    );
    Ok(())
}
