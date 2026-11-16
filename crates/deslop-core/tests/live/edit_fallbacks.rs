//! [PIPELINE-INCREMENTAL-ANALYSIS-PAIRS] The live passes where the kept pairs must not be trusted, or must still be honoured, each compared with a cold session over the same tree ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]).
//!
//! Embedding pairs come from a nearest-neighbour search, not from groups, so with embeddings on every edit pairs the whole corpus. A pass that fails before it renders leaves its edits for the next pass to pair. A file whose bytes change while its code pieces stay identical is still re-paired, and must change nothing.

#![cfg(feature = "live")]

use std::{fs, os::unix::fs::PermissionsExt, path::Path, sync::Arc};

use anyhow::{anyhow, ensure, Context, Result};
use deslop_core::{
    embedding::{test_support::StubProvider, EmbeddingMode},
    live::AnalysisSession,
    report::Report,
};
use serde_json::Value;

use crate::common::{
    copy_fixture, live_session, CaptureSubscriber, CapturedEvents, DEFAULT_LIVE_MIN_NODES,
};

const FIXTURE: &str = "csharp-small";
/// The file every scenario edits; it shares no code with the clone pair.
const EDITED_FILE: &str = "Ledger.cs";
/// The fixture's clone pair; `Alpha.cs` sorts first, so it is the centre of every star the pair shares.
const CENTRE_FILE: &str = "Alpha.cs";
const CLONE_FILES: [&str; 2] = [CENTRE_FILE, "Beta.cs"];
/// The report field that records cache reuse, the one permitted difference from a cold report.
const CACHE_STATS_FIELD: &str = "cache_stats";
const STAGE_MESSAGE: &str = "cluster stage complete";
const PAIRING_STAGE: &str = "candidate_pairs";
const EDIT_ROUNDS: u8 = 3;
/// File modes that deny and restore reading.
const UNREADABLE: u32 = 0o000;
const READABLE: u32 = 0o644;
/// Bytes appended after the last line: they move no code piece.
const TRAILING_BLANK_LINES: &str = "\n\n\n";
/// The byte-identical pair the embedding scenario adds.
const TWIN_FILES: [&str; 2] = ["Twin.cs", "TwinMirror.cs"];
const TWIN_SOURCE: &str = "namespace Mirrors\n{\n    public class Mirror\n    {\n        public int Reflect(int depth)\n        {\n            var level = depth;\n            while (level > 1)\n            {\n                level = level / 2 + depth % 3;\n            }\n            return level * depth;\n        }\n    }\n}\n";
const EMBEDDING_MESSAGE: &str = "embedding pass complete";
/// One more entry per round, so every save changes structure.
const EXTRA_ENTRY: &str = "            totals.Add(owner.Length);\n";

/// A tally class unrelated to the fixture; `round` adds statements.
fn ledger_source(round: u8) -> String {
    let extra = EXTRA_ENTRY.repeat(usize::from(round));
    format!(
        "using System.Collections.Generic;\n\nnamespace Tallies\n{{\n    public class Tally\n    {{\n        private readonly List<int> totals = new List<int>();\n\n        public int Record(string owner)\n        {{\n{extra}            totals.Add(owner.GetHashCode());\n            totals.Reverse();\n            return totals.Count * 3 + totals[0];\n        }}\n    }}\n}}\n"
    )
}

/// A live session over `root` with the deterministic stub provider in `mode`.
fn stub_session(root: &Path, mode: EmbeddingMode) -> Result<AnalysisSession> {
    let provider = Arc::new(StubProvider::new());
    AnalysisSession::new_with_mode(
        root.to_path_buf(),
        DEFAULT_LIVE_MIN_NODES,
        false,
        None,
        provider,
        mode,
    )
    .context("embedding session")
}

/// A report as comparable JSON, without the cache-reuse record.
fn comparable(report: &Report) -> Result<Value> {
    let mut value = serde_json::to_value(report).context("serialise report")?;
    if let Some(fields) = value.as_object_mut() {
        drop(fields.remove(CACHE_STATS_FIELD));
    }
    Ok(value)
}

