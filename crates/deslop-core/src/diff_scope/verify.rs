//! Byte-verification of a parsed diff against the scanned corpus
//! ([CLI-ARG-DIFF], [PIPELINE-DIFF-INGEST]).
//!
//! Every context and added line the diff claims must byte-match the
//! analysed file content at its new-side line number — otherwise the
//! diff describes a different revision than the one on disk, and
//! tagging with it would mislabel every population. Verified files
//! project their added-line numbers into the [`DiffScope`]; git copy
//! sections project their whole target ([`copy`]). Files outside the
//! scan root, with unsupported extensions, or excluded-but-present on
//! disk are ignored and counted on the `diff ingested` tracing event;
//! a supported in-root target missing from the tree is a stale diff,
//! refused rather than silently zeroing the scope a merge gate reads.

mod copy;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::error::CoreError;

use super::{
    parser::{FilePatch, Hunk, HunkLineKind, ParsedDiff},
    DiffScope,
};

/// Verifies `parsed` against the corpus and builds the [`DiffScope`].
///
/// `cwd` is the directory the diff's paths are relative to (the
/// invoker's working directory, matching how CI produces and applies
/// diffs); `scan_root` is the canonical analysis root; `corpus` maps
/// scan-root-relative paths to the exact source bytes the pipeline
/// analysed.
///
/// # Errors
///
/// Returns [`CoreError::DiffStale`] naming the file and new-side line
/// of the first context or added line that does not byte-match the
/// corpus; of a supported in-root target the tree no longer holds; or
/// of a copy section whose source, target, or byte-equality claim the
/// tree contradicts.
pub fn build_diff_scope(
    parsed: &ParsedDiff,
    cwd: &Path,
    scan_root: &Path,
    corpus: &BTreeMap<PathBuf, &[u8]>,
) -> Result<DiffScope, CoreError> {
    let mut scope = DiffScope::default();
    let mut ignored: usize = 0;
    for patch in &parsed.files {
        if ingest_file(patch, cwd, scan_root, corpus, &mut scope)? {
            continue;
        }
        ignored = ignored.saturating_add(1);
    }
    tracing::info!(
        files_in_scope = scope.files_with_added_lines(),
        files_ignored = ignored,
        added_loc = scope.added_line_total(),
        "diff ingested",
    );
    Ok(scope)
}

/// Verifies one file section and inserts its added lines. Returns
/// `false` when the file is out of scope (deleted, outside the scan
/// root, or ignorably absent from the analysed corpus) and was
/// skipped. Copy sections take the [`copy`] path: their target is
/// wholly new content whatever the hunks say.
fn ingest_file(
    patch: &FilePatch,
    cwd: &Path,
    scan_root: &Path,
    corpus: &BTreeMap<PathBuf, &[u8]>,
    scope: &mut DiffScope,
) -> Result<bool, CoreError> {
    if let Some(file_copy) = patch.copy.as_ref() {
        return copy::ingest_copy(patch, file_copy, cwd, scan_root, corpus, scope);
    }
    let Some(new_path) = patch.new_path.as_deref() else {
        return Ok(false);
    };
    let Some(relative) = resolve_to_scan_root(new_path, cwd, scan_root) else {
        return Ok(false);
    };
    let Some(source) = corpus.get(&relative) else {
        return refuse_or_ignore_missing(patch, scan_root, &relative);
    };
    let lines = split_lines(source);
    let mut added: Vec<u64> = Vec::new();
    for hunk in &patch.hunks {
        verify_hunk(hunk, &relative, &lines, &mut added)?;
    }
    scope.insert_lines(relative, added);
    Ok(true)
}

