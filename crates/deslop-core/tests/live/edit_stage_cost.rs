//! [PIPELINE-INCREMENTAL-ANALYSIS-VERDICTS] After a live edit, the stages that measure each candidate pair (the shared-subtree rescue and the content gate) measure only the pairs the edit could have changed.
//!
//! Wall time depends on the machine, so this suite pins the work: the `input` each stage records for one pass. An edit to one unrelated file may re-measure no more pairs than that file holds on its own; the untouched clone pair keeps its verdicts and stays reported. Black-box: it drives [`AnalysisSession`] the way a watcher event does and reads the pipeline's own `tracing` records.

#![cfg(feature = "live")]

use std::{fs, path::Path};

use anyhow::{anyhow, Context, Result};
use deslop_core::{live::AnalysisSession, report::Report};

use crate::common::{copy_fixture, live_session, CaptureSubscriber, CapturedEvents};

const FIXTURE: &str = "csharp-small";
/// The file the test edits; it shares no structure with the fixture's clone pair.
const EDITED_FILE: &str = "Inventory.cs";
const CLONE_FILES: [&str; 2] = ["Alpha.cs", "Beta.cs"];
const STAGE_MESSAGE: &str = "cluster stage complete";
/// The per-pair measuring stages, in pipeline order.
const MEASURING_STAGES: [&str; 2] = ["shared_subtree_rescue", "pair_content_gate"];
/// Successive saves of the edited file, each a structural change.
const EDIT_ROUNDS: u8 = 3;
/// One more statement per round.
const EXTRA_STATEMENT: &str = "            stock[name] = stock.Count;\n";

/// A stock class unrelated to the fixture; `round` adds statements so every save changes structure.
fn inventory_source(round: u8) -> String {
    let extra = EXTRA_STATEMENT.repeat(usize::from(round));
    format!(
        "using System.Collections.Generic;\n\nnamespace Stores\n{{\n    public class Inventory\n    {{\n        private readonly Dictionary<string, int> stock = new Dictionary<string, int>();\n\n        public int Restock(string name, int amount)\n        {{\n{extra}            stock.TryGetValue(name, out var held);\n            stock[name] = held + amount * 2;\n            return stock[name] - held;\n        }}\n    }}\n}}\n"
    )
}

/// The `input` each measuring stage recorded in the last pass, in stage order.
fn measured_inputs(captured: &CapturedEvents) -> Result<[u64; 2]> {
    let events = captured.events(STAGE_MESSAGE)?;
    let input = |stage: &str| -> Result<u64> {
        events
            .iter()
            .rfind(|event| event.values.get("stage").map(String::as_str) == Some(stage))
            .and_then(|event| event.values.get("input"))
            .ok_or_else(|| anyhow!("no {stage} record: {:?}", captured.all()))?
            .parse()
            .with_context(|| format!("{stage} input is not a count"))
    };
    Ok([input(MEASURING_STAGES[0])?, input(MEASURING_STAGES[1])?])
}

/// Runs `pass` under a capturing subscriber; returns its value and the measuring stages' inputs.
fn observed<T>(pass: impl FnOnce() -> Result<T>) -> Result<(T, [u64; 2])> {
    let captured = CapturedEvents::default();
    let subscriber = CaptureSubscriber::new(captured.clone());
    let value = tracing::subscriber::with_default(subscriber, pass)?;
    Ok((value, measured_inputs(&captured)?))
}

/// The measuring work a workspace holding only `source` costs from cold.
fn cold_cost_of(source: &str) -> Result<[u64; 2]> {
    let alone = tempfile::tempdir().context("tempdir")?;
    fs::write(alone.path().join(EDITED_FILE), source).context("write lone file")?;
    observed(|| live_session(alone.path())).map(|(_, inputs)| inputs)
}

/// Asserts the report still carries the fixture's clone pair, whole.
fn assert_clone_pair_reported(report: &Report, when: &str) {
    let pair = report.clusters.iter().find(|cluster| {
        CLONE_FILES.iter().all(|name| {
            cluster
                .occurrences
                .iter()
                .any(|occurrence| occurrence.path.ends_with(name))
        })
    });
    assert_eq!(
        pair.map(|cluster| cluster.occurrences.len()),
        Some(CLONE_FILES.len()),
        "{when}: Alpha/Beta pair, two occurrences"
    );
}

/// Saves a structural edit to the unrelated file and feeds it through one pass.
fn save_edit(session: &mut AnalysisSession, root: &Path, round: u8) -> Result<()> {
    let path = root.join(EDITED_FILE);
    fs::write(&path, inventory_source(round)).with_context(|| format!("save round {round}"))?;
    session
        .apply_changes(&[path])
        .map(drop)
        .context("apply edit")
}

#[test]
fn an_edit_to_one_file_measures_only_that_files_pairs() -> Result<()> {
    let workspace = copy_fixture(FIXTURE)?;
    let root = workspace.path();
    fs::write(root.join(EDITED_FILE), inventory_source(0)).context("write inventory")?;

    // Interaction 1: the cold pass measures the clone pair, so the stages are not blind.
    let (mut session, cold) = observed(|| live_session(root))?;
    assert_clone_pair_reported(&session.report(), "cold");
    for (stage, input) in MEASURING_STAGES.iter().zip(cold) {
        assert!(
            input > 0,
            "cold {stage} must measure the clone pair, or this test is blind"
        );
    }

    // Interactions 2..: each save measures no more pairs than the edited file holds alone.
    for round in 1..=EDIT_ROUNDS {
        let before = session.generation();
        let ((), edit) = observed(|| save_edit(&mut session, root, round))?;
        let alone = cold_cost_of(&inventory_source(round))?;
        assert!(
            session.generation() > before,
            "round {round}: a real edit publishes a generation"
        );
        assert_clone_pair_reported(&session.report(), "after edit");
        for ((stage, edited), lone) in MEASURING_STAGES.iter().zip(edit).zip(alone) {
            assert!(
                edited <= lone,
                "round {round}: {stage} measured {edited} pairs for an edit to a file that holds {lone} alone; the cold workspace measured {}",
                cold[MEASURING_STAGES.iter().position(|name| name == stage).unwrap_or(0)],
            );
        }
    }
    Ok(())
}