/// Asserts the live report equals `cold` and still carries the clone pair, whole.
fn assert_matches(live: &Report, cold: &Report, when: &str) -> Result<()> {
    assert_eq!(
        comparable(live)?,
        comparable(cold)?,
        "{when}: live report differs from cold"
    );
    let pair = live.clusters.iter().find(|cluster| {
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
    Ok(())
}

/// Runs `pass` under a capturing subscriber; returns its value and what it logged.
fn observed<T>(pass: impl FnOnce() -> Result<T>) -> Result<(T, CapturedEvents)> {
    let captured = CapturedEvents::default();
    let subscriber = CaptureSubscriber::new(captured.clone());
    let value = tracing::subscriber::with_default(subscriber, pass)?;
    Ok((value, captured))
}

/// One numeric field of the last `message` record whose `filter` field holds `value`.
fn last_count(
    captured: &CapturedEvents,
    message: &str,
    filter: Option<(&str, &str)>,
    field: &str,
) -> Result<u64> {
    let event = captured
        .events(message)?
        .into_iter()
        .rfind(|event| {
            filter
                .is_none_or(|(key, value)| event.values.get(key).map(String::as_str) == Some(value))
        })
        .ok_or_else(|| anyhow!("no {message} record"))?;
    event
        .values
        .get(field)
        .ok_or_else(|| anyhow!("{message} has no {field}"))?
        .parse()
        .with_context(|| format!("{field} is not a count"))
}

/// The records the last pairing stage was fed.
fn pairing_input(captured: &CapturedEvents) -> Result<u64> {
    last_count(
        captured,
        STAGE_MESSAGE,
        Some(("stage", PAIRING_STAGE)),
        "input",
    )
}

/// Saves round `round` of the edited file and feeds it through one pass.
fn save_edit(session: &mut AnalysisSession, root: &Path, round: u8) -> Result<()> {
    let path = root.join(EDITED_FILE);
    fs::write(&path, ledger_source(round)).with_context(|| format!("save round {round}"))?;
    session
        .apply_changes(&[path])
        .map(drop)
        .context("apply edit")
}

/// The fixture plus the edited file at round 0.
fn workspace() -> Result<tempfile::TempDir> {
    let workspace = copy_fixture(FIXTURE)?;
    fs::write(workspace.path().join(EDITED_FILE), ledger_source(0)).context("write ledger")?;
    Ok(workspace)
}

/// Two byte-identical files: the embedding pass pairs identical snippets, so every pass has embedding pairs.
fn add_twins(root: &Path) -> Result<()> {
    for name in TWIN_FILES {
        fs::write(root.join(name), TWIN_SOURCE).with_context(|| format!("write {name}"))?;
    }
    Ok(())
}

#[test]
fn with_embeddings_on_every_edit_pairs_the_whole_corpus_and_matches_cold() -> Result<()> {
    let workspace = workspace()?;
    let root = workspace.path();
    add_twins(root)?;
    let mut session = stub_session(root, EmbeddingMode::Auto)?;
    for round in 1..=EDIT_ROUNDS {
        let ((), edit) = observed(|| save_edit(&mut session, root, round))?;
        let (cold, whole) = observed(|| stub_session(root, EmbeddingMode::Auto))?;
        let embedding_pairs = last_count(&edit, EMBEDDING_MESSAGE, None, "pair_count")?;
        ensure!(
            embedding_pairs > 0,
            "round {round}: the edit pass must find embedding pairs, or this test is blind"
        );
        assert_eq!(
            pairing_input(&edit)?,
            pairing_input(&whole)?,
            "round {round}: with embeddings on, an edit pairs every record"
        );
        assert_matches(
            &session.report(),
            &cold.report(),
            &format!("embeddings round {round}"),
        )?;
    }
    Ok(())
}

/// Sets `path`'s permission bits.
fn set_mode(path: &Path, mode: u32) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .with_context(|| format!("chmod {mode:o} {}", path.display()))
}

#[test]
fn an_edit_after_a_failed_pass_matches_cold() -> Result<()> {
    let workspace = workspace()?;
    let root = workspace.path();
    let mut session = live_session(root)?;
    let (edited, centre) = (root.join(EDITED_FILE), root.join(CENTRE_FILE));
    fs::write(&edited, ledger_source(1)).context("save round 1")?;
    set_mode(&centre, UNREADABLE)?;
    let failed = session.apply_changes(&[edited.clone(), centre.clone()]);
    set_mode(&centre, READABLE)?;
    ensure!(
        failed.is_err(),
        "an unreadable file must fail the pass: {failed:?}"
    );
    session
        .apply_changes(&[centre])
        .map(drop)
        .context("re-read the centre")?;
    assert_matches(
        &session.report(),
        &live_session(root)?.report(),
        "after the failed pass",
    )?;
    for round in 2..=EDIT_ROUNDS {
        save_edit(&mut session, root, round)?;
        assert_matches(
            &session.report(),
            &live_session(root)?.report(),
            &format!("round {round}"),
        )?;
    }
    Ok(())
}

#[test]
fn a_byte_change_that_moves_no_code_matches_cold() -> Result<()> {
    let workspace = workspace()?;
    let root = workspace.path();
    let mut session = live_session(root)?;
    for round in 1..=EDIT_ROUNDS {
        for name in CLONE_FILES {
            let path = root.join(name);
            let mut source = fs::read_to_string(&path).with_context(|| format!("read {name}"))?;
            source.push_str(TRAILING_BLANK_LINES);
            fs::write(&path, source).with_context(|| format!("pad {name}"))?;
            let before = session.generation();
            session
                .apply_changes(&[path])
                .map(drop)
                .with_context(|| format!("apply {name}"))?;
            let when = format!("round {round}, {name} padded");
            ensure!(
                session.generation() > before,
                "{when}: a byte change publishes a generation"
            );
            assert_matches(&session.report(), &live_session(root)?.report(), &when)?;
        }
    }
    Ok(())
}
