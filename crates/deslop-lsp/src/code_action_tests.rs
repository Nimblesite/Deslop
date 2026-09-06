use std::{fs, path::PathBuf};

use anyhow::{anyhow, ensure, Context, Result};
use deslop_core::{
    report::{CacheStats, Report, ReportCluster, ReportOccurrence, ReportSignals},
    report_metrics::RepoMetrics,
};
use tower_lsp::lsp_types::{CodeActionOrCommand, Position, Range};

use super::*;

/// The shared C# Type-1 fixture backing every scenario here — the same
/// file the E2E suites cluster ([AUTOFIX-EXTRACT-TESTING]).
fn fixture_source() -> Result<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../deslop/tests/fixtures/csharp-extract-type1/InvoiceMath.cs");
    Ok(fs::read_to_string(path)?)
}

/// Byte spans of the two duplicated statement runs in the fixture.
fn occurrence_spans(source: &str) -> Result<((usize, usize), (usize, usize))> {
    let needle_start = "var total = 0;";
    let needle_end = "return total;";
    let first_start = source.find(needle_start).context("first run start")?;
    let resume = first_start.saturating_add(needle_start.len());
    let second_start = source
        .get(resume..)
        .and_then(|rest| rest.find(needle_start))
        .map(|offset| resume.saturating_add(offset))
        .context("second run start")?;
    let first_end = source
        .find(needle_end)
        .map(|offset| offset.saturating_add(needle_end.len()))
        .context("first run end")?;
    let second_end = source
        .get(first_end..)
        .and_then(|rest| rest.find(needle_end))
        .map(|offset| {
            first_end
                .saturating_add(offset)
                .saturating_add(needle_end.len())
        })
        .context("second run end")?;
    Ok(((first_start, first_end), (second_start, second_end)))
}

/// Wraps one proven-Identical cluster over the fixture spans in a
/// minimal report.
fn report_with_cluster(path: &std::path::Path, spans: ((usize, usize), (usize, usize))) -> Report {
    let occurrences = vec![occurrence(path, spans.0), occurrence(path, spans.1)];
    Report {
        tool_version: "test".to_owned(),
        min_nodes: 30,
        files_analysed: 1,
        clusters_hidden: 0,
        cache_stats: CacheStats::default(),
        metrics: RepoMetrics::default(),
        schema_doc: String::new(),
        action_hints: Vec::new(),
        boilerplate_hints: Vec::new(),
        embedding_provenance: None,
        clusters: vec![ReportCluster {
            id: "abcdef0123456789".to_owned(),
            weight: 10.0,
            size: 2,
            canonical_node_count: 40,
            signals: ReportSignals {
                structural: 1.0,
                token_jaccard: 1.0,
                embedding_cos: 0.0,
                fused: 1.0,
            },
            bucket: "identical".to_owned(),
            category: "logic".to_owned(),
            occurrences_total: 2,
            occurrences,
            occurrences_truncated: false,
            summary: String::new(),
            interpretation: String::new(),
        }],
    }
}

fn occurrence(path: &std::path::Path, span: (usize, usize)) -> ReportOccurrence {
    ReportOccurrence {
        path: path.to_path_buf(),
        start_byte: span.0,
        end_byte: span.1,
        start_line: 0,
        end_line: 0,
        hidden: false,
    }
}

fn fixture_setup() -> Result<(tempfile::TempDir, PathBuf, Url, String, Report)> {
    let source = fixture_source()?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("InvoiceMath.cs");
    fs::write(&path, &source)?;
    let uri = Url::from_file_path(&path).map_err(|()| anyhow!("absolute fixture path"))?;
    let spans = occurrence_spans(&source)?;
    let report = report_with_cluster(&path, spans);
    Ok((dir, path, uri, source, report))
}