/// Decides the fate of a diff target absent from the analysed corpus
/// ([PIPELINE-DIFF-INGEST]). Three misses stay ignorable: a path whose
/// extension no registered language parser claims (the tool could
/// never analyse it), a file present on disk that discovery
/// deliberately excluded (gitignore / config exclusion), and a section
/// claiming no new-side lines (nothing verifiable, nothing added).
/// A *supported* in-root file the diff fills that is nowhere on disk
/// is a stale diff: the tree has moved past it, and ignoring it would
/// silently zero the scope a merge gate reads. Pinned by
/// `missing_supported_target_in_root_is_refused_as_stale` and the
/// ignorable-direction tests beside it.
fn refuse_or_ignore_missing(
    patch: &FilePatch,
    scan_root: &Path,
    relative: &Path,
) -> Result<bool, CoreError> {
    if corpus_miss_is_ignorable(scan_root, relative) {
        return Ok(false);
    }
    let Some(line) = first_new_side_line(patch) else {
        return Ok(false);
    };
    Err(CoreError::DiffStale {
        path: relative.to_path_buf(),
        line,
    })
}

/// True when a corpus miss carries no accuracy risk: the extension is
/// outside the language registry, or the file exists on disk and was
/// therefore excluded from the corpus on purpose.
fn corpus_miss_is_ignorable(scan_root: &Path, relative: &Path) -> bool {
    crate::pipeline::language_for_path(relative) == "unknown" || scan_root.join(relative).is_file()
}

/// First new-side line number a patch claims (context or added), or
/// `None` when every hunk only removes.
fn first_new_side_line(patch: &FilePatch) -> Option<u64> {
    patch
        .hunks
        .iter()
        .find(|hunk| {
            hunk.lines
                .iter()
                .any(|line| line.kind != HunkLineKind::Removed)
        })
        .map(|hunk| hunk.new_start)
}

/// Verifies one hunk's context and added lines against `lines` and
/// collects the added new-side line numbers.
fn verify_hunk(
    hunk: &Hunk,
    relative: &Path,
    lines: &[&[u8]],
    added: &mut Vec<u64>,
) -> Result<(), CoreError> {
    let mut new_line = hunk.new_start;
    for body in &hunk.lines {
        if body.kind == HunkLineKind::Removed {
            continue;
        }
        verify_line(relative, lines, new_line, body.content.as_bytes())?;
        if body.kind == HunkLineKind::Added {
            added.push(new_line);
        }
        new_line = new_line.saturating_add(1);
    }
    Ok(())
}

/// Byte-compares one new-side line against the analysed source.
fn verify_line(
    relative: &Path,
    lines: &[&[u8]],
    new_line: u64,
    expected: &[u8],
) -> Result<(), CoreError> {
    let index = usize::try_from(new_line.saturating_sub(1)).unwrap_or(usize::MAX);
    let actual = lines.get(index).copied();
    if actual == Some(expected) {
        return Ok(());
    }
    Err(CoreError::DiffStale {
        path: relative.to_path_buf(),
        line: new_line,
    })
}

/// Resolves a diff path (already `a/`/`b/`-stripped) to its
/// scan-root-relative form, or `None` when it lies outside the root.
/// Canonicalises when the file exists so symlinked roots (macOS
/// `/var` → `/private/var`) compare on one filesystem identity; a
/// path the diff adds outside the corpus need not exist, so the
/// lexical join stands in when canonicalisation fails.
fn resolve_to_scan_root(new_path: &str, cwd: &Path, scan_root: &Path) -> Option<PathBuf> {
    let joined = cwd.join(new_path);
    let absolute = std::fs::canonicalize(&joined).unwrap_or(joined);
    absolute
        .strip_prefix(scan_root)
        .ok()
        .map(crate::paths::wire_path)
}

/// Splits source bytes into lines on `\n`, excluding the terminator.
/// A trailing newline yields no phantom final line.
fn split_lines(source: &[u8]) -> Vec<&[u8]> {
    let mut lines: Vec<&[u8]> = source.split(|byte| *byte == b'\n').collect();
    if lines.last().is_some_and(|last| last.is_empty()) && !source.is_empty() {
        let _trailing = lines.pop();
    }
    lines
}

#[cfg(test)]
mod tests;
