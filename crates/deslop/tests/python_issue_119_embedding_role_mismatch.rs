//! E2E regression for GH #119 [CLONE-NOISE-EMBEDDING-ROLE-MISMATCH].
//!
//! The embedding pass can pair two snippets that share a topic
//! vocabulary but live in structurally incompatible constructs — a
//! reusable helper *class* and a constructor-storage *test method*.
//! Such a pair reaches `structural=0.00`, `embedding_cos>=0.80`, and
//! surfaces as "Same behavior, different code" even though a class
//! definition and a function/method have no safe shared extraction.
//!
//! The fix requires an embedding-dominant `same_behavior` cluster to be
//! role/context compatible (all classes, or all functions) before
//! surfacing. A class-def paired with a function/method is suppressed.
//! Two genuinely behaviour-equivalent functions (same role) that the
//! embedding correctly pairs must still surface.
//!
//! Determinism: the in-process [`MockOllama`] embeds each snippet to a
//! 4-lane vector seeded by its byte length and first byte, so two
//! snippets of equal length give cosine ~= 1.0. The fixtures are tuned
//! so the cross-role and same-role pairs both clear the embedding gate
//! while structural overlap stays at zero.

#[path = "cli/mock_ollama.rs"]
mod mock_ollama;

use std::{fs, path::Path};

use anyhow::Result;
use assert_cmd::Command;
use mock_ollama::MockOllama;
use serde_json::Value;

mod common;
use crate::common::*;

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

/// Returns visible clusters that pair the helper class with the test
/// function. Either occurrence covering the class keyword and another
/// covering the test function body is the role-mismatch signature.
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

// GH #119 acceptance: an embedding-dominant pair whose members have
// different top-level roles (a `class` definition and a function body)
// must NOT surface. The cluster is suppressed into `clusters_hidden`.
#[test]
fn class_function_role_mismatch_does_not_surface() -> Result<()> {
    let scan_root = fixture("python-issue-119-role-mismatch");
    let report = run_report(&scan_root)?;
    let offenders = class_function_role_pairs(&report, &scan_root)?;
    assert!(
        offenders.is_empty(),
        "a helper class paired with a test function by the embedding pass \
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
        "no same_behavior cluster may remain visible for the role-mismatch \
         fixture: {:#?}",
        clusters(&report)
    );
    Ok(())
}

// GH #119 guard against over-suppression: two genuinely behaviour-
// equivalent FUNCTIONS (recursive vs iterative sum) that the embedding
// pass pairs share one top-level role, so the role gate must NOT hide
// them. They must still surface as "Same behavior, different code".
#[test]
fn same_role_function_pair_still_surfaces() -> Result<()> {
    let scan_root = fixture("python-issue-119-same-role");
    let report = run_report(&scan_root)?;
    let same_behavior: Vec<&Value> = clusters(&report)
        .iter()
        .filter(|cluster| bucket(cluster) == "same_behavior")
        .collect();
    assert!(
        !same_behavior.is_empty(),
        "two same-role behaviour-equivalent functions must still surface as \
         same_behavior — the role gate must not over-suppress: {:#?}",
        clusters(&report)
    );
    let pairs_both_functions = same_behavior.iter().try_fold(false, |found, cluster| {
        let texts = occurrence_texts(&scan_root, cluster)?;
        let touches_recursive = texts.iter().any(|text| text.contains("total_recursive"));
        let touches_iterative = texts.iter().any(|text| text.contains("while index"));
        Ok::<bool, anyhow::Error>(found || (touches_recursive && touches_iterative))
    })?;
    assert!(
        pairs_both_functions,
        "the surviving same_behavior cluster must pair the recursive and \
         iterative functions: {same_behavior:#?}"
    );
    Ok(())
}