/// [AUTOFIX-EXTRACT-CODE-ACTION]: an eligible cluster intersecting the
/// range yields one complete action — exact title, exact kind, one
/// insertion plus two call rewrites in descending order, all targeting
/// the requested document.
#[test]
fn eligible_cluster_yields_one_complete_action() -> Result<()> {
    let (_dir, path, uri, source, report) = fixture_setup()?;
    let range = Range {
        start: Position {
            line: 4,
            character: 8,
        },
        end: Position {
            line: 5,
            character: 0,
        },
    };
    let actions = build_for_range(&report, &path, &uri, source.as_bytes(), range);
    ensure!(
        actions.len() == 1,
        "exactly one action, got {}",
        actions.len()
    );
    let CodeActionOrCommand::CodeAction(action) = actions.first().context("first action")? else {
        return Err(anyhow!("expected a code action literal"));
    };
    ensure!(
        action.title == EXTRACT_ACTION_TITLE,
        "title mismatch: {}",
        action.title
    );
    ensure!(
        action.kind == Some(CodeActionKind::REFACTOR_EXTRACT),
        "kind mismatch: {:?}",
        action.kind
    );
    let changes = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .context("edit changes present")?;
    let edits = changes.get(&uri).context("edits target the document")?;
    ensure!(edits.len() == 3, "3 edits expected, got {}", edits.len());
    ensure!(
        edits
            .windows(2)
            .all(|pair| matches!(pair, [left, right] if left.range.start >= right.range.start)),
        "edits must be in descending start order"
    );
    let helper = edits
        .iter()
        .find(|edit| edit.new_text.contains("private static"))
        .context("helper insertion present")?;
    ensure!(
        helper.new_text.contains("ExtractedFromCluster_abcdef"),
        "helper embeds the cluster id prefix: {}",
        helper.new_text
    );
    Ok(())
}

/// A range that touches no occurrence yields no actions.
#[test]
fn range_outside_occurrences_yields_nothing() -> Result<()> {
    let (_dir, path, uri, source, report) = fixture_setup()?;
    let range = Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 0,
            character: 5,
        },
    };
    let actions = build_for_range(&report, &path, &uri, source.as_bytes(), range);
    ensure!(actions.is_empty(), "no action outside occurrences");
    Ok(())
}

/// Unsupported file extensions have no parser and yield no actions.
#[test]
fn unsupported_language_yields_nothing() -> Result<()> {
    let (_dir, _path, uri, source, report) = fixture_setup()?;
    let unsupported = std::path::Path::new("InvoiceMath.txt");
    let range = Range {
        start: Position {
            line: 4,
            character: 8,
        },
        end: Position {
            line: 5,
            character: 0,
        },
    };
    let actions = build_for_range(&report, unsupported, &uri, source.as_bytes(), range);
    ensure!(actions.is_empty(), "no action for unsupported language");
    Ok(())
}

/// Non-UTF-8 buffers are refused outright — position math would be
/// meaningless.
#[test]
fn non_utf8_source_yields_nothing() -> Result<()> {
    let (_dir, path, uri, _source, report) = fixture_setup()?;
    let range = Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 9,
            character: 0,
        },
    };
    let actions = build_for_range(&report, &path, &uri, &[0xFF, 0xFE, 0x00], range);
    ensure!(actions.is_empty(), "no action for undecodable source");
    Ok(())
}

/// An eligible cluster whose slices differ (renamed) yields the lazy
/// `refactor.rewrite` offer with the cluster id stashed in `data`
/// ([AUTOFIX-MERGE-CODE-ACTION] step 1).
#[test]
fn non_extractable_cluster_yields_lazy_merge_offer() -> Result<()> {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../deslop/tests/fixtures/csharp-extract-type2/RateMath.cs"),
    )?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("RateMath.cs");
    fs::write(&path, &source)?;
    let uri = Url::from_file_path(&path).map_err(|()| anyhow!("absolute fixture path"))?;
    let first = source.find("var total = 0;").context("first body")?;
    let second = source.find("var sum = 0;").context("second body")?;
    let spans = (
        (first, first.saturating_add(200)),
        (second, second.saturating_add(190)),
    );
    let report = report_with_cluster(&path, spans);
    let range = Range {
        start: Position {
            line: 4,
            character: 8,
        },
        end: Position {
            line: 5,
            character: 0,
        },
    };
    let actions = build_for_range(&report, &path, &uri, source.as_bytes(), range);
    ensure!(actions.len() == 1, "one merge offer, got {}", actions.len());
    let CodeActionOrCommand::CodeAction(action) = actions.first().context("first")? else {
        return Err(anyhow!("expected an action literal"));
    };
    ensure!(
        action.title == MERGE_ACTION_TITLE,
        "merge title, got {}",
        action.title
    );
    ensure!(
        action.kind == Some(CodeActionKind::REFACTOR_REWRITE),
        "rewrite kind"
    );
    ensure!(action.edit.is_none(), "offer omits the edit");
    ensure!(
        offered_cluster_id(action).as_deref() == Some("abcdef0123456789"),
        "cluster id rides in data"
    );
    Ok(())
}

