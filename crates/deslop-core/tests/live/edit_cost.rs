//! [LIVE-PERF-BUDGETS] A live edit costs work in proportion to the file it touched.
//!
//! A production `deslop-lsp` over the Osprey checkout (2,935 files, 1.24 million fingerprints) fed every fingerprint of the workspace back through candidate pairing on each saved edit: 33.6 million LSH pair visits and a 2–3 GB working set per pass, 11–15 s on an idle machine and up to 1,000 s on a busy one, nine passes in six minutes while an agent edited. The incremental budget is 500 ms for ten changed files.
//!
//! Wall time and resident memory depend on the machine, so this suite pins the work that causes them: the counts the pairing stage reports for one pass. Black-box: it drives [`AnalysisSession`] the way a watcher event does and reads the pipeline's own `tracing` records.

#![cfg(feature = "live")]

use std::{fs, path::Path};

use anyhow::{anyhow, Context, Result};
use deslop_core::{live::AnalysisSession, report::Report};

use crate::common::{copy_fixture, live_session, CaptureSubscriber, CapturedEvents};

const FIXTURE: &str = "csharp-small";
/// The file the test edits; it shares no structure with the fixture's clone pair.
const EDITED_FILE: &str = "Ledger.cs";
const CLONE_FILES: [&str; 2] = ["Alpha.cs", "Beta.cs"];
const WORKSPACE_FILE_COUNT: usize = 3;
const STAGE_MESSAGE: &str = "cluster stage complete";
const PAIRING_STAGE: &str = "candidate_pairs";
const LSH_MESSAGE: &str = "pairs: post-lsh";
/// Successive saves of the edited file, each a structural change.
const EDIT_ROUNDS: u8 = 3;

/// One more statement in the edited method.
const EXTRA_STATEMENT: &str = "            entries.Add(\"extra\");\n";

/// A ledger class unrelated to the fixture; `round` adds statements so every save changes structure.
fn ledger_source(round: u8) -> String {
    let extra = EXTRA_STATEMENT.repeat(usize::from(round));
    format!(
        "using System.Collections.Generic;\n\nnamespace Books\n{{\n    public class Ledger\n    {{\n        private readonly List<string> entries = new List<string>();\n\n        public string Describe(string owner)\n        {{\n{extra}            entries.Add(owner);\n            entries.Sort();\n            return string.Join(\"|\", entries) + \":\" + entries.Count;\n        }}\n    }}\n}}\n"
    )
}

/// What the pairing stage reported for one pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PairingWork {
    /// Fingerprints fed into candidate pairing.
    fingerprints: u64,
    /// LSH pair visits.
    lsh_scanned: u64,
}

/// Reads the last pass's pairing counts out of the captured records.
fn pairing_work(captured: &CapturedEvents) -> Result<PairingWork> {
    let stage = captured
        .events(STAGE_MESSAGE)?
        .into_iter()
        .rfind(|event| event.values.get("stage").map(String::as_str) == Some(PAIRING_STAGE))
        .ok_or_else(|| anyhow!("no {PAIRING_STAGE} record: {:?}", captured.all()))?;
    let lsh = captured
        .events(LSH_MESSAGE)?
        .pop()
        .ok_or_else(|| anyhow!("no {LSH_MESSAGE} record"))?;
    Ok(PairingWork {
        fingerprints: number(&stage.values, "input")?,
        lsh_scanned: number(&lsh.values, "lsh_scanned")?,
    })
}

/// Parses one numeric field of a captured record.
fn number(values: &std::collections::BTreeMap<String, String>, name: &str) -> Result<u64> {
    values
        .get(name)
        .ok_or_else(|| anyhow!("record has no {name}: {values:?}"))?
        .parse()
        .with_context(|| format!("{name} is not a count"))
}

/// Runs `pass` under a capturing subscriber and returns its result with the pairing work it logged.
fn observed<T>(pass: impl FnOnce() -> Result<T>) -> Result<(T, PairingWork)> {
    let captured = CapturedEvents::default();
    let subscriber = CaptureSubscriber::new(captured.clone());
    let value = tracing::subscriber::with_default(subscriber, pass)?;
    Ok((value, pairing_work(&captured)?))
}

/// The pairing work a workspace holding only `source` costs to analyse from cold.
fn cold_cost_of(source: &str) -> Result<PairingWork> {
    let alone = tempfile::tempdir().context("tempdir")?;
    fs::write(alone.path().join(EDITED_FILE), source).context("write lone file")?;
    observed(|| live_session(alone.path())).map(|(_, work)| work)
}

/// Asserts the report still carries the fixture's clone pair, whole.
fn assert_clone_pair_reported(report: &Report, when: &str) {
    assert_eq!(
        report.files_analysed, WORKSPACE_FILE_COUNT,
        "{when}: files analysed"
    );
    let pair = report.clusters.iter().find(|cluster| {
        CLONE_FILES.iter().all(|name| {
            cluster
                .occurrences
                .iter()
                .any(|occurrence| occurrence.path.ends_with(name))
        })
    });
    let occurrences = pair.map(|cluster| cluster.occurrences.len());
    assert_eq!(
        occurrences,
        Some(CLONE_FILES.len()),
        "{when}: Alpha/Beta pair, two occurrences"
    );
}

/// Saves a structural edit to the lone unrelated file and feeds it through one pass.
fn save_edit(session: &mut AnalysisSession, root: &Path, round: u8) -> Result<()> {
    let path = root.join(EDITED_FILE);
    fs::write(&path, ledger_source(round)).with_context(|| format!("save round {round}"))?;
    session
        .apply_changes(&[path])
        .map(drop)
        .context("apply edit")
}

#[test]
fn an_edit_to_one_file_pairs_only_that_file() -> Result<()> {
    let workspace = copy_fixture(FIXTURE)?;
    let root = workspace.path();
    fs::write(root.join(EDITED_FILE), ledger_source(0)).context("write ledger")?;

    // Interaction 1: the cold pass pairs the whole workspace, clone pair included.
    let (mut session, cold) = observed(|| live_session(root))?;
    assert_clone_pair_reported(&session.report(), "cold");
    assert!(
        cold.lsh_scanned > 0,
        "the clone pair must reach LSH pairing, or this test is blind"
    );

    // Interactions 2..: each save of the unrelated file is a real edit, and costs no more pairing than that file alone.
    for round in 1..=EDIT_ROUNDS {
        let before = session.generation();
        let ((), edit) = observed(|| save_edit(&mut session, root, round))?;
        let alone = cold_cost_of(&ledger_source(round))?;
        assert!(
            session.generation() > before,
            "round {round}: a real edit publishes a generation"
        );
        assert_clone_pair_reported(&session.report(), "after edit");
        assert!(
            edit.fingerprints <= alone.fingerprints,
            "round {round}: pairing was fed {} fingerprints for an edit to a file that holds {}; the cold workspace pass was fed {}",
            edit.fingerprints, alone.fingerprints, cold.fingerprints,
        );
        assert!(
            edit.lsh_scanned <= alone.lsh_scanned,
            "round {round}: {} LSH pair visits for an edit whose file alone costs {}; the untouched Alpha/Beta pair was rescanned (cold pass: {})",
            edit.lsh_scanned, alone.lsh_scanned, cold.lsh_scanned,
        );
    }
    Ok(())
}
