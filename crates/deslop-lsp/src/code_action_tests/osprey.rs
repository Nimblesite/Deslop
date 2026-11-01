//! [LANG-CAND-OSPREY] Parser identity governs available editor refactors.

use super::*;
use deslop_core::lang::LanguageParser;

/// Existing duplicated Python implementation used as the ordinary-Python control.
const PYTHON_FIXTURE: &str = "python-extract-type1/metrics.py";
/// Osprey's Python syntax has its own refactor capability boundary.
const OSPREY_PYTHON: &str = "osprey-python";
/// Range covering the first duplicated implementation in the shared fixture.
const BODY_RANGE: ((u32, u32), (u32, u32)) = ((1, 4), (7, 16));
/// Each eligible cluster contributes one action.
const ACTION_COUNT: usize = 1;

#[test]
fn recorded_osprey_identity_controls_python_shaped_actions() -> Result<()> {
    let mut fixture = clustered_fixture(PYTHON_FIXTURE, python_body_spans)?;
    let selection = range(BODY_RANGE.0, BODY_RANGE.1);
    let control = fixture.actions(selection);
    ensure!(single_action(&control)?.title == EXTRACT_ACTION_TITLE);
    select_osprey(&mut fixture.report);
    assert_osprey_action(&fixture.actions(selection))
}

/// Preserve manifest/buffer-selected identity on every occurrence.
fn select_osprey(report: &mut Report) {
    for occurrence in report
        .clusters
        .iter_mut()
        .flat_map(|cluster| &mut cluster.occurrences)
    {
        occurrence.language = Some(OSPREY_PYTHON.to_owned());
    }
}

/// Both flavors must retain their one eligible action.
fn single_action(actions: &[CodeActionOrCommand]) -> Result<&CodeAction> {
    ensure!(actions.len() == ACTION_COUNT, "one eligible action remains");
    action_literal(actions.first().context("eligible action")?)
}

/// Osprey offers review rather than an unsupported automatic Python rewrite.
fn assert_osprey_action(actions: &[CodeActionOrCommand]) -> Result<()> {
    let action = single_action(actions)?;
    ensure!(
        action.title == MERGE_ACTION_TITLE,
        "Osprey does not inherit Python extraction"
    );
    ensure!(action.kind == Some(CodeActionKind::REFACTOR_REWRITE));
    ensure!(
        action.edit.is_none(),
        "no unimplemented Osprey refactor may edit code"
    );
    ensure!(offered_cluster_id(action).as_deref() == Some(CLUSTER_ID));
    Ok(())
}

/// Select whole body nodes from the real CST, never source text patterns.
fn python_body_spans(source: &str) -> Result<OccurrenceSpans> {
    let parser = deslop_core::lang::python::PythonParser::new();
    let tree =
        deslop_core::lang::shared::parse_source(parser.id(), &parser.grammar(), source.as_bytes())?;
    let mut cursor = tree.root_node().walk();
    let spans: Vec<_> = tree
        .root_node()
        .named_children(&mut cursor)
        .filter(|node| node.kind() == "function_definition")
        .filter_map(|node| node.child_by_field_name("body"))
        .map(|body| (body.start_byte(), body.end_byte()))
        .collect();
    let [first, second] = spans
        .try_into()
        .map_err(|_| anyhow!("exactly two bodies required"))?;
    Ok((first, second))
}
