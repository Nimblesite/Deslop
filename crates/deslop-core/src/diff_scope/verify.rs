//! Byte-verification of a parsed diff against the scanned corpus
//! ([CLI-ARG-DIFF]).
//!
//! Every context and added line the diff claims must byte-match the
//! analysed file content at its new-side line number — otherwise the
//! diff describes a different revision than the one on disk, and
//! tagging with it would mislabel every population. Verified files
//! project their added-line numbers into the [`DiffScope`]; files
//! outside the scan root or absent from the corpus are ignored and
//! counted on the `diff ingested` tracing event.

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
/// corpus.
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
/// root, or not part of the analysed corpus) and was skipped.
fn ingest_file(
    patch: &FilePatch,
    cwd: &Path,
    scan_root: &Path,
    corpus: &BTreeMap<PathBuf, &[u8]>,
    scope: &mut DiffScope,
) -> Result<bool, CoreError> {
    let Some(new_path) = patch.new_path.as_deref() else {
        return Ok(false);
    };
    let Some(relative) = resolve_to_scan_root(new_path, cwd, scan_root) else {
        return Ok(false);
    };
    let Some(source) = corpus.get(&relative) else {
        return Ok(false);
    };
    let lines = split_lines(source);
    let mut added: Vec<u64> = Vec::new();
    for hunk in &patch.hunks {
        verify_hunk(hunk, &relative, &lines, &mut added)?;
    }
    scope.insert_lines(relative, added);
    Ok(true)
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
        .map(Path::to_path_buf)
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
mod tests {
    use super::{super::parse_unified_diff, *};

    fn corpus(entries: &[(&str, &'static [u8])]) -> BTreeMap<PathBuf, &'static [u8]> {
        entries
            .iter()
            .map(|(path, bytes)| (PathBuf::from(path), *bytes))
            .collect()
    }

    // [CLI-ARG-DIFF] verification: matching context + added lines
    // project the added line numbers into the scope.
    #[test]
    fn matching_diff_projects_added_lines() {
        let text = "--- a/src/x.rs\n\
                    +++ b/src/x.rs\n\
                    @@ -1,1 +1,3 @@\n \
                    fn keep() {}\n\
                    +fn one() {}\n\
                    +fn two() {}\n";
        let parsed = parse_unified_diff(text).expect("diff parses");
        let corpus = corpus(&[("src/x.rs", b"fn keep() {}\nfn one() {}\nfn two() {}\n")]);
        let scope = build_diff_scope(&parsed, Path::new("/repo"), Path::new("/repo"), &corpus)
            .expect("clean diff verifies");
        assert_eq!(scope.added_line_total(), 2);
        assert!(scope.contains(Path::new("src/x.rs"), 2));
        assert!(scope.contains(Path::new("src/x.rs"), 3));
        assert!(!scope.contains(Path::new("src/x.rs"), 1));
    }

    // [CLI-ARG-DIFF] verification: a context line that disagrees with
    // the corpus is refused with the file and new-side line.
    #[test]
    fn stale_context_line_names_file_and_line() {
        let text = "--- a/src/x.rs\n\
                    +++ b/src/x.rs\n\
                    @@ -1,1 +1,2 @@\n \
                    fn old_shape() {}\n\
                    +fn added() {}\n";
        let parsed = parse_unified_diff(text).expect("diff parses");
        let corpus = corpus(&[("src/x.rs", b"fn keep() {}\nfn added() {}\n")]);
        let error = build_diff_scope(&parsed, Path::new("/repo"), Path::new("/repo"), &corpus)
            .expect_err("stale context must be refused");
        let CoreError::DiffStale { path, line } = error else {
            panic!("expected DiffStale, got {error:?}");
        };
        assert_eq!(path, PathBuf::from("src/x.rs"));
        assert_eq!(line, 1);
    }

    // [CLI-ARG-DIFF] verification: files absent from the corpus are
    // skipped, never verified, never counted.
    #[test]
    fn out_of_corpus_files_are_ignored() {
        let text = "--- /dev/null\n+++ b/docs/notes.md\n@@ -0,0 +1 @@\n+# Notes\n";
        let parsed = parse_unified_diff(text).expect("diff parses");
        let corpus = corpus(&[("src/x.rs", b"fn keep() {}\n")]);
        let scope = build_diff_scope(&parsed, Path::new("/repo"), Path::new("/repo/src"), &corpus)
            .expect("out-of-root file is skipped, not an error");
        assert_eq!(scope.added_line_total(), 0);
        assert_eq!(scope.files_with_added_lines(), 0);
    }

    // [CLI-ARG-DIFF] verification: CRLF sources verify byte-exactly
    // when the diff payload carries the same `\r`.
    #[test]
    fn crlf_source_verifies_byte_exactly() {
        let text = "--- /dev/null\n+++ b/win.cs\n@@ -0,0 +1 @@\n+var x = 1;\r\n";
        let parsed = parse_unified_diff(text).expect("diff parses");
        let corpus = corpus(&[("win.cs", b"var x = 1;\r\n")]);
        let scope = build_diff_scope(&parsed, Path::new("/repo"), Path::new("/repo"), &corpus)
            .expect("CRLF content matches CRLF source");
        assert_eq!(scope.added_line_total(), 1);
    }
}
