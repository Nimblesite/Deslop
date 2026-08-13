//! [FUSION-STRATEGY-BOUNDED-MAX] / [PIPELINE-DETERMINISM]: bounded fused
//! confidence and reproducible snapshots across embedding refreshes.
//! Drives the real `deslop-lsp` binary over stdio; no pipeline internals
//! are called.

#[path = "../../deslop/tests/cli/mock_ollama.rs"]
mod mock_ollama;

mod common;

use std::{
    io::BufReader,
    process::{ChildStdin, ChildStdout},
};

use anyhow::{anyhow, Result};
use common::{
    at, call, handshake, path as json_path,
    reports::{
        assert_initialize_contract, assert_report_shell, assert_report_shell_without_clusters,
        cluster_paths, copy_fixture_files, has_suffix, report_clusters, signal, wait_for_report,
    },
    spawn_lsp_guarded,
};
use mock_ollama::MockOllama;
use serde_json::{json, Value};

const SET_MODEL: &str = "deslop/embeddingSetModel";
const LEDGER_FILES: [&str; 2] = ["ledger_a.ts", "ledger_c.ts"];

/// [FUSION-STRATEGY-BOUNDED-MAX] / [PIPELINE-DETERMINISM]: selecting a model
/// through the editor-facing LSP method must expose honest bounded scores,
/// and two full embedding refreshes over unchanged files must produce the
/// same ordered clusters, identifiers, metrics, and signals.
#[test]
fn lsp_embedding_refresh_is_bounded_and_reproducible() -> Result<()> {
    let server = MockOllama::spawn()?;
    let workspace = ledger_workspace()?;
    let (_guard, mut stdin, mut stdout) = spawn_lsp_guarded(workspace.path())?;
    assert_initialize_contract(&handshake(&mut stdin, &mut stdout)?);

    let initial = wait_for_report(&mut stdin, &mut stdout, report_has_two_files)?;
    assert_embeddings_off(&initial);
    assert_model_selection(&mut stdin, &mut stdout, "ollama", server.endpoint())?;
    let first = wait_for_report(&mut stdin, &mut stdout, report_has_ollama_provenance)?;
    assert_embedding_report(&first)?;

    assert_model_selection(&mut stdin, &mut stdout, "off", server.endpoint())?;
    let disabled = wait_for_report(&mut stdin, &mut stdout, report_has_embeddings_off)?;
    assert_embeddings_off(&disabled);
    assert_model_selection(&mut stdin, &mut stdout, "ollama", server.endpoint())?;
    let second = wait_for_report(&mut stdin, &mut stdout, report_has_ollama_provenance)?;
    assert_embedding_report(&second)?;
    assert_repeated_report_identity(&first, &second);
    Ok(())
}

fn ledger_workspace() -> Result<tempfile::TempDir> {
    let workspace = tempfile::tempdir()?;
    copy_fixture_files("ts-mixed-band", &LEDGER_FILES, workspace.path())?;
    Ok(workspace)
}

fn assert_model_selection(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    provider_id: &str,
    endpoint: &str,
) -> Result<()> {
    let model_id = if provider_id == "off" {
        "off"
    } else {
        "nomic-embed-text"
    };
    let response = call(
        stdin,
        stdout,
        SET_MODEL,
        &json!({ "provider_id": provider_id, "model_id": model_id, "endpoint": endpoint }),
    )?;
    assert!(
        response.get("error").is_none(),
        "model selection failed: {response}"
    );
    assert!(
        response.get("result").is_some(),
        "model selection returned no result: {response}"
    );
    Ok(())
}

fn report_has_two_files(report: &Value) -> bool {
    at(report, "files_analysed").as_u64() == Some(2)
}

fn report_has_ollama_provenance(report: &Value) -> bool {
    json_path(report, &["embedding_provenance", "provider_id"]).as_str() == Some("ollama")
}

fn report_has_embeddings_off(report: &Value) -> bool {
    report_has_two_files(report) && at(report, "embedding_provenance").is_null()
}

fn assert_embeddings_off(report: &Value) {
    assert_report_shell_without_clusters(report, 2);
    assert!(at(report, "embedding_provenance").is_null(), "{report:#}");
    assert_eq!(
        at(report, "clusters"),
        &json!([]),
        "mid-band pair needs embedding evidence"
    );
}

fn assert_embedding_report(report: &Value) -> Result<()> {
    assert_report_shell(report, 2);
    assert_embedding_provenance(report);
    let cluster = ledger_cluster(report)?;
    assert_ledger_cluster_identity(cluster);
    assert_ledger_signal_contract(cluster);
    assert_ledger_occurrences(cluster)?;
    Ok(())
}

fn assert_embedding_provenance(report: &Value) {
    let provenance = at(report, "embedding_provenance");
    assert_eq!(at(provenance, "provider_id"), "ollama", "{report:#}");
    assert_eq!(at(provenance, "model_id"), "nomic-embed-text", "{report:#}");
    assert_eq!(at(provenance, "dimensions"), 4, "{report:#}");
    assert_eq!(at(provenance, "failed_subtrees"), 0, "{report:#}");
    assert!(
        at(provenance, "attempted_subtrees")
            .as_u64()
            .unwrap_or_default()
            > 0
    );
    assert!(
        at(provenance, "indexed_subtrees")
            .as_u64()
            .unwrap_or_default()
            > 0
    );
}

