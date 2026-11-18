//! [LIVE-SCHEDULER-HEAP-STEADY] [TEST-MEMORY-BINARY] A live session that keeps returning to the same workspace keeps returning to the same heap.
//!
//! Each round edits a file and puts it back, deletes a file and recreates it, and saves it the way editors do (write a temporary copy, rename it over the original). After every round the workspace is byte-for-byte what it was, so any heap a round leaves behind is a leak: a registry entry for a forgotten path, a report generation nobody reads, a cache keyed on content that is gone.
//!
//! The counting global allocator (`peak_alloc`) measures live heap bytes exactly, with no RSS sampling and no sleeps. It is its own test target because a global allocator counts the whole process.

#![cfg(feature = "live")]

use std::{fs, path::Path, sync::Arc};

use anyhow::{anyhow, Context, Result};
use deslop_core::{embedding::test_support::StubProvider, live::AnalysisSession};
use peak_alloc::PeakAlloc;

/// Counts every live heap byte.
#[global_allocator]
static ALLOCATOR: PeakAlloc = PeakAlloc;

/// Files that stay in the workspace; they duplicate one another, so the report holds a cluster.
const RESIDENT_FILES: [&str; 3] = ["Ledger.cs", "Journal.cs", "Statement.cs"];
/// The file edited and restored every round.
const EDITED_FILE: &str = "Ledger.cs";
/// The file deleted, recreated and rename-saved every round.
const CHURNED_FILE: &str = "Register.cs";
/// The temporary copy an editor writes before renaming it over the original.
const TEMPORARY_COPY: &str = "Register.cs.tmp~";
/// Rounds run before measuring, so caches and history reach their steady size.
const WARM_ROUNDS: u32 = 8;
/// Rounds run between the two measurements; a per-round leak grows with this.
const MEASURED_ROUNDS: u32 = 24;
/// Heap the measured rounds may add in total. Far below one round's worth of retained report, source text and registry entry.
const MAX_GROWTH_BYTES: usize = 4096;
/// Smallest clone the session reports.
const MIN_NODES: u32 = 15;
/// The body every resident file starts with.
const SETTLED: &str = "total = total + amount * 2;";
/// The body the edited file holds for half a round.
const EDITED: &str = "total = total - amount * 3;";

/// A class named after its file whose loop holds `body`.
fn class_source(stem: &str, body: &str) -> String {
    format!(
        "namespace Books\n{{\n    public class {stem}\n    {{\n        public int Settle(int[] amounts)\n        {{\n            var total = 0;\n            foreach (var amount in amounts)\n            {{\n                {body}\n                if (total > 1000)\n                {{\n                    total = total / 2;\n                }}\n            }}\n            return total;\n        }}\n    }}\n}}\n"
    )
}

/// Writes `name` holding `body`.
fn write(root: &Path, name: &str, body: &str) -> Result<()> {
    let stem = name.trim_end_matches(".cs");
    fs::write(root.join(name), class_source(stem, body)).with_context(|| format!("write {name}"))
}

/// Feeds `names` through one pass, as a watcher flush would.
fn apply(session: &mut AnalysisSession, root: &Path, names: &[&str]) -> Result<()> {
    let paths: Vec<_> = names.iter().map(|name| root.join(name)).collect();
    session.apply_changes(&paths).map(drop).context("apply changes")
}

/// One round that leaves the workspace exactly as it found it.
fn round(session: &mut AnalysisSession, root: &Path) -> Result<()> {
    write(root, EDITED_FILE, EDITED)?;
    apply(session, root, &[EDITED_FILE])?;
    write(root, EDITED_FILE, SETTLED)?;
    apply(session, root, &[EDITED_FILE])?;
    fs::remove_file(root.join(CHURNED_FILE)).context("delete")?;
    apply(session, root, &[CHURNED_FILE])?;
    write(root, CHURNED_FILE, SETTLED)?;
    apply(session, root, &[CHURNED_FILE])?;
    fs::write(root.join(TEMPORARY_COPY), class_source("Register", SETTLED)).context("temp copy")?;
    fs::rename(root.join(TEMPORARY_COPY), root.join(CHURNED_FILE)).context("rename over")?;
    apply(session, root, &[TEMPORARY_COPY, CHURNED_FILE])
}

/// Runs `count` rounds.
fn rounds(session: &mut AnalysisSession, root: &Path, count: u32) -> Result<()> {
    (0..count).try_for_each(|_| round(session, root))
}

/// Files analysed, file handles held and clusters reported.
fn shape(session: &AnalysisSession) -> Result<(usize, usize, usize)> {
    let pipeline = session.pipeline().ok_or_else(|| anyhow!("no installed pipeline"))?;
    let report = session.report();
    Ok((report.files_analysed, pipeline.registry().len(), report.clusters.len()))
}

#[test]
fn edit_delete_and_rename_save_rounds_return_the_heap_to_where_it_was() -> Result<()> {
    let workspace = tempfile::tempdir().context("tempdir")?;
    let root = workspace.path();
    RESIDENT_FILES
        .iter()
        .chain([&CHURNED_FILE])
        .try_for_each(|name| write(root, name, SETTLED))?;
    let provider = Arc::new(StubProvider::new());
    let mut session = AnalysisSession::new(root.to_path_buf(), MIN_NODES, false, None, provider)
        .context("session")?;
    let all_files = RESIDENT_FILES.len() + 1;
    let cold = shape(&session)?;
    assert_eq!(cold.0, all_files, "cold: every file is analysed");
    assert_eq!(cold.1, all_files, "cold: one handle per file");
    assert!(cold.2 >= 1, "cold: the identical classes form a cluster, or the rounds analyse nothing");

    rounds(&mut session, root, WARM_ROUNDS)?;
    assert_eq!(shape(&session)?, cold, "warm: the same workspace gives the same report");
    let warm_heap = ALLOCATOR.current_usage();

    rounds(&mut session, root, MEASURED_ROUNDS)?;
    assert_eq!(shape(&session)?, cold, "measured: the same workspace gives the same report");
    let grown = ALLOCATOR.current_usage().saturating_sub(warm_heap);
    assert!(
        grown <= MAX_GROWTH_BYTES,
        "{MEASURED_ROUNDS} rounds that each restore the workspace left {grown} more heap bytes held (from {warm_heap}): past the {MAX_GROWTH_BYTES}-byte bound, so a round leaks"
    );
    Ok(())
}
