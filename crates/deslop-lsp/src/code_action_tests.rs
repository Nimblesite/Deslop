use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, ensure, Context, Result};
use deslop_core::{
    report::{CacheStats, Report, ReportCluster, ReportOccurrence, ReportSignals},
    report_metrics::RepoMetrics,
    wire_generated::{MergePlan, MergeVerdict},
};
use tower_lsp::lsp_types::{CodeActionOrCommand, Position, Range};

use super::*;

/// Byte spans of the two duplicated statement runs in a fixture.
type OccurrenceSpans = ((usize, usize), (usize, usize));

/// Cluster id every report built here carries.
const CLUSTER_ID: &str = "abcdef0123456789";

/// Byte-proven signals for the fixture cluster: shape saturated, so
/// `shape` is 1.0 and the evidence verdict comes from the engine rather
/// than a second reading of the same numbers here.
const IDENTICAL_SIGNALS: ReportSignals = ReportSignals {
    structural: 1.0,
    token_jaccard: 1.0,
    shape: 1.0,
    embedding_cos: 0.0,
    pair_agreement: 1.0,
    pair_rename_consistency: 0.0,
    literal_fraction: 0.0,
};

/// Builds an LSP range from `(line, character)` pairs.
fn range(start: (u32, u32), end: (u32, u32)) -> Range {
    let position = |(line, character): (u32, u32)| Position { line, character };
    Range {
        start: position(start),
        end: position(end),
    }
}

/// The range covering the first duplicated run in both C# fixtures.
fn occurrence_range() -> Range {
    range((4, 8), (5, 0))
}

/// Byte offset of the `nth` match of `needle` in `source`.
fn nth_match(source: &str, needle: &str, nth: usize) -> Option<usize> {
    source.match_indices(needle).nth(nth).map(|(at, _)| at)
}

/// Byte spans of the two duplicated statement runs in the Type-1 fixture.
fn occurrence_spans(source: &str) -> Result<OccurrenceSpans> {
    let end_needle = "return total;";
    let run = |nth: usize| -> Result<(usize, usize)> {
        let start =
            nth_match(source, "var total = 0;", nth).with_context(|| format!("run {nth} start"))?;
        let end = nth_match(source, end_needle, nth).with_context(|| format!("run {nth} end"))?;
        Ok((start, end.saturating_add(end_needle.len())))
    };
    Ok((run(0)?, run(1)?))
}

/// Byte spans of the two renamed statement runs in the Type-2 fixture.
fn renamed_spans(source: &str) -> Result<OccurrenceSpans> {
    let first = source.find("var total = 0;").context("first body")?;
    let second = source.find("var sum = 0;").context("second body")?;
    Ok((
        (first, first.saturating_add(200)),
        (second, second.saturating_add(190)),
    ))
}

/// Wraps one proven-Identical cluster over the fixture spans in a
/// minimal report.
fn report_with_cluster(path: &Path, spans: OccurrenceSpans) -> Report {
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
        clusters: vec![fixture_target_cluster(occurrences)],
        clusters_outside_diff: None,
    }
}

/// The fixture's one byte-proven cluster, at the id the autofix suites
/// address it by.
fn fixture_target_cluster(occurrences: Vec<ReportOccurrence>) -> ReportCluster {
    let mut cluster = deslop_core::report_fixtures::fixture_cluster(CLUSTER_ID, occurrences);
    cluster.weight = 10.0;
    cluster.canonical_node_count = 40;
    cluster.signals = IDENTICAL_SIGNALS;
    deslop_core::report_fixtures::restamp_fixture(&mut cluster);
    cluster
}

fn occurrence(path: &Path, span: (usize, usize)) -> ReportOccurrence {
    ReportOccurrence {
        path: path.to_path_buf(),
        start_byte: span.0,
        end_byte: span.1,
        start_line: 0,
        end_line: 0,
        hidden: false,
        in_diff: None,
    }
}

/// A checked-in E2E fixture copied into a temp dir, paired with the one-cluster
/// report covering its duplicated spans ([AUTOFIX-EXTRACT-TESTING]).
struct Fixture {
    /// Kept alive so the written document stays on disk for the test.
    _dir: tempfile::TempDir,
    path: PathBuf,
    uri: Url,
    source: String,
    report: Report,
}

impl Fixture {
    /// Actions offered for `range` over this fixture's own document.
    fn actions(&self, range: Range) -> Vec<CodeActionOrCommand> {
        self.actions_for(&self.path, self.source.as_bytes(), range)
    }

    /// Actions offered for `range` against a substitute path and buffer.
    fn actions_for(&self, path: &Path, source: &[u8], range: Range) -> Vec<CodeActionOrCommand> {
        build_for_range(&self.report, path, &self.uri, source, range)
    }
}

