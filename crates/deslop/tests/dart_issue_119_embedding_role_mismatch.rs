//! E2E regression for GH #119 [CLONE-NOISE-EMBEDDING-ROLE-MISMATCH] on
//! Dart ([LANG-CAND-DART]).
//!
//! The embedding pass can pair two Dart snippets that share a topic
//! vocabulary but live in structurally incompatible constructs — a
//! `class` definition and a top-level function. Such a pair reaches
//! `structural=0.00`, `embedding_cos>=0.80`, and would surface as "Same
//! behavior, different code" even though a class and a function have no
//! safe shared extraction.
//!
//! This proves the role-compatibility gate is wired for Dart's grammar:
//! the gate re-parses each member and resolves its enclosing construct via
//! the Dart `class_declaration` / `function_declaration` node kinds. Dart
//! previously bypassed every re-parse filter (`grammar_for` had no Dart
//! arm), so this gate could never engage. A class-def paired with a
//! function is now suppressed, while two genuinely behaviour-equivalent
//! Dart functions (same role) still surface.
//!
//! Determinism: the in-process [`MockOllama`] embeds each snippet to a
//! 4-lane vector seeded by its byte length and first byte, so the
//! cross-role and same-role pairs both clear the embedding gate while
//! structural overlap stays at zero.

#[path = "cli/mock_ollama.rs"]
mod mock_ollama;

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use assert_cmd::Command;
use mock_ollama::MockOllama;
use serde_json::Value;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Runs the CLI against `scan_root` with the deterministic mock Ollama
/// wired in via `--embeddings required`, returning the parsed JSON.
fn run_report(scan_root: &Path) -> Result<Value> {
    let server = MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = Command::cargo_bin("deslop")?
        .arg(scan_root)
        .arg("--min-nodes")
        .arg("5")
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-provider")
        .arg("ollama")
        .arg("--embedding-model")
        .arg("nomic-embed-text")
        .arg("--embedding-endpoint")
        .arg(server.endpoint())
        .arg("--output")
        .arg(&output)
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

fn clusters(report: &Value) -> &[Value] {
    report
        .get("clusters")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn bucket(cluster: &Value) -> &str {
    cluster.get("bucket").and_then(Value::as_str).unwrap_or("")
}

fn clusters_hidden(report: &Value) -> u64 {
    report
        .get("clusters_hidden")
        .and_then(Value::as_u64)
        .unwrap_or_default()
}

/// Collects the raw source text of every occurrence in `cluster`.
fn occurrence_texts(scan_root: &Path, cluster: &Value) -> Result<Vec<String>> {
    let occurrences = cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    occurrences
        .iter()
        .map(|occurrence| occurrence_text(scan_root, occurrence))
        .collect()
}

fn occurrence_text(scan_root: &Path, occurrence: &Value) -> Result<String> {
    let path = occurrence
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("occurrence missing path"))?;
    let source = fs::read_to_string(scan_root.join(path))?;
    let start = occurrence_byte(occurrence, "start_byte")?;
    let end = occurrence_byte(occurrence, "end_byte")?;
    source
        .get(start..end)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("occurrence range invalid"))
}

fn occurrence_byte(occurrence: &Value, field: &str) -> Result<usize> {
    occurrence
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| anyhow!("occurrence missing {field}"))
}

/// Returns visible clusters that pair the Dart class with the top-level
/// function. An occurrence covering a class field (`alpha = 0`) plus
/// another covering the function body (`saved.bind`) is the role-mismatch
/// signature.
fn class_function_role_pairs(report: &Value, scan_root: &Path) -> Result<Vec<Vec<String>>> {
    let mut offenders = Vec::new();
    for cluster in clusters(report) {
        let texts = occurrence_texts(scan_root, cluster)?;
        let touches_class = texts.iter().any(|text| text.contains("alpha = 0"));
        let touches_function = texts.iter().any(|text| text.contains("saved.bind"));
        if touches_class && touches_function {
            offenders.push(texts);
        }
    }
    Ok(offenders)
}

// GH #119 acceptance on Dart: an embedding-dominant pair whose members
// have different top-level roles (a Dart `class` definition and a
// top-level function body) must NOT surface. The cluster is suppressed
// into `clusters_hidden`.
#[test]
fn dart_class_function_role_mismatch_does_not_surface() -> Result<()> {
    let scan_root = fixture("dart-issue-119-role-mismatch");
    let report = run_report(&scan_root)?;
    let offenders = class_function_role_pairs(&report, &scan_root)?;
    assert!(
        offenders.is_empty(),
        "a Dart class paired with a top-level function by the embedding pass \
         must not surface as duplication — there is no safe cross-role \
         extraction: {offenders:#?}"
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the role-incompatible embedding pair must be counted in \
         clusters_hidden, got {}",
        clusters_hidden(&report)
    );
    assert!(
        clusters(&report)
            .iter()
            .all(|cluster| bucket(cluster) != "same_behavior"),
        "no same_behavior cluster may remain visible for the Dart role-mismatch \
         fixture: {:#?}",
        clusters(&report)
    );
    Ok(())
}

// GH #119 guard against over-suppression on Dart: two genuinely
// behaviour-equivalent FUNCTIONS (recursive vs iterative sum) that the
// embedding pass pairs share one top-level role, so the role gate must
// NOT hide them. They must still surface as "Same behavior, different
// code".
#[test]
fn dart_same_role_function_pair_still_surfaces() -> Result<()> {
    let scan_root = fixture("dart-issue-119-same-role");
    let report = run_report(&scan_root)?;
    let same_behavior: Vec<&Value> = clusters(&report)
        .iter()
        .filter(|cluster| bucket(cluster) == "same_behavior")
        .collect();
    assert!(
        !same_behavior.is_empty(),
        "two same-role behaviour-equivalent Dart functions must still surface as \
         same_behavior — the role gate must not over-suppress: {:#?}",
        clusters(&report)
    );
    let pairs_both_functions = same_behavior.iter().try_fold(false, |found, cluster| {
        let texts = occurrence_texts(&scan_root, cluster)?;
        let touches_recursive = texts.iter().any(|text| text.contains("totalRecursive"));
        let touches_iterative = texts.iter().any(|text| text.contains("while (index"));
        Ok::<bool, anyhow::Error>(found || (touches_recursive && touches_iterative))
    })?;
    assert!(
        pairs_both_functions,
        "the surviving same_behavior cluster must pair the recursive and \
         iterative Dart functions: {same_behavior:#?}"
    );
    Ok(())
}
