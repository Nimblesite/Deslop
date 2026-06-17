//! End-to-end signature/detection tests for the Dart language plug-in
//! ([LANG-CAND-DART], [PIPELINE-LANG-TRAIT]). Mirrors the Python suite in
//! [`python_signatures`]: it drives the `deslop` binary as a black box
//! against Dart fixtures and asserts on the rendered JSON report.
//!
//! These prove the full pipeline works for Dart end to end:
//!   - Type-2 renamed clones reach `structural = 1.0` AND
//!     `token_jaccard = 1.0` (identical k-gram sets after Dart
//!     normalisation collapse identifiers/literals).
//!   - A whole-function near-miss still produces a genuine cross-file
//!     cluster with `structural = 1.0` on the shared sub-structures —
//!     proving the Dart structural fingerprint path detects Type-3
//!     near-misses across files, while the signature-only sibling match
//!     is correctly suppressed ([CLONE-NOISE-SIGNATURE-ONLY], #154).
//!   - `token_jaccard` is bit-identical across process restarts
//!     (deterministic signatures).
//!   - False-positive regression guards proving the re-parse filter
//!     subsystem is fully wired for Dart — Dart previously bypassed it
//!     entirely because `grammar_for` had no Dart arm, so every member
//!     re-parse returned `None` and no filter could fire. These guards
//!     prove each filter now fires on Dart's real CST: generated
//!     `*.g.dart`/`*.freezed.dart` self-duplication is hidden (#95) while
//!     hand-written clones still surface, `export`/`import` barrels are
//!     not flagged (#96/#150/#155), signature-only structural matches are
//!     suppressed (#154), polymorphic method overrides sharing one name
//!     are suppressed (#69), and calls varying only in string-literal
//!     arguments are suppressed (#70) — each paired with a genuine clone
//!     that must still surface, proving the suppression stays targeted.

use std::{fs, path::Path};

use anyhow::Result;
use assert_cmd::Command;

mod common;
use crate::common::*;

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

