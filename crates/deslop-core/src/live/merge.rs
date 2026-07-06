//! Live-session glue for the mechanical merge ([AUTOFIX-MERGE-MCP]).
//!
//! Resolves a cluster id against the session, gathers the in-memory
//! source bytes and normalised tree ([AUTOFIX-MERGE]'s in-process
//! accessors), and hands everything to the refactor engine. Every
//! infrastructure gap (cache-seed window, unknown file) surfaces as an
//! `ai_or_human` refusal with a reason — an agent can always act on
//! the response.

use crate::{
    ast::ByteRange,
    live::{errors::LiveError, session::AnalysisSession},
    refactor::{self, merge},
    report::ReportCluster,
    wire_generated::{MergePlan, MergeVerdict},
};

/// Computes the merge plan for `cluster_id` against the live session.
///
/// # Errors
///
/// Returns [`LiveError::UnknownCluster`] when no cluster matches the
/// id, and [`LiveError::Core`] when the occurrence file fails to
/// parse.
pub fn merge_plan_for(session: &AnalysisSession, cluster_id: &str) -> Result<MergePlan, LiveError> {
    let cluster = session.cluster_by_id(cluster_id)?;
    let Some(pipeline) = session.pipeline() else {
        return Ok(refusal(
            &cluster,
            "analysis is still warming up (cache-seed window)".to_owned(),
        ));
    };
    let Some(first) = cluster.occurrences.first() else {
        return Ok(refusal(&cluster, "cluster has no occurrences".to_owned()));
    };
    let absolute = pipeline.root().join(&first.path);
    let (Some(file_id), Some(parser)) = (
        pipeline.file_id_for(&absolute),
        refactor::parser_for_path(&absolute),
    ) else {
        return Ok(refusal(
            &cluster,
            format!("occurrence file {} is not analysable", first.path.display()),
        ));
    };
    let Some(source) = pipeline.source_bytes_for(file_id) else {
        return Ok(refusal(
            &cluster,
            "occurrence source bytes are unavailable".to_owned(),
        ));
    };
    let full_range = ByteRange {
        start: 0,
        end: source.len(),
    };
    let Some(file_root) = pipeline.subtree_at_range(file_id, full_range) else {
        return Ok(refusal(&cluster, "normalised tree unavailable".to_owned()));
    };
    merge::compute_merge_plan(&cluster, source, file_root, &absolute, parser.as_ref())
        .map_err(|refactor::RefactorError::Core(error)| LiveError::Core(error))
}

/// An `ai_or_human` plan for infrastructure-level refusals.
fn refusal(cluster: &ReportCluster, reason: String) -> MergePlan {
    MergePlan {
        cluster_id: cluster.id.clone(),
        language: String::new(),
        verdict: MergeVerdict::AiOrHuman { reason },
        helper_name: String::new(),
        helper_body: String::new(),
        parameters: Vec::new(),
        workspace_edit: None,
    }
}
