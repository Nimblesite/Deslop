//! End-to-end signature/detection tests for the Dart language plug-in
//! ([LANG-CAND-DART], [PIPELINE-LANG-TRAIT]). Mirrors the Python suite in
//! [`python_signatures`]: it drives the `deslop` binary as a black box
//! against Dart fixtures and asserts on the rendered JSON report.
//!
//! These prove the full pipeline works for Dart end to end:
//!   - Type-2 renamed clones reach `structural = 1.0` AND
//!     `token_jaccard = 1.0` (identical k-gram sets after Dart
//!     normalisation collapse identifiers/literals).
//!   - A whole-function near-miss still produces a cross-file cluster
//!     with `token_jaccard > 0` via the shared sub-structures — proving
//!     the `MinHash` signature path is wired for Dart tokens.
//!   - `token_jaccard` is bit-identical across process restarts
//!     (deterministic signatures).

use std::{fs, path::Path, path::PathBuf};

use anyhow::Result;
use assert_cmd::Command;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn run_cli(fixture_name: &str, min_nodes: u32) -> Result<serde_json::Value> {
    let tmp = tempfile::tempdir()?;
    let out = tmp.path().join("report.json");
    let _assertion = Command::cargo_bin("deslop")?
        .arg(fixture(fixture_name))
        .arg("--min-nodes")
        .arg(min_nodes.to_string())
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = fs::read_to_string(&out)?;
    Ok(serde_json::from_str(&json)?)
}

fn clusters(report: &serde_json::Value) -> Vec<&serde_json::Value> {
    report
        .pointer("/clusters")
        .and_then(serde_json::Value::as_array)
        .map(|values| values.iter().collect())
        .unwrap_or_default()
}

fn signal(cluster: &serde_json::Value, key: &str) -> f64 {
    cluster
        .pointer(&format!("/signals/{key}"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(f64::NAN)
}

fn occurrence_files(cluster: &serde_json::Value) -> Vec<String> {
    cluster
        .pointer("/occurrences")
        .and_then(serde_json::Value::as_array)
        .map(|occ| {
            occ.iter()
                .filter_map(|occurrence| {
                    occurrence
                        .get("path")
                        .and_then(serde_json::Value::as_str)
                        .map(|path| {
                            Path::new(path).file_name().map_or_else(
                                || path.to_owned(),
                                |name| name.to_string_lossy().into_owned(),
                            )
                        })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn is_exact_one(value: f64) -> bool {
    (value - 1.0).abs() <= f64::EPSILON
}

fn spans_both(cluster: &serde_json::Value, left: &str, right: &str) -> bool {
    let files: std::collections::BTreeSet<String> = occurrence_files(cluster).into_iter().collect();
    files.contains(left) && files.contains(right)
}

// [FUSION-SIGNALS-THREE-LAYER] Type-2 Dart clones (identical after
// normalisation, every identifier renamed) must produce both
// `structural = 1.0` and `token_jaccard = 1.0` — the structural pass
// proves the Merkle hashes match and the MinHash pass proves identical
// k-gram sets map to identical signatures.
#[test]
fn dart_type2_clone_has_structural_and_token_jaccard_of_one() -> Result<()> {
    let report = run_cli("dart-small", 10)?;
    let clusters = clusters(&report);
    assert!(
        !clusters.is_empty(),
        "dart-small must produce at least one cluster",
    );
    let top = clusters
        .first()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("dart-small must produce at least one cluster"))?;
    let structural = signal(top, "structural");
    let token_jaccard = signal(top, "token_jaccard");
    assert!(
        is_exact_one(structural),
        "Type-2 Dart clone must have structural = 1.0, got {structural}",
    );
    assert!(
        is_exact_one(token_jaccard),
        "Type-2 Dart clone must have token_jaccard = 1.0 (identical k-gram sets), \
         got {token_jaccard}",
    );
    assert!(
        spans_both(top, "alpha.dart", "beta.dart"),
        "the Type-2 cluster must span both alpha.dart and beta.dart",
    );
    Ok(())
}

// [FUSION-SIGNALS-THREE-LAYER] Two Dart functions sharing structural
// subtrees (`_ = _ + _`, `if (_ < _) return _;`) must produce a
// cross-file cluster with `token_jaccard > 0`.
//
// delta.dart: accumulate() runs `running + step` AND `running + 2` per
// iteration. epsilon.dart: aggregate() runs only `accumulator + cursor`.
// Despite the whole-function structural difference, the shared
// sub-structures are compared via the Dart MinHash signature path — a
// broken signature pipeline (garbage output or wrong per-file tree)
// would collapse the cross-file cluster or zero its Jaccard.
#[test]
fn dart_multi_file_corpus_produces_cross_file_cluster_with_positive_token_jaccard() -> Result<()> {
    let report = run_cli("dart-type3", 8)?;
    let clusters = clusters(&report);
    let cross_file = clusters
        .iter()
        .find(|cluster| spans_both(cluster, "delta.dart", "epsilon.dart") && signal(cluster, "token_jaccard") > 0.0);
    let Some(cluster) = cross_file else {
        anyhow::bail!(
            "dart-type3 must produce a cross-file cluster spanning delta.dart and epsilon.dart \
             with token_jaccard > 0; got clusters: {clusters:#?}"
        );
    };
    let token_jaccard = signal(cluster, "token_jaccard");
    assert!(
        token_jaccard > 0.0,
        "cross-file Dart cluster must have token_jaccard > 0.0 (the MinHash signature path \
         must produce meaningful signatures for shared Dart subtrees), got {token_jaccard}",
    );
    Ok(())
}

// Zero-false-positive guard: two structurally unrelated Dart functions
// (`tally()` map-building loop vs `describe()` if-cascade) must never
// share a cluster. Every cluster's occurrences must come from a single
// file — a human reading the report must not be told they are duplicates.
#[test]
fn dissimilar_dart_functions_never_form_a_cross_file_cluster() -> Result<()> {
    let report = run_cli("dart-dissimilar-functions", 8)?;
    for cluster in clusters(&report) {
        let files: std::collections::BTreeSet<String> =
            occurrence_files(cluster).into_iter().collect();
        assert!(
            files.len() <= 1,
            "dissimilar Dart functions must not cluster across files; got files {files:?}",
        );
    }
    Ok(())
}

// [PIPELINE-DETERMINISM] Two CLI runs over the same Dart corpus must
// produce bit-identical `token_jaccard` values — proves the MinHash
// (blake3 XOF) signature path is deterministic across process restarts.
#[test]
fn dart_token_jaccard_is_deterministic_across_runs() -> Result<()> {
    let run1 = run_cli("dart-small", 10)?;
    let run2 = run_cli("dart-small", 10)?;
    let jaccards1: Vec<u64> = clusters(&run1)
        .iter()
        .map(|cluster| signal(cluster, "token_jaccard").to_bits())
        .collect();
    let jaccards2: Vec<u64> = clusters(&run2)
        .iter()
        .map(|cluster| signal(cluster, "token_jaccard").to_bits())
        .collect();
    assert!(
        !jaccards1.is_empty(),
        "dart-small must produce at least one cluster",
    );
    assert_eq!(
        jaccards1, jaccards2,
        "token_jaccard values must be bit-identical across runs on the same Dart corpus",
    );
    Ok(())
}
