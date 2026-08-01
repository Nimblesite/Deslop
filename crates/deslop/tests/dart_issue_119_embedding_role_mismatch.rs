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

use std::{fs, path::Path};

use anyhow::Result;
use mock_ollama::MockOllama;
use serde_json::Value;

mod common;
use crate::common::*;

/// Runs the CLI against a private copy of `fixture_root` with the
/// deterministic mock Ollama
/// wired in via `--embeddings required`, returning the parsed JSON.
fn run_report(fixture_root: &Path) -> Result<Value> {
    let server = MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    // `--embeddings required` writes `.deslop/cache/embeddings/` into the
    // *scan root* ([OUTPUT-DIR]); `--no-incremental` gates only the
    // fingerprint layer. Scan a copy so the fixture stays pristine.
    let scan_root = &tmp.path().join("src");
    seed(fixture_root, scan_root)?;
    let mut cmd = deslop_cmd(scan_root, &output)?;
    let _assertion = cmd
        .args([
            "--min-nodes",
            "5",
            "--embeddings",
            "required",
            "--embedding-provider",
            "ollama",
            "--embedding-model",
            "nomic-embed-text",
            "--embedding-endpoint",
            server.endpoint(),
        ])
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

fn bucket(cluster: &Value) -> &str {
    cluster.get("bucket").and_then(Value::as_str).unwrap_or("")
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