fn ledger_cluster(report: &Value) -> Result<&Value> {
    report_clusters(report)?
        .iter()
        .find(|cluster| {
            let paths = cluster_paths(cluster);
            LEDGER_FILES.iter().all(|name| has_suffix(&paths, name))
        })
        .ok_or_else(|| anyhow!("no cluster spans both ledger fixtures: {report:#}"))
}

fn assert_ledger_cluster_identity(cluster: &Value) {
    assert_eq!(at(cluster, "bucket"), "same_behavior", "{cluster:#}");
    assert_eq!(at(cluster, "category"), "logic", "{cluster:#}");
    assert_eq!(at(cluster, "size"), 2, "{cluster:#}");
    assert_eq!(at(cluster, "occurrences_total"), 2, "{cluster:#}");
    assert_eq!(at(cluster, "occurrences_truncated"), false, "{cluster:#}");
    assert!(at(cluster, "id").as_str().is_some_and(|id| !id.is_empty()));
    assert!(
        at(cluster, "canonical_node_count")
            .as_u64()
            .unwrap_or_default()
            >= 30
    );
    assert!(at(cluster, "weight").as_f64().unwrap_or_default() > 0.0);
    assert_eq!(
        at(cluster, "summary"),
        "",
        "LSP wire must omit derivable summary"
    );
    assert_eq!(
        at(cluster, "interpretation"),
        "",
        "LSP wire must omit derivable prose"
    );
}

fn assert_ledger_signal_contract(cluster: &Value) {
    let structural = signal(cluster, "structural");
    let token = signal(cluster, "token_jaccard");
    let embedding = signal(cluster, "embedding_cos");
    let fused = signal(cluster, "fused");
    for (name, value) in [
        ("structural", structural),
        ("token", token),
        ("embedding", embedding),
        ("fused", fused),
    ] {
        assert!(
            (0.0..=1.0).contains(&value),
            "{name} escaped [0,1]: {cluster:#}"
        );
    }
    assert!(
        structural < 0.05,
        "fixture gained a structural anchor: {cluster:#}"
    );
    assert!(
        token < 0.95,
        "fixture reached the content-gate corner: {cluster:#}"
    );
    assert!(
        token > 0.05,
        "fixture lost the second correlated signal: {cluster:#}"
    );
    assert!(
        (0.80..=0.99).contains(&embedding),
        "embedding left the calibrated band: {cluster:#}"
    );
    let strongest = structural.max(token).max(embedding);
    assert!(
        structural + token + embedding > 1.0,
        "fixture no longer reproduces sum/clamp saturation: {cluster:#}"
    );
    assert!(
        fused <= strongest + 1e-6,
        "fused exceeded its strongest axis: {cluster:#}"
    );
    assert!(
        (fused - strongest).abs() <= 1e-6,
        "bounded fusion must equal max axis: {cluster:#}"
    );
    assert!(
        fused < 1.0,
        "non-verbatim pair saturated confidence: {cluster:#}"
    );
    assert!(
        fused >= 0.85,
        "genuine pair fell below the act-now line: {cluster:#}"
    );
}

fn assert_ledger_occurrences(cluster: &Value) -> Result<()> {
    let occurrences = at(cluster, "occurrences")
        .as_array()
        .ok_or_else(|| anyhow!("cluster carries no occurrences: {cluster}"))?;
    assert_eq!(occurrences.len(), 2, "{cluster:#}");
    assert_eq!(
        cluster_paths(cluster).len(),
        2,
        "one occurrence per ledger file"
    );
    for occurrence in occurrences {
        assert_eq!(at(occurrence, "hidden"), false, "{occurrence}");
        assert!(at(occurrence, "start_byte").as_u64() < at(occurrence, "end_byte").as_u64());
        assert!(at(occurrence, "start_line").as_u64() <= at(occurrence, "end_line").as_u64());
    }
    Ok(())
}

fn assert_repeated_report_identity(first: &Value, second: &Value) {
    assert_eq!(at(first, "files_analysed"), at(second, "files_analysed"));
    assert_eq!(at(first, "clusters_hidden"), at(second, "clusters_hidden"));
    assert_eq!(
        at(first, "metrics"),
        at(second, "metrics"),
        "repo metrics drifted across refreshes"
    );
    assert_eq!(
        at(first, "clusters"),
        at(second, "clusters"),
        "ordered clusters drifted across refreshes"
    );
    assert_eq!(
        at(first, "embedding_provenance"),
        at(second, "embedding_provenance")
    );
    assert_eq!(at(first, "action_hints"), at(second, "action_hints"));
    assert_eq!(
        at(first, "boilerplate_hints"),
        at(second, "boilerplate_hints")
    );
}
