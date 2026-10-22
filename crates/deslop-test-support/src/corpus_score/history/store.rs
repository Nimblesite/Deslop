//! [CORPUS-HISTORY] Where the run history is kept: one pretty-printed record
//! per run under `corpus/history/runs/`, named after the scorecard the run
//! wrote so the files sort in the order the runs happened, and
//! `corpus/history/HISTORY.md`, rendered from every record each time one is
//! added. The records are what later runs are held to; the document is what a
//! person reads.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use super::RunRecord;
use crate::{corpus_score::render::history_log, read_json};

/// Where each run's record is kept, relative to the repository root.
pub const RUNS_DIR: &str = "corpus/history/runs";
/// The history document rendered from every record.
pub const LOG_PATH: &str = "corpus/history/HISTORY.md";
/// The extension a run record is written with.
const RECORD_EXTENSION: &str = "json";

/// Every run record under `root`, oldest first; none when no run was
/// recorded.
///
/// # Errors
///
/// Returns an error when a record cannot be read: a damaged history must fail
/// the run rather than hold it to nothing.
pub fn load(root: &Path) -> Result<Vec<RunRecord>> {
    let dir = root.join(RUNS_DIR);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    record_paths(&dir)?
        .iter()
        .map(|path| read_json(path).with_context(|| format!("{} is not a run", path.display())))
        .collect()
}

/// Every record file in `dir`, sorted by name — which is by run time.
fn record_paths(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = fs::read_dir(dir)
        .with_context(|| format!("cannot read {}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<PathBuf>>>()?;
    paths.retain(|path| path.extension().is_some_and(|extension| extension == RECORD_EXTENSION));
    paths.sort();
    Ok(paths)
}

/// Writes `record` under `root` as its own file, then renders the history
/// document again from every record.
///
/// # Errors
///
/// Returns an error when the record or the document cannot be written.
pub fn add(root: &Path, record: &RunRecord) -> Result<()> {
    let dir = root.join(RUNS_DIR);
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.{RECORD_EXTENSION}", record.scorecard));
    fs::write(&path, serde_json::to_string_pretty(record)? + "\n")
        .with_context(|| format!("cannot write {}", path.display()))?;
    let log = root.join(LOG_PATH);
    fs::write(&log, history_log(&load(root)?))
        .with_context(|| format!("cannot write {}", log.display()))
}
