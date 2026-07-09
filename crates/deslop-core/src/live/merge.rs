//! Live-session glue for the mechanical merge and the cross-file
//! consolidation ([AUTOFIX-MERGE-MCP], [AUTOFIX-CONSOLIDATE-SURFACE]).
//!
//! Resolves a cluster id against the session, gathers the in-memory
//! source bytes and normalised tree ([AUTOFIX-MERGE]'s in-process
//! accessors), and hands everything to the refactor engine — routing
//! by cluster shape: single-file clusters merge, multi-file clusters
//! consolidate. Every infrastructure gap (cache-seed window, unknown
//! file) surfaces as an `ai_or_human` refusal with a reason — an agent
//! can always act on the response.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    ast::ByteRange,
    live::{errors::LiveError, session::AnalysisSession},
    pipeline::PipelineSession,
    refactor::{
        self,
        consolidate::{compute_consolidation_plan, ConsolidatePlan, ConsolidationOutcome},
        emit::PlannedEdit,
        merge, wire_edit,
    },
    report::ReportCluster,
    wire_generated::{MergePlan, MergeVerdict},
};

/// Annotation id labelling consolidation edits in the preview tree.
const CONSOLIDATE_ANNOTATION_ID: &str = "deslop.consolidate";

/// Annotation label shown on the consolidation preview tree.
const CONSOLIDATE_ANNOTATION_LABEL: &str =
    "Deslop: consolidate identical duplicates into one canonical definition";

/// Computes the mechanical plan for `cluster_id` against the live
/// session — the merge for single-file clusters, the consolidation for
/// multi-file clusters ([AUTOFIX-CONSOLIDATE-SURFACE] routing).
///
/// # Errors
///
/// Returns [`LiveError::UnknownCluster`] when no cluster matches the
/// id, and [`LiveError::Core`] when an occurrence file fails to
/// parse.
pub fn merge_plan_for(session: &AnalysisSession, cluster_id: &str) -> Result<MergePlan, LiveError> {
    let cluster = session.cluster_by_id(cluster_id)?;
    let Some(pipeline) = session.pipeline() else {
        return Ok(refusal(
            &cluster,
            "analysis is still warming up (cache-seed window)".to_owned(),
        ));
    };
    let distinct_paths: std::collections::HashSet<_> = cluster
        .occurrences
        .iter()
        .filter(|occurrence| !occurrence.hidden)
        .map(|occurrence| &occurrence.path)
        .collect();
    if distinct_paths.len() > 1 {
        return consolidation_plan_for(&cluster, pipeline);
    }
    single_file_merge_plan(&cluster, pipeline)
}

/// The original single-file merge path ([AUTOFIX-MERGE-MCP]).
fn single_file_merge_plan(
    cluster: &ReportCluster,
    pipeline: &PipelineSession,
) -> Result<MergePlan, LiveError> {
    let Some(first) = cluster.occurrences.first() else {
        return Ok(refusal(cluster, "cluster has no occurrences".to_owned()));
    };
    let absolute = pipeline.root().join(&first.path);
    let (Some(file_id), Some(parser)) = (
        pipeline.file_id_for(&absolute),
        refactor::parser_for_path(&absolute),
    ) else {
        return Ok(refusal(
            cluster,
            format!("occurrence file {} is not analysable", first.path.display()),
        ));
    };
    let Some(source) = pipeline.source_bytes_for(file_id) else {
        return Ok(refusal(
            cluster,
            "occurrence source bytes are unavailable".to_owned(),
        ));
    };
    let full_range = ByteRange {
        start: 0,
        end: source.len(),
    };
    let Some(file_root) = pipeline.subtree_at_range(file_id, full_range) else {
        return Ok(refusal(cluster, "normalised tree unavailable".to_owned()));
    };
    merge::compute_merge_plan(cluster, source, file_root, &absolute, parser.as_ref())
        .map_err(|refactor::RefactorError::Core(error)| LiveError::Core(error))
}

