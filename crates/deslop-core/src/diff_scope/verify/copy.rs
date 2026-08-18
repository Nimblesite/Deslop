//! Git copy-section verification and projection
//! ([PIPELINE-DIFF-INGEST], [METRICS-DIFF-SCOPE]).
//!
//! A `copy from` / `copy to` section names a file that did not exist
//! before the change, so every line of the copy target is new content
//! the change introduced. Git's patch format makes the same statement
//! in both of its copy shapes: a metadata-only copy (`similarity
//! index 100%`, no hunks) asserts the target byte-equals the source,
//! and a copy *with* hunks describes the target as the source plus a
//! delta — either way the target's full 1..=line_count range is added.
//! The source file is untouched by the copy and stays out of the
//! scope (`[existing]`). Hunks, when present, are verified against
//! the target like any other hunk but project nothing of their own:
//! the full-range projection subsumes their added lines, so nothing
//! is ever counted twice.

use std::{
    borrow::Cow,
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::{
    diff_scope::{
        parser::{FileCopy, FilePatch},
        DiffScope,
    },
    error::CoreError,
};

use super::{corpus_miss_is_ignorable, resolve_to_scan_root, split_lines, verify_hunk};

/// Verifies one copy section and marks every target line added.
/// Returns `false` when the target is ignorable (outside the scan
/// root, unsupported extension, or excluded-but-present on disk).
///
/// # Errors
///
/// Returns [`CoreError::DiffStale`] when the target or source is
/// missing from the tree, when a metadata-only copy's target does not
/// byte-equal its source, or when a hunk disagrees with the target.
pub(super) fn ingest_copy(
    patch: &FilePatch,
    file_copy: &FileCopy,
    cwd: &Path,
    scan_root: &Path,
    corpus: &BTreeMap<PathBuf, &[u8]>,
    scope: &mut DiffScope,
) -> Result<bool, CoreError> {
    let Some(target) = resolve_to_scan_root(&file_copy.to, cwd, scan_root) else {
        return Ok(false);
    };
    let Some(target_bytes) = corpus.get(&target).copied() else {
        return missing_copy_target(scan_root, &target);
    };
    let source_bytes = copy_source_bytes(&file_copy.from, cwd, scan_root, corpus)?;
    if patch.hunks.is_empty() {
        require_byte_equal(&source_bytes, &target, target_bytes)?;
    }
    let lines = split_lines(target_bytes);
    verify_copy_hunks(patch, &target, &lines)?;
    tracing::debug!(
        target_lines = lines.len(),
        hunks = patch.hunks.len(),
        "copy section ingested"
    );
    scope.insert_lines(target, 1..=line_count(&lines));
    Ok(true)
}

/// Verifies a copy section's hunks against the target's analysed
/// bytes. The collected added lines are discarded: the full-range
/// projection in [`ingest_copy`] already covers them, so the hunks
/// serve purely as a staleness check here.
fn verify_copy_hunks(
    patch: &FilePatch,
    target: &Path,
    lines: &[&[u8]],
) -> Result<(), CoreError> {
    let mut added: Vec<u64> = Vec::new();
    for hunk in &patch.hunks {
        verify_hunk(hunk, target, lines, &mut added)?;
    }
    Ok(())
}

/// A copy target absent from the corpus: ignorable only when the tool
/// could never have analysed it (unsupported extension) or when it
/// exists on disk but was deliberately excluded. Otherwise the tree
/// no longer holds the file the diff created — a stale diff, refused
/// rather than silently dropping a wholesale file copy from the scope.
fn missing_copy_target(scan_root: &Path, target: &Path) -> Result<bool, CoreError> {
    if corpus_miss_is_ignorable(scan_root, target) {
        return Ok(false);
    }
    Err(CoreError::DiffStale {
        path: target.to_path_buf(),
        line: 1,
    })
}

/// Resolves the copy source's bytes: the analysed corpus first, then
/// the file on disk (a source excluded from the corpus is still a
/// real file the copy claim can be verified against). A source that
/// resolves nowhere means the diff describes a tree this one is not.
fn copy_source_bytes<'src>(
    from: &str,
    cwd: &Path,
    scan_root: &Path,
    corpus: &BTreeMap<PathBuf, &'src [u8]>,
) -> Result<Cow<'src, [u8]>, CoreError> {
    let Some(relative) = resolve_to_scan_root(from, cwd, scan_root) else {
        return read_disk_source(&cwd.join(from), PathBuf::from(from));
    };
    if let Some(bytes) = corpus.get(&relative) {
        return Ok(Cow::Borrowed(*bytes));
    }
    let absolute = scan_root.join(&relative);
    read_disk_source(&absolute, relative)
}

/// Reads a copy source from disk, mapping absence to the stale-diff
/// refusal naming `reported` (the path form the user can act on).
fn read_disk_source<'src>(
    absolute: &Path,
    reported: PathBuf,
) -> Result<Cow<'src, [u8]>, CoreError> {
    std::fs::read(absolute)
        .map(Cow::Owned)
        .map_err(|_| CoreError::DiffStale {
            path: reported,
            line: 1,
        })
}

/// Enforces what a metadata-only copy asserts: `similarity index
/// 100%` with no hunks means the target byte-equals the source. A
/// divergence means the tree moved past the diff — refused naming the
/// target and its first divergent line, because tagging the whole
/// target as added on the strength of a claim the tree contradicts
/// would mislabel every line after the divergence.
fn require_byte_equal(
    source: &[u8],
    target_path: &Path,
    target: &[u8],
) -> Result<(), CoreError> {
    if source == target {
        return Ok(());
    }
    Err(CoreError::DiffStale {
        path: target_path.to_path_buf(),
        line: first_divergent_line(source, target),
    })
}

/// 1-indexed first line where the two byte streams disagree; when
/// every shared line matches (a length or trailing-newline
/// difference), the line just past the shared prefix.
fn first_divergent_line(source: &[u8], target: &[u8]) -> u64 {
    let source_lines = split_lines(source);
    let target_lines = split_lines(target);
    let divergent = source_lines
        .iter()
        .zip(target_lines.iter())
        .position(|(source_line, target_line)| source_line != target_line)
        .unwrap_or_else(|| source_lines.len().min(target_lines.len()));
    u64::try_from(divergent).unwrap_or(u64::MAX).saturating_add(1)
}

/// Number of lines as the `u64` the span projection iterates.
fn line_count(lines: &[&[u8]]) -> u64 {
    u64::try_from(lines.len()).unwrap_or(u64::MAX)
}
