//! Shared plumbing for the mechanical call-site merge suites
//! ([AUTOFIX-MERGE], [AUTOFIX-MERGE-GATE], [AUTOFIX-MERGE-SAFETY]).
//!
//! `refactor_merge.rs` drives the mechanical path to its goldens and
//! `refactor_merge_refusals.rs` drives the refusal path; both need the
//! same pipeline-to-`MergePlan` bridge and the same LSP-host `apply`.

use std::{fs, path::Path};

use anyhow::{anyhow, ensure, Context, Result};
use deslop_core::{
    ast::ByteRange,
    refactor::{self, merge},
    wire_generated::{MergePlan, MergeVerdict},
};
use serde_json::Value;

use crate::common::{
    census::assert_body_deduplicated, fixture, refactor_golden as golden,
    refactor_pipeline_session as session,
};

/// Computes merge plans for every cluster of a single-file fixture and
/// returns them in rank order.
pub(crate) fn merge_plans(fixture_name: &str, file_name: &str) -> Result<Vec<MergePlan>> {
    merge_plans_under(&fixture(fixture_name), file_name)
}

/// Computes merge plans for a fixture rooted at an already-resolved
/// path, so a caller can choose the shape of the absolute path the
/// plans are addressed with (plain, or canonicalised as the servers do).
pub(crate) fn merge_plans_under(root: &Path, file_name: &str) -> Result<Vec<MergePlan>> {
    let (session, report) = session(root)?;
    let absolute = root.join(file_name);
    let file_id = session
        .file_id_for(&absolute)
        .context("fixture file registered")?;
    let source = session
        .source_bytes_for(file_id)
        .context("source retrievable")?
        .to_vec();
    let file_root = session
        .subtree_at_range(
            file_id,
            ByteRange {
                start: 0,
                end: source.len(),
            },
        )
        .context("file root retrievable")?;
    let parser = refactor::parser_for_path(&absolute).context("parser registered for fixture")?;
    ensure!(
        !report.clusters.is_empty(),
        "{} must produce clusters",
        root.display()
    );
    report
        .clusters
        .iter()
        .map(|cluster| {
            merge::compute_merge_plan(cluster, &source, file_root, &absolute, parser.as_ref())
                .map_err(|error| anyhow!("merge plan failed: {error}"))
        })
        .collect()
}

/// The first mechanical plan in rank order; errors list every
/// refusal reason so a failing fixture explains itself.
pub(crate) fn first_mechanical(plans: Vec<MergePlan>) -> Result<MergePlan> {
    let reasons: Vec<String> = plans
        .iter()
        .map(|plan| match &plan.verdict {
            MergeVerdict::Mechanical => format!("{}: mechanical", plan.cluster_id),
            MergeVerdict::AiOrHuman { reason } => format!("{}: {reason}", plan.cluster_id),
        })
        .collect();
    plans
        .into_iter()
        .find(|plan| matches!(plan.verdict, MergeVerdict::Mechanical))
        .ok_or_else(|| anyhow!("no mechanical plan; verdicts:\n{}", reasons.join("\n")))
}

/// Asserts that at least one plan of a fixture refuses with a reason
/// containing `needle`, and that no plan is mechanical.
pub(crate) fn assert_all_refused_with(
    fixture_name: &str,
    file_name: &str,
    needle: &str,
) -> Result<()> {
    let plans = merge_plans(fixture_name, file_name)?;
    ensure!(!plans.is_empty(), "{fixture_name} produces clusters");
    let mut matched = false;
    for plan in plans {
        let MergeVerdict::AiOrHuman { reason } = plan.verdict else {
            return Err(anyhow!(
                "{fixture_name}: cluster {} must refuse",
                plan.cluster_id
            ));
        };
        matched = matched || reason.contains(needle);
    }
    ensure!(matched, "{fixture_name}: some refusal names `{needle}`");
    Ok(())
}

/// Applies a wire `WorkspaceEdit` (documentChanges form) to an ASCII
/// buffer, mirroring an LSP host.
pub(crate) fn apply_workspace_edit(source: &str, edit: &Value) -> Result<String> {
    ensure!(source.is_ascii(), "fixture must stay ASCII for offset math");
    let edits = edit
        .pointer("/documentChanges/0/edits")
        .and_then(Value::as_array)
        .context("documentChanges edits array")?;
    let mut spans: Vec<(usize, usize, String)> = edits
        .iter()
        .map(|entry| {
            let range = entry.get("range").context("edit range")?;
            let start = byte_offset(source, range.get("start").context("range start")?)?;
            let end = byte_offset(source, range.get("end").context("range end")?)?;
            let text = entry
                .get("newText")
                .and_then(Value::as_str)
                .context("edit newText")?
                .to_owned();
            Ok((start, end, text))
        })
        .collect::<Result<_>>()?;
    spans.sort_by_key(|span| std::cmp::Reverse(span.0));
    let mut buffer = source.to_owned();
    for (start, end, text) in spans {
        ensure!(start <= end && end <= buffer.len(), "edit range in bounds");
        buffer.replace_range(start..end, &text);
    }
    Ok(buffer)
}

/// The byte offset of an LSP `{line, character}` position in an ASCII
/// buffer.
fn byte_offset(source: &str, position: &Value) -> Result<usize> {
    let line = position
        .get("line")
        .and_then(Value::as_u64)
        .context("position line")?;
    let character = position
        .get("character")
        .and_then(Value::as_u64)
        .context("position character")?;
    let line_start = source
        .split_inclusive('\n')
        .scan(0_usize, |offset, text| {
            let start = *offset;
            *offset = offset.saturating_add(text.len());
            Some(start)
        })
        .nth(usize::try_from(line).context("line fits")?)
        .context("line exists")?;
    Ok(line_start.saturating_add(usize::try_from(character).context("char fits")?))
}

/// Compares a rendered buffer against the shared golden, honouring
/// `DESLOP_BLESS`.
fn assert_matches_golden(applied: &str, golden_name: &str) -> Result<()> {
    let golden_path = golden(golden_name);
    if std::env::var_os("DESLOP_BLESS").is_some() {
        fs::write(&golden_path, applied).context("blessing golden")?;
    }
    let expected = fs::read_to_string(&golden_path).context("golden")?;
    ensure!(
        applied == expected,
        "applied merge must match golden {}.\n--- applied ---\n{applied}",
        golden_path.display()
    );
    Ok(())
}

/// Applies the plan's wire edit and asserts the result twice: every
/// statement of the duplicated body was consumed exactly once, and the
/// buffer matches the shared golden.
///
/// The census is what makes the golden's embedded cluster id provable
/// ([PIPELINE-CLUSTER-EXACT]). One duplication yields several candidate
/// clusters — the whole duplicated body and the statement runs nested
/// inside it — and a plan computed from a nested run merges cleanly,
/// names its helper after its own id, and produces an internally
/// consistent buffer. Only the statements it left behind give it away.
pub(crate) fn assert_merge_golden(
    plan: &MergePlan,
    fixture_name: &str,
    file_name: &str,
    golden_name: &str,
    duplicated_statements: &[&str],
) -> Result<String> {
    let source = fs::read_to_string(fixture(fixture_name).join(file_name))?;
    let edit = plan.workspace_edit.as_ref().context("wire edit present")?;
    let applied = apply_workspace_edit(&source, edit)?;
    assert_body_deduplicated(&source, &applied, duplicated_statements, file_name)?;
    assert_matches_golden(&applied, golden_name)?;
    Ok(applied)
}
