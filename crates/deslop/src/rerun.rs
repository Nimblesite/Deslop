//! Incremental rerun argument helpers for the `deslop` CLI.

use std::path::PathBuf;

use anyhow::Result;

use crate::Cli;

/// Parsed `--rerun-add` entry.
#[derive(Debug)]
pub(crate) struct RerunAdd {
    /// Source path copied from.
    pub(crate) src: PathBuf,
    /// Destination path copied to.
    pub(crate) dst: PathBuf,
}

/// Parses the `SRC=DST` spec.
pub(crate) fn parse_rerun_adds(specs: &[String]) -> Result<Vec<RerunAdd>> {
    specs
        .iter()
        .map(|spec| {
            let (src, dst) = spec
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("--rerun-add spec must be SRC=DST, got {spec:?}"))?;
            Ok(RerunAdd {
                src: PathBuf::from(src),
                dst: PathBuf::from(dst),
            })
        })
        .collect()
}

/// Merges all paths that must be replayed through `update_files`.
pub(crate) fn assemble_touched(args: &Cli, adds: &[RerunAdd]) -> Vec<PathBuf> {
    let capacity = args
        .rerun_touch
        .len()
        .saturating_add(args.rerun_remove.len())
        .saturating_add(adds.len());
    let mut out: Vec<PathBuf> = Vec::with_capacity(capacity);
    out.extend(args.rerun_touch.iter().cloned());
    push_unique_paths(&mut out, &args.rerun_remove);
    push_unique_paths(&mut out, adds.iter().map(|add| &add.dst));
    out
}

/// Appends paths that are not already present.
fn push_unique_paths<'a>(out: &mut Vec<PathBuf>, paths: impl IntoIterator<Item = &'a PathBuf>) {
    for path in paths {
        if !out.contains(path) {
            out.push(path.clone());
        }
    }
}
