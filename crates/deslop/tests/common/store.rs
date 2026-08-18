//! Reading what a run left behind: the on-disk parse store
//! ([PIPELINE-INCREMENTAL] layout) and the run's single tracing log.
//!
//! Kept apart from `common::incremental`, which owns the *accounting* —
//! counters, `cache_stats`, equivalence. These helpers answer the other
//! question: what is actually on disk, and what did the run say about
//! it. Blob-integrity scenarios need both, and the store's rejection
//! diagnostics are only observable here.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context as _};

use super::Result;

/// The parse store's directory inside a scan root ([OUTPUT-DIR]).
pub(crate) fn store_dir(scan_root: &Path) -> PathBuf {
    scan_root.join(".deslop/cache/fingerprints")
}

/// Every `.bin` blob under the scan root's parse store, sorted so
/// scenarios pick blobs deterministically. Errors when the store is
/// empty — a scenario that reads blobs must not silently proceed against
/// none.
pub(crate) fn blob_paths(scan_root: &Path) -> Result<Vec<PathBuf>> {
    let store = store_dir(scan_root);
    let mut found = Vec::new();
    collect_blobs(&store, &mut found)?;
    found.sort();
    anyhow::ensure!(
        !found.is_empty(),
        "no blobs under {} — the cold pass did not fill the store",
        store.display()
    );
    Ok(found)
}

/// Recursively collects `.bin` files under `dir` into `found`.
fn collect_blobs(dir: &Path, found: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_blobs(&path, found)?;
        } else if path.extension().is_some_and(|ext| ext == "bin") {
            found.push(path);
        }
    }
    Ok(())
}

/// Every store blob's bytes, in [`blob_paths`] order — the on-disk state
/// a scenario compares before and after a pass.
pub(crate) fn blob_bytes(scan_root: &Path) -> Result<Vec<Vec<u8>>> {
    Ok(blob_paths(scan_root)?
        .iter()
        .map(fs::read)
        .collect::<std::io::Result<Vec<Vec<u8>>>>()?)
}

/// Asserts the pass's log carries `needle`, and counts its occurrences.
///
/// The store's rejection paths are specified to log at `warn!`
/// ([PIPELINE-INCREMENTAL-INTEGRITY] failure modes), and that log line is
/// the only way an operator learns a blob was refused — counters alone
/// say "miss", which is also what a first-ever run says. Asserting the
/// diagnostic keeps it from being silently dropped.
pub(crate) fn assert_log_mentions(
    out_dir: &Path,
    needle: &str,
    times: usize,
    label: &str,
) -> Result<()> {
    let log = read_single_log(out_dir)?;
    let found = log.lines().filter(|line| line.contains(needle)).count();
    assert_eq!(
        found, times,
        "{label}: expected {times} log line(s) containing {needle:?}, found \
         {found}. The rejection diagnostic is the operator's only signal that \
         a blob was refused rather than simply absent."
    );
    Ok(())
}

/// Reads the single `deslop-<ts>.log` under `<out_dir>/logs/`
/// ([OUTPUT-DIR]) — the ANSI-free default sink the CLI routes tracing
/// events to (`tests/cli/logging.rs` pins that routing).
pub(crate) fn read_single_log(out_dir: &Path) -> Result<String> {
    let logs_dir = out_dir.join("logs");
    let logs: Vec<PathBuf> = fs::read_dir(&logs_dir)
        .with_context(|| format!("no logs directory under {}", out_dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| is_timestamped_log(path))
        .collect();
    match logs.as_slice() {
        [only] => Ok(fs::read_to_string(only)?),
        other => Err(anyhow!(
            "expected exactly one deslop-*.log under {}, found {other:?}",
            logs_dir.display()
        )),
    }
}

/// True for the CLI's `deslop-<unix-seconds>.log` file names.
fn is_timestamped_log(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("deslop-"))
        && path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("log"))
}