/// Routes a multi-file cluster to the consolidation engine
/// ([AUTOFIX-CONSOLIDATE-SURFACE]): mechanical plans answer with the
/// multi-file `WorkspaceEdit`, refusals stay `ai_or_human` with the
/// gate's reason.
fn consolidation_plan_for(
    cluster: &ReportCluster,
    pipeline: &PipelineSession,
) -> Result<MergePlan, LiveError> {
    let Some(first) = cluster.occurrences.first() else {
        return Ok(refusal(cluster, "cluster has no occurrences".to_owned()));
    };
    let Some(parser) = refactor::parser_for_path(&pipeline.root().join(&first.path)) else {
        return Ok(refusal(
            cluster,
            format!("occurrence file {} is not analysable", first.path.display()),
        ));
    };
    let sources = match occurrence_sources(cluster, pipeline) {
        Ok(sources) => sources,
        Err(reason) => return Ok(refusal(cluster, reason)),
    };
    let outcome = compute_consolidation_plan(cluster, &sources, parser.as_ref())
        .map_err(|refactor::RefactorError::Core(error)| LiveError::Core(error))?;
    Ok(match outcome {
        ConsolidationOutcome::Refused(reason) => refusal(cluster, reason),
        ConsolidationOutcome::Mechanical(plan) => {
            mechanical_consolidation(cluster, parser.id(), &plan, pipeline.root(), &sources)
        }
    })
}

/// In-memory bytes for every occurrence file, keyed by the
/// workspace-relative path the report uses.
fn occurrence_sources(
    cluster: &ReportCluster,
    pipeline: &PipelineSession,
) -> Result<HashMap<PathBuf, Vec<u8>>, String> {
    let mut sources = HashMap::new();
    for occurrence in cluster.occurrences.iter().filter(|entry| !entry.hidden) {
        if sources.contains_key(&occurrence.path) {
            continue;
        }
        let absolute = pipeline.root().join(&occurrence.path);
        let bytes = pipeline
            .file_id_for(&absolute)
            .and_then(|file_id| pipeline.source_bytes_for(file_id))
            .ok_or_else(|| {
                format!(
                    "occurrence source bytes are unavailable for {}",
                    occurrence.path.display()
                )
            })?;
        let _inserted = sources.insert(occurrence.path.clone(), bytes.to_vec());
    }
    Ok(sources)
}

/// Projects a mechanical [`ConsolidatePlan`] onto the wire `MergePlan`
/// shape: the consolidated symbols ride in `helper_name`, the
/// multi-file edit in `workspace_edit` ([AUTOFIX-CONSOLIDATE-SURFACE]).
fn mechanical_consolidation(
    cluster: &ReportCluster,
    language: &str,
    plan: &ConsolidatePlan,
    root: &Path,
    sources: &HashMap<PathBuf, Vec<u8>>,
) -> MergePlan {
    MergePlan {
        cluster_id: cluster.id.clone(),
        language: language.to_owned(),
        verdict: MergeVerdict::Mechanical,
        helper_name: plan.symbols.join(", "),
        helper_body: String::new(),
        parameters: Vec::new(),
        workspace_edit: wire_edit::workspace_edit_json(
            &consolidation_file_edits(plan, root, sources),
            CONSOLIDATE_ANNOTATION_ID,
            CONSOLIDATE_ANNOTATION_LABEL,
        ),
    }
}

/// Groups the plan's per-file byte edits into the wire serialiser's
/// shape, preserving the plan's per-file descending order.
fn consolidation_file_edits<'a>(
    plan: &ConsolidatePlan,
    root: &Path,
    sources: &'a HashMap<PathBuf, Vec<u8>>,
) -> Vec<wire_edit::FileEdits<'a>> {
    let mut files: Vec<wire_edit::FileEdits<'a>> = Vec::new();
    for edit in &plan.edits {
        let Some(source) = sources.get(&edit.path) else {
            continue;
        };
        let planned = PlannedEdit {
            start_byte: edit.start_byte,
            end_byte: edit.end_byte,
            new_text: edit.new_text.clone(),
        };
        let absolute = root.join(&edit.path);
        match files.iter_mut().find(|file| file.absolute_path == absolute) {
            Some(file) => file.edits.push(planned),
            None => files.push(wire_edit::FileEdits {
                absolute_path: absolute,
                source,
                edits: vec![planned],
            }),
        }
    }
    files
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
