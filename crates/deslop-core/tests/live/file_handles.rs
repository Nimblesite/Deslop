//! [LIVE-SCHEDULER-REGISTRY-BOUNDED] A live session holds one file handle per analysed file: deletes release handles, and editor-style saves (write a temporary copy, rename it over the original) do not add any.
//!
//! Black-box: drives [`AnalysisSession::apply_changes`] the way the watcher does and reads the handle count the session publishes.

#![cfg(feature = "live")]

use std::{fs, path::Path};

use anyhow::{anyhow, Context, Result};
use deslop_core::live::AnalysisSession;

use crate::common::live_session;

/// Files that stay in the workspace for the whole test.
const RESIDENT_FILES: [&str; 2] = ["Ledger.cs", "Journal.cs"];
/// The file deleted, recreated and saved through a rename every round.
const CHURNED_FILE: &str = "Register.cs";
/// The temporary copy an editor writes before renaming it over the original.
const TEMPORARY_COPY: &str = "Register.cs.tmp~";
/// Delete/recreate/rename-save cycles; enough that unbounded growth shows.
const CHURN_ROUNDS: u32 = 6;

/// A small class named after its file; `round` changes the body each save.
fn class_source(stem: &str, round: u32) -> String {
    format!(
        "namespace Books\n{{\n    public class {stem}\n    {{\n        public int Settle(int[] amounts)\n        {{\n            var total = {round};\n            foreach (var amount in amounts)\n            {{\n                total = total + amount * 2;\n            }}\n            return total;\n        }}\n    }}\n}}\n"
    )
}

/// Feeds `paths` through one pass, as a watcher flush would.
fn apply(session: &mut AnalysisSession, paths: &[&Path]) -> Result<()> {
    let owned: Vec<_> = paths.iter().map(|path| path.to_path_buf()).collect();
    session
        .apply_changes(&owned)
        .map(drop)
        .context("apply changes")
}

/// Asserts the handle count equals the analysed file count and the file count on disk.
fn assert_one_handle_per_file(session: &AnalysisSession, on_disk: usize, when: &str) -> Result<()> {
    let pipeline = session
        .pipeline()
        .ok_or_else(|| anyhow!("{when}: no installed pipeline"))?;
    let analysed = session.report().files_analysed;
    assert_eq!(analysed, on_disk, "{when}: every file on disk is analysed");
    assert_eq!(
        pipeline.registry().len(),
        analysed,
        "{when}: one handle per analysed file, none left behind by removed files"
    );
    Ok(())
}

#[test]
fn deletes_and_rename_saves_leave_one_handle_per_file() -> Result<()> {
    let workspace = tempfile::tempdir().context("tempdir")?;
    let root = workspace.path();
    for name in RESIDENT_FILES.iter().chain([&CHURNED_FILE]) {
        let stem = name.trim_end_matches(".cs");
        fs::write(root.join(name), class_source(stem, 0))
            .with_context(|| format!("write {name}"))?;
    }
    let all_files = RESIDENT_FILES.len() + 1;
    let mut session = live_session(root)?;
    assert_one_handle_per_file(&session, all_files, "cold")?;

    let churned = root.join(CHURNED_FILE);
    let temporary = root.join(TEMPORARY_COPY);
    for round in 1..=CHURN_ROUNDS {
        fs::remove_file(&churned).context("delete")?;
        apply(&mut session, &[&churned])?;
        assert_one_handle_per_file(
            &session,
            RESIDENT_FILES.len(),
            &format!("round {round}: deleted"),
        )?;

        fs::write(&churned, class_source("Register", round)).context("recreate")?;
        apply(&mut session, &[&churned])?;
        assert_one_handle_per_file(&session, all_files, &format!("round {round}: recreated"))?;

        fs::write(&temporary, class_source("Register", round + CHURN_ROUNDS))
            .context("temp copy")?;
        fs::rename(&temporary, &churned).context("rename over")?;
        apply(&mut session, &[&temporary, &churned])?;
        assert_one_handle_per_file(&session, all_files, &format!("round {round}: rename-saved"))?;
        assert!(
            session
                .pipeline()
                .and_then(|pipeline| pipeline.file_id_for(&churned))
                .is_some(),
            "round {round}: the saved file resolves to a live handle"
        );
    }
    Ok(())
}

/// A file too deeply nested to analyse, present before the session starts.
const UNANALYSABLE_FILE: &str = "Deep.dart";
/// Bracket depth past the parser's nesting limit.
const UNANALYSABLE_DEPTH: usize = 5000;
/// What the file holds once its author repairs it.
const REPAIRED_SOURCE: &str =
    "int settle(int a, int b) { final total = a + b; return total * 2; }\n";

/// The handle count and the analysed file count the session publishes.
fn handles_and_analysed(session: &AnalysisSession, when: &str) -> Result<(usize, usize)> {
    let pipeline = session
        .pipeline()
        .ok_or_else(|| anyhow!("{when}: no installed pipeline"))?;
    Ok((pipeline.registry().len(), session.report().files_analysed))
}

#[test]
fn a_file_skipped_at_startup_holds_no_handle_before_or_after_it_is_repaired() -> Result<()> {
    let workspace = tempfile::tempdir().context("tempdir")?;
    let root = workspace.path();
    for name in RESIDENT_FILES {
        let stem = name.trim_end_matches(".cs");
        fs::write(root.join(name), class_source(stem, 0))
            .with_context(|| format!("write {name}"))?;
    }
    let unanalysable = root.join(UNANALYSABLE_FILE);
    let nested = format!(
        "var x = {}{};\n",
        "[".repeat(UNANALYSABLE_DEPTH),
        "]".repeat(UNANALYSABLE_DEPTH)
    );
    fs::write(&unanalysable, nested).context("write unanalysable")?;

    let mut session = live_session(root)?;
    let (handles, analysed) = handles_and_analysed(&session, "cold")?;
    assert_eq!(
        analysed,
        RESIDENT_FILES.len(),
        "cold: the file that cannot be analysed is skipped, the rest are analysed"
    );
    assert_eq!(
        handles, analysed,
        "cold: a skipped file is not analysed, so it holds no handle"
    );

    fs::write(&unanalysable, REPAIRED_SOURCE).context("repair")?;
    apply(&mut session, &[&unanalysable])?;
    let (handles, analysed) = handles_and_analysed(&session, "repaired")?;
    assert_eq!(
        analysed,
        RESIDENT_FILES.len() + 1,
        "repaired: the file joins the analysed set"
    );
    assert_eq!(
        handles, analysed,
        "repaired: one handle per analysed file, none left from when it was skipped"
    );
    Ok(())
}