/// Resolution attaches the edit for a mechanical plan and disables the
/// action with the reason for a refusal ([AUTOFIX-MERGE-CODE-ACTION]
/// step 2).
#[test]
fn resolved_action_attaches_edit_or_disables() -> Result<()> {
    let offer = CodeAction {
        title: MERGE_ACTION_TITLE.to_owned(),
        kind: Some(CodeActionKind::REFACTOR_REWRITE),
        data: Some(serde_json::json!({ "cluster_id": "abcdef0123456789" })),
        ..CodeAction::default()
    };
    let mechanical = deslop_core::wire_generated::MergePlan {
        cluster_id: "abcdef0123456789".to_owned(),
        language: "csharp".to_owned(),
        verdict: deslop_core::wire_generated::MergeVerdict::Mechanical,
        helper_name: "MergedFromCluster_abcdef".to_owned(),
        helper_body: "var x = arg0;".to_owned(),
        parameters: Vec::new(),
        workspace_edit: Some(serde_json::json!({
            "documentChanges": [{
                "textDocument": { "uri": "file:///tmp/a.cs", "version": null },
                "edits": [{
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 0, "character": 1 }
                    },
                    "newText": "x"
                }]
            }]
        })),
    };
    let resolved = resolved_action(offer.clone(), &mechanical);
    ensure!(resolved.edit.is_some(), "mechanical plans attach the edit");
    ensure!(resolved.disabled.is_none(), "mechanical plans stay enabled");

    let refused = deslop_core::wire_generated::MergePlan {
        verdict: deslop_core::wire_generated::MergeVerdict::AiOrHuman {
            reason: "structural drift".to_owned(),
        },
        workspace_edit: None,
        ..mechanical
    };
    let disabled = resolved_action(offer, &refused);
    ensure!(disabled.edit.is_none(), "refusals attach no edit");
    ensure!(
        disabled
            .disabled
            .as_ref()
            .is_some_and(|entry| entry.reason == "structural drift"),
        "the refusal reason surfaces on the action"
    );
    Ok(())
}

/// A mechanical verdict whose edit is missing or undeserializable must
/// disable the action — an enabled action with no edit applies as a
/// silent no-op the user reads as success.
#[test]
fn resolved_action_disables_when_edit_is_unusable() -> Result<()> {
    let offer = CodeAction {
        title: MERGE_ACTION_TITLE.to_owned(),
        kind: Some(CodeActionKind::REFACTOR_REWRITE),
        data: Some(serde_json::json!({ "cluster_id": "abcdef0123456789" })),
        ..CodeAction::default()
    };
    let base = deslop_core::wire_generated::MergePlan {
        cluster_id: "abcdef0123456789".to_owned(),
        language: "csharp".to_owned(),
        verdict: deslop_core::wire_generated::MergeVerdict::Mechanical,
        helper_name: "MergedFromCluster_abcdef".to_owned(),
        helper_body: "var x = arg0;".to_owned(),
        parameters: Vec::new(),
        workspace_edit: None,
    };
    for (label, edit) in [
        ("missing edit", None),
        ("undeserializable edit", Some(serde_json::json!(42))),
    ] {
        let plan = deslop_core::wire_generated::MergePlan {
            workspace_edit: edit,
            ..base.clone()
        };
        let resolved = resolved_action(offer.clone(), &plan);
        ensure!(resolved.edit.is_none(), "{label}: no edit attaches");
        ensure!(
            resolved
                .disabled
                .as_ref()
                .is_some_and(|entry| entry.reason.contains("edit")),
            "{label}: the action must disable with a reason naming the missing edit"
        );
    }
    Ok(())
}