// [FUSION-SIGNALS-THREE-LAYER] Two Dart functions sharing control-flow
// sub-structure (`if (_ < _) { return _; }`, `for (...) { _ = _ + _; }`)
// but differing in body length are a genuine Type-3 near-miss. The shared
// subtrees must surface as a cross-file cluster with `structural = 1.0`,
// proving the Dart structural fingerprint path detects near-misses across
// files.
//
// delta.dart: accumulate() runs `running + step` AND `running + 2` per
// iteration. epsilon.dart: aggregate() runs only `accumulator + cursor`.
// The signature-only sibling match — the two `int f(int)` headers, whose
// bodies differ — is a known false positive ([CLONE-NOISE-SIGNATURE-ONLY],
// #154) and is correctly suppressed; it must NOT be what carries the
// cluster, so we require a genuine `structural = 1.0` body match.
#[test]
fn dart_near_miss_produces_genuine_cross_file_structural_cluster() -> Result<()> {
    let report = run_cli("dart-type3", 8)?;
    let clusters = clusters(&report);
    let cross_file = clusters
        .iter()
        .find(|cluster| spans_both(cluster, "delta.dart", "epsilon.dart"));
    let Some(cluster) = cross_file else {
        anyhow::bail!(
            "dart-type3 must produce a cross-file cluster spanning delta.dart and \
             epsilon.dart; got clusters: {clusters:#?}"
        );
    };
    let structural = signal(cluster, "structural");
    assert!(
        is_exact_one(structural),
        "the cross-file Dart near-miss cluster must reach structural = 1.0 on the shared \
         subtree (genuine Type-3 detection via the structural path), got {structural}",
    );
    let occurrences = cluster
        .pointer("/occurrences")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    assert!(
        occurrences >= 2,
        "a clone cluster must have at least two occurrences, got {occurrences}",
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

/// True when any cluster in the report spans both named files. Hidden
/// clusters are dropped before serialisation, so every cluster here is
/// one a human is actually shown.
fn any_cluster_spans(report: &serde_json::Value, left: &str, right: &str) -> bool {
    clusters(report)
        .iter()
        .any(|cluster| spans_both(cluster, left, right))
}

/// Count of clusters the renderer analysed but hid from the ranked report.
fn clusters_hidden(report: &serde_json::Value) -> u64 {
    report
        .pointer("/clusters_hidden")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
}

// [EXCLUSION-CONFIG] #95 — Dart code generators (`*.g.dart`,
// `*.freezed.dart`, …) emit near-identical serialisation blocks that
// self-duplicate across every annotated type. They must be hidden from the
// ranked report (still analysed, never surfaced). The suppression must stay
// targeted: `dart-generated-files` pairs two identical `.g.dart` files with
// two identical hand-written parsers — only the generated pair may vanish.
#[test]
fn dart_generated_files_are_hidden_but_handwritten_clones_surface() -> Result<()> {
    let report = run_cli("dart-generated-files", 10)?;
    assert!(
        !any_cluster_spans(&report, "serializers.g.dart", "models.g.dart"),
        "generated `.g.dart` files must not surface as a ranked duplicate cluster",
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the generated-file cluster must be actively hidden, not merely absent",
    );
    assert!(
        any_cluster_spans(&report, "parser_alpha.dart", "parser_beta.dart"),
        "hand-written duplicates must still surface even when generated files are hidden",
    );
    Ok(())
}

// [PIPELINE-BOILERPLATE-FILTER] #96 / #150 / #155 — Dart `export`/`import`
// barrel files are top-level scaffolding, never duplicate logic. Two
// export-only barrels share the identical directive shape but must never be
// reported as a clone of one another.
#[test]
fn dart_export_barrels_are_not_flagged_as_duplicates() -> Result<()> {
    let report = run_cli("dart-export-barrel", 8)?;
    assert!(
        !any_cluster_spans(&report, "widgets.dart", "models.dart"),
        "Dart export barrels are import scaffolding and must not cluster as duplicates",
    );
    Ok(())
}

// [CLONE-NOISE-SIGNATURE-ONLY] #154 — after identifier/literal
// normalisation two functions with the same parameter shape collapse to the
// same signature even when their bodies are unrelated. Such a signature-only
// match (bodies differ in raw bytes) is a false positive and must be
// suppressed. `dart-signature-only` shares the header
// `int computeScore(Map<String, int>, List<String>)` across two functions
// with entirely different bodies.
#[test]
fn dart_signature_only_match_with_differing_bodies_is_suppressed() -> Result<()> {
    let report = run_cli("dart-signature-only", 8)?;
    assert!(
        !any_cluster_spans(&report, "alpha.dart", "beta.dart"),
        "a signature-only structural match with differing bodies must be suppressed for Dart",
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the signature-only match must be actively suppressed (#154), not merely absent",
    );
    Ok(())
}

// [EXCLUSION-CONFIG] #95 — generators that emit no stable file suffix
// (ffigen/jnigen name FFI output `*_bindings.dart`) are still recognised by
// the machine-generated banner in the file head and hidden. Both fixture
// files are byte-identical generated code carrying `AUTO GENERATED FILE,
// DO NOT EDIT.` but no `.g.dart` suffix, so only the header check can hide
// them — exactly the dart-lang/http FFI-binding case from the real-repo sweep.
#[test]
fn dart_generated_header_files_are_hidden_without_a_suffix() -> Result<()> {
    let report = run_cli("dart-generated-header", 8)?;
    assert!(
        !any_cluster_spans(
            &report,
            "native_alpha_bindings.dart",
            "native_beta_bindings.dart"
        ),
        "banner-marked generated files (no `.g.dart` suffix) must still be hidden",
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the banner-marked generated clone must be actively hidden, not merely absent",
    );
    Ok(())
}

// #69 — the same method name implemented across several concrete
// subclasses of one abstract `Shape` clusters on its identical
// signature/shape after Type-2 normalisation, but each override has a
// genuinely different body. That is the polymorphic-implementation
// pattern, not extractable duplication. The three `measure` bodies share
// AST *shape* (only identifiers/literals differ), so the signature-only
// filter (#154) deliberately does NOT fire here — only the polymorphic
// filter (#69) can suppress this, and only once `enclosing_function_name`
// resolves Dart's nested `signature → function_signature → name`. A
// genuinely copy-pasted top-level `loadSettings` (byte-identical bodies)
// must still surface, proving the suppression is targeted.
#[test]
fn dart_polymorphic_override_signatures_are_suppressed() -> Result<()> {
    let report = run_cli("dart-issue-69-polymorphic", 8)?;
    let measure_pairs = [
        ("circle.dart", "square.dart"),
        ("circle.dart", "triangle.dart"),
        ("square.dart", "triangle.dart"),
    ];
    for (left, right) in measure_pairs {
        assert!(
            !any_cluster_spans(&report, left, right),
            "polymorphic `measure` overrides ({left} / {right}) must not surface as \
             duplication (#69)",
        );
    }
    assert!(
        clusters_hidden(&report) >= 1,
        "the polymorphic-signature cluster must be actively hidden (#69), not merely absent",
    );
    assert!(
        any_cluster_spans(&report, "repo_alpha.dart", "repo_beta.dart"),
        "a genuine byte-identical clone must still surface even when polymorphic \
         overrides are suppressed",
    );
    Ok(())
}

// #70 — `recordEvent("user_login", {...}, "evt-001")` and its sibling
// differ only in their string-literal arguments. The call-shape clusters
// across files after normalisation, but varying test data is not
// duplication. The enclosing functions are differently named (so #69
// cannot fire) and the matched range is a call body, not a signature (so
// #154 cannot fire), so only the literal-variation call filter — which
// re-parses via the now-wired Dart grammar and reads the `call_expression`
// `function`/`arguments` fields — can suppress it. A byte-identical
// `summarize` clone must still surface.
#[test]
fn dart_literal_variation_calls_are_suppressed() -> Result<()> {
    let report = run_cli("dart-issue-70-test-data", 8)?;
    assert!(
        !any_cluster_spans(&report, "events_alpha.dart", "events_beta.dart"),
        "calls varying only in string-literal arguments must not surface (#70)",
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the literal-variation call cluster must be actively hidden (#70)",
    );
    assert!(
        any_cluster_spans(&report, "summary_alpha.dart", "summary_beta.dart"),
        "a genuine byte-identical clone must still surface alongside the suppressed \
         literal-variation cluster",
    );
    Ok(())
}