/// Copies `relative` out of the E2E fixture tree shared with the CLI
/// suites, clustering the two spans `spans_of` finds in it.
fn clustered_fixture(
    relative: &str,
    spans_of: impl Fn(&str) -> Result<OccurrenceSpans>,
) -> Result<Fixture> {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../deslop/tests/fixtures");
    let source = fs::read_to_string(fixtures.join(relative))?;
    let name = Path::new(relative)
        .file_name()
        .context("fixture file name")?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join(name);
    fs::write(&path, &source)?;
    let uri = Url::from_file_path(&path).map_err(|()| anyhow!("absolute fixture path"))?;
    let report = report_with_cluster(&path, spans_of(&source)?);
    Ok(Fixture {
        _dir: dir,
        path,
        uri,
        source,
        report,
    })
}

/// The shared C# Type-1 fixture backing the extract scenarios.
fn type1_fixture() -> Result<Fixture> {
    clustered_fixture("csharp-extract-type1/InvoiceMath.cs", occurrence_spans)
}

/// Unwraps the code-action literal from one offered entry.
fn action_literal(entry: &CodeActionOrCommand) -> Result<&CodeAction> {
    let CodeActionOrCommand::CodeAction(action) = entry else {
        return Err(anyhow!("expected a code action literal"));
    };
    Ok(action)
}

/// The lazy merge offer the server hands back before resolution.
fn merge_offer() -> CodeAction {
    CodeAction {
        title: MERGE_ACTION_TITLE.to_owned(),
        kind: Some(CodeActionKind::REFACTOR_REWRITE),
        data: Some(serde_json::json!({ "cluster_id": CLUSTER_ID })),
        ..CodeAction::default()
    }
}

/// A mechanical merge plan carrying `workspace_edit` verbatim.
fn mechanical_plan(workspace_edit: Option<serde_json::Value>) -> MergePlan {
    MergePlan {
        cluster_id: CLUSTER_ID.to_owned(),
        language: "csharp".to_owned(),
        verdict: MergeVerdict::Mechanical,
        helper_name: "MergedFromCluster_abcdef".to_owned(),
        helper_body: "var x = arg0;".to_owned(),
        parameters: Vec::new(),
        workspace_edit,
    }
}

/// A single-edit `WorkspaceEdit` payload as the merge planner emits it.
fn mechanical_workspace_edit() -> serde_json::Value {
    serde_json::json!({
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
    })
}

/// [AUTOFIX-EXTRACT-CODE-ACTION]: an eligible cluster intersecting the
/// range yields one complete action — exact title, exact kind, one
/// insertion plus two call rewrites in descending order, all targeting
/// the requested document.
#[test]
fn eligible_cluster_yields_one_complete_action() -> Result<()> {
    let fixture = type1_fixture()?;
    let actions = fixture.actions(occurrence_range());
    ensure!(
        actions.len() == 1,
        "exactly one action, got {}",
        actions.len()
    );
    let action = action_literal(actions.first().context("first action")?)?;
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
    let edits = changes
        .get(&fixture.uri)
        .context("edits target the document")?;
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
    let fixture = type1_fixture()?;
    let actions = fixture.actions(range((0, 0), (0, 5)));
    ensure!(actions.is_empty(), "no action outside occurrences");
    Ok(())
}

/// Unsupported file extensions have no parser and yield no actions.
#[test]
fn unsupported_language_yields_nothing() -> Result<()> {
    let fixture = type1_fixture()?;
    let unsupported = Path::new("InvoiceMath.txt");
    let actions = fixture.actions_for(unsupported, fixture.source.as_bytes(), occurrence_range());
    ensure!(actions.is_empty(), "no action for unsupported language");
    Ok(())
}

/// Non-UTF-8 buffers are refused outright — position math would be
/// meaningless.
#[test]
fn non_utf8_source_yields_nothing() -> Result<()> {
    let fixture = type1_fixture()?;
    let actions = fixture.actions_for(&fixture.path, &[0xFF, 0xFE, 0x00], range((0, 0), (9, 0)));
    ensure!(actions.is_empty(), "no action for undecodable source");
    Ok(())
}

/// An eligible cluster whose slices differ (renamed) yields the lazy
/// `refactor.rewrite` offer with the cluster id stashed in `data`
/// ([AUTOFIX-MERGE-CODE-ACTION] step 1).
#[test]
fn non_extractable_cluster_yields_lazy_merge_offer() -> Result<()> {
    let fixture = clustered_fixture("csharp-extract-type2/RateMath.cs", renamed_spans)?;
    let actions = fixture.actions(occurrence_range());
    ensure!(actions.len() == 1, "one merge offer, got {}", actions.len());
    let action = action_literal(actions.first().context("first")?)?;
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
    let offer = merge_offer();
    let mechanical = mechanical_plan(Some(mechanical_workspace_edit()));
    let resolved = resolved_action(offer.clone(), &mechanical);
    ensure!(resolved.edit.is_some(), "mechanical plans attach the edit");
    ensure!(resolved.disabled.is_none(), "mechanical plans stay enabled");

    let refused = MergePlan {
        verdict: MergeVerdict::AiOrHuman {
            reason: "structural drift".to_owned(),
        },
        ..mechanical_plan(None)
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
    let offer = merge_offer();
    for (label, edit) in [
        ("missing edit", None),
        ("undeserializable edit", Some(serde_json::json!(42))),
    ] {
        let resolved = resolved_action(offer.clone(), &mechanical_plan(edit));
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
