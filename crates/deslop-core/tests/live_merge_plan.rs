//! E2E coverage for the live merge-plan glue ([AUTOFIX-MERGE-MCP]):
//! `live::merge::merge_plan_for` resolves a cluster id against a real
//! `AnalysisSession`, gathers the in-memory inputs, and returns the
//! mechanical plan — or a reasoned refusal, never a partial answer.

use std::sync::Arc;

use anyhow::{anyhow, ensure, Context, Result};
use deslop_core::{
    live::{merge::merge_plan_for, AnalysisSession, LiveError},
    wire_generated::MergeVerdict,
    NoopProvider,
};

use crate::common::{copy_fixture as fixture_workspace, REFACTOR_MIN_NODES as MIN_NODES};

/// Builds a live session over a fixture.
fn live_session(name: &str) -> Result<(tempfile::TempDir, AnalysisSession)> {
    let workspace = fixture_workspace(name)?;
    let provider = Arc::new(NoopProvider);
    let session = AnalysisSession::new(
        workspace.path().to_path_buf(),
        MIN_NODES,
        false,
        None,
        provider,
    )
    .map_err(|error| anyhow!("session: {error}"))?;
    Ok((workspace, session))
}

/// The leaf-gap fixture merges mechanically through the live glue.
#[test]
fn live_session_returns_mechanical_plan() -> Result<()> {
    let (_workspace, session) = live_session("csharp-merge-leafgap")?;
    let report = session.report();
    let cluster = report.clusters.first().context("cluster present")?;
    let plan = merge_plan_for(&session, &cluster.id)
        .map_err(|error| anyhow!("merge_plan_for: {error}"))?;
    ensure!(
        matches!(plan.verdict, MergeVerdict::Mechanical),
        "the leaf-gap cluster merges mechanically, got {:?}",
        plan.verdict
    );
    ensure!(
        plan.workspace_edit.is_some(),
        "mechanical plans carry the wire WorkspaceEdit"
    );
    Ok(())
}

/// Unknown ids surface `LiveError::UnknownCluster`.
#[test]
fn live_session_rejects_unknown_ids() -> Result<()> {
    let (_workspace, session) = live_session("csharp-merge-leafgap")?;
    let outcome = merge_plan_for(&session, "ffffffffffffffff");
    ensure!(
        matches!(outcome, Err(LiveError::UnknownCluster { .. })),
        "unknown ids must error with UnknownCluster"
    );
    Ok(())
}

/// Structural drift refuses with a reason through the live glue.
#[test]
fn live_session_refuses_drifted_cluster() -> Result<()> {
    let (_workspace, session) = live_session("csharp-merge-drift")?;
    let report = session.report();
    let cluster = report.clusters.first().context("cluster present")?;
    let plan = merge_plan_for(&session, &cluster.id)
        .map_err(|error| anyhow!("merge_plan_for: {error}"))?;
    let MergeVerdict::AiOrHuman { reason } = plan.verdict else {
        return Err(anyhow!("drifted cluster must refuse"));
    };
    ensure!(!reason.is_empty(), "refusal carries a reason");
    Ok(())
}

/// During the cache-seed window (no installed pipeline) the merge glue
/// answers with a reasoned refusal instead of blocking or erroring.
#[test]
fn cache_seed_window_refuses_with_reason() -> Result<()> {
    let workspace = fixture_workspace("csharp-merge-leafgap")?;
    let provider: Arc<dyn deslop_core::EmbeddingProvider> = Arc::new(NoopProvider);
    // A first full session persists the seed cache the seeded session
    // boots from.
    let warm = AnalysisSession::new(
        workspace.path().to_path_buf(),
        MIN_NODES,
        false,
        None,
        provider.clone(),
    )
    .map_err(|error| anyhow!("warm session: {error}"))?;
    warm.persist_seed_cache();
    let report = warm.report();
    let cluster = report.clusters.first().context("cluster present")?;
    drop(warm);

    let seeded = AnalysisSession::try_seeded_from_cache(
        workspace.path().to_path_buf(),
        MIN_NODES,
        false,
        None,
        provider,
        deslop_core::EmbeddingMode::Off,
    )
    .context("seed cache must hydrate a session")?;
    ensure!(seeded.is_seed_only(), "seeded session has no pipeline yet");
    let plan =
        merge_plan_for(&seeded, &cluster.id).map_err(|error| anyhow!("merge_plan_for: {error}"))?;
    let MergeVerdict::AiOrHuman { reason } = plan.verdict else {
        return Err(anyhow!("cache-seed window must refuse"));
    };
    ensure!(
        reason.contains("warming up"),
        "the refusal names the cache-seed window, got {reason}"
    );
    Ok(())
}

/// [AUTOFIX-CONSOLIDATE-SURFACE] (issue #277): a cross-file identical
/// definition routes through the live glue to the consolidation engine
/// and answers a mechanical plan carrying the multi-file
/// `WorkspaceEdit` and the consolidated symbol.
#[test]
fn live_session_consolidates_cross_file_cluster() -> Result<()> {
    let (_workspace, session) = live_session("rust-consolidate")?;
    let report = session.report();
    let cluster = crate::common::clusters::cross_file_identical_cluster(&report)?;
    let plan = merge_plan_for(&session, &cluster.id)
        .map_err(|error| anyhow!("merge_plan_for: {error}"))?;
    ensure!(
        matches!(plan.verdict, MergeVerdict::Mechanical),
        "the sibling-module duplicate consolidates mechanically, got {:?}",
        plan.verdict
    );
    ensure!(
        plan.helper_name == "normalise_labels",
        "the consolidated symbol rides in helper_name, got {}",
        plan.helper_name
    );
    let edit = plan.workspace_edit.context("wire WorkspaceEdit present")?;
    let edits = edit
        .pointer("/documentChanges/0/edits")
        .and_then(serde_json::Value::as_array)
        .context("documentChanges carry the duplicate file's edits")?;
    ensure!(edits.len() == 2, "deletion + import, got {}", edits.len());
    let uri = edit
        .pointer("/documentChanges/0/textDocument/uri")
        .and_then(serde_json::Value::as_str)
        .context("edited uri present")?;
    ensure!(
        uri.starts_with("file://") && uri.contains("pricing_"),
        "absolute duplicate-file uri, got {uri}"
    );
    Ok(())
}

/// [AUTOFIX-CONSOLIDATE-GATE] binding drift through the live glue
/// (issue #279): byte-identical `run` bodies calling a per-module
/// `shift` refuse with the drifting symbol named — never a mechanical
/// plan that would change behaviour.
#[test]
fn live_session_refuses_binding_drifted_consolidation() -> Result<()> {
    let (_workspace, session) = live_session("rust-consolidate-drift")?;
    let report = session.report();
    let cluster = crate::common::clusters::cross_file_identical_cluster(&report)?;
    let plan = merge_plan_for(&session, &cluster.id)
        .map_err(|error| anyhow!("merge_plan_for: {error}"))?;
    let MergeVerdict::AiOrHuman { reason } = plan.verdict else {
        return Err(anyhow!("drifted consolidation must refuse"));
    };
    ensure!(
        reason.contains("shift"),
        "the drifting symbol is named in the refusal, got {reason}"
    );
    ensure!(
        plan.workspace_edit.is_none(),
        "refusals never carry an edit"
    );
    Ok(())
}
