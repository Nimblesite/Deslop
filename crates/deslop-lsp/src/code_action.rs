//! `textDocument/codeAction` surface ([AUTOFIX-EXTRACT-CODE-ACTION]).
//!
//! Thin projection layer: clusters intersecting the requested range
//! come from the lock-free report snapshot, the refactor plan comes
//! from `deslop-core::refactor`, and this module only maps byte-based
//! [`PlannedEdit`]s onto LSP positions. No AST walking, no code
//! emission here ([AUTOFIX-EXTRACT-CODE-ACTION] layering rule).

use std::{collections::HashMap, path::Path};

use deslop_core::{
    live::report_for_range_in,
    refactor::{self, ExtractMethodPlan},
    report::Report,
};
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Range, TextEdit, Url, WorkspaceEdit,
};

use crate::position::{byte_for_position, position_for_byte};

/// User-facing action title, verbatim per [AUTOFIX-EXTRACT-NORTH-STAR].
pub const EXTRACT_ACTION_TITLE: &str = "Extract identical code to shared method";

/// User-facing title of the mechanical merge action
/// ([AUTOFIX-MERGE-CODE-ACTION]).
pub const MERGE_ACTION_TITLE: &str = "Merge duplicates into one parameterised helper";

/// User-facing title of the cross-file consolidation action
/// ([AUTOFIX-CONSOLIDATE-SURFACE]).
pub const CONSOLIDATE_ACTION_TITLE: &str =
    "Consolidate identical duplicates into one canonical definition";

/// Builds the `refactor.extract` actions for every eligible cluster
/// intersecting `range` — one complete, atomically-applicable
/// `WorkspaceEdit` per action, never a partial edit
/// ([AUTOFIX-EXTRACT-CODE-ACTION]).
#[must_use]
pub fn build_for_range(
    report: &Report,
    path: &Path,
    uri: &Url,
    source: &[u8],
    range: Range,
) -> Vec<CodeActionOrCommand> {
    let Ok(text) = std::str::from_utf8(source) else {
        return Vec::new();
    };
    let Some(parser) = refactor::parser_for_path(path) else {
        return Vec::new();
    };
    let start_byte = byte_for_position(text, range.start);
    let end_byte = byte_for_position(text, range.end);
    let mut actions = Vec::new();
    for cluster in &report_for_range_in(report, path, start_byte, end_byte) {
        if let Some(plan) = plan_for_cluster(cluster, source, parser.as_ref()) {
            actions.push(action_for_plan(uri, text, &plan));
        } else if refactor::preconditions::eligible_ranges(cluster).is_some() {
            actions.push(rewrite_offer(cluster, MERGE_ACTION_TITLE));
        } else if refactor::preconditions::consolidation_candidate(cluster) {
            actions.push(rewrite_offer(cluster, CONSOLIDATE_ACTION_TITLE));
        }
    }
    actions
}

/// The lazily-resolved `refactor.rewrite` offer shared by the merge
/// and consolidation actions ([AUTOFIX-MERGE-CODE-ACTION] step 1,
/// [AUTOFIX-CONSOLIDATE-SURFACE]): the edit is omitted and `data`
/// carries the cluster id for `codeAction/resolve`, where the engine
/// routes by cluster shape.
fn rewrite_offer(cluster: &deslop_core::report::ReportCluster, title: &str) -> CodeActionOrCommand {
    CodeActionOrCommand::CodeAction(CodeAction {
        title: title.to_owned(),
        kind: Some(CodeActionKind::REFACTOR_REWRITE),
        is_preferred: Some(true),
        data: Some(serde_json::json!({ "cluster_id": cluster.id })),
        ..CodeAction::default()
    })
}

/// The cluster id stashed in a rewrite offer's `data`, if any.
#[must_use]
pub fn offered_cluster_id(action: &CodeAction) -> Option<String> {
    action
        .data
        .as_ref()
        .and_then(|data| data.pointer("/cluster_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

/// Finishes a rewrite offer with the computed plan
/// ([AUTOFIX-MERGE-CODE-ACTION] step 2): mechanical plans attach the
/// transactional `WorkspaceEdit`; refusals disable the action with the
/// routing reason so the user sees why.
#[must_use]
pub fn resolved_action(
    mut action: CodeAction,
    plan: &deslop_core::wire_generated::MergePlan,
) -> CodeAction {
    match &plan.verdict {
        deslop_core::wire_generated::MergeVerdict::Mechanical => {
            let edit = plan
                .workspace_edit
                .clone()
                .and_then(|value| serde_json::from_value::<WorkspaceEdit>(value).ok());
            match edit {
                // An enabled action without an edit would apply as a
                // silent no-op, so a missing/malformed edit disables.
                Some(edit) => action.edit = Some(edit),
                None => {
                    action.disabled = Some(tower_lsp::lsp_types::CodeActionDisabled {
                        reason: "the engine returned no applicable edit for this plan".to_owned(),
                    });
                }
            }
        }
        deslop_core::wire_generated::MergeVerdict::AiOrHuman { reason } => {
            action.disabled = Some(tower_lsp::lsp_types::CodeActionDisabled {
                reason: reason.clone(),
            });
        }
    }
    action
}

/// Computes one cluster's plan, logging (rather than surfacing) parse
/// failures so a single unreadable file cannot break the whole
/// code-action response.
fn plan_for_cluster(
    cluster: &deslop_core::report::ReportCluster,
    source: &[u8],
    parser: &dyn deslop_core::lang::LanguageParser,
) -> Option<ExtractMethodPlan> {
    match refactor::compute_plan(cluster, source, parser) {
        Ok(plan) => plan,
        Err(error) => {
            tracing::warn!(cluster_id = %cluster.id, %error, "extract plan computation failed");
            None
        }
    }
}

/// Maps a byte-addressed plan onto one LSP `CodeAction` with a
/// same-document `WorkspaceEdit` ([AUTOFIX-EXTRACT-WORKSPACE-EDIT]).
fn action_for_plan(uri: &Url, text: &str, plan: &ExtractMethodPlan) -> CodeActionOrCommand {
    let edits: Vec<TextEdit> = plan
        .edits
        .iter()
        .map(|edit| TextEdit {
            range: Range {
                start: position_for_byte(text, edit.start_byte),
                end: position_for_byte(text, edit.end_byte),
            },
            new_text: edit.new_text.clone(),
        })
        .collect();
    let changes = HashMap::from([(uri.clone(), edits)]);
    CodeActionOrCommand::CodeAction(CodeAction {
        title: EXTRACT_ACTION_TITLE.to_owned(),
        kind: Some(CodeActionKind::REFACTOR_EXTRACT),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..WorkspaceEdit::default()
        }),
        ..CodeAction::default()
    })
}

#[cfg(test)]
#[allow(clippy::missing_docs_in_private_items)]
#[path = "code_action_tests.rs"]
mod tests;
