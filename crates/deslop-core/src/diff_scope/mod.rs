//! Diff ingestion for diff-scoped reporting ([CLI-ARG-DIFF],
//! [OUTPUT-SCHEMA-DIFF-TAGS], [METRICS-DIFF-SCOPE]).
//!
//! [`parser`] turns unified-diff text into a [`ParsedDiff`];
//! [`verify`] byte-checks it against the scanned corpus and projects
//! it into a [`DiffScope`] — the per-file added-line spans every
//! downstream tag and metric reads; [`tag`] stamps the wire tags onto
//! a rendered cluster list.

mod parser;
mod tag;
mod verify;

use std::{collections::BTreeMap, path::PathBuf};

pub use parser::{
    parse_unified_diff, FileCopy, FilePatch, Hunk, HunkLine, HunkLineKind, ParsedDiff,
};
pub use tag::{apply_only_changed, tag_clusters};
pub use verify::build_diff_scope;

/// An inclusive, 1-indexed span of new-side lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineSpan {
    /// First line of the span.
    pub start: u64,
    /// Last line of the span.
    pub end: u64,
}

/// The verified added-line footprint of an ingested diff: for each
/// scan-root-relative path in the analysed corpus, the merged spans of
/// lines the diff added. Files the diff touched without adding lines
/// carry an empty span list and contribute nothing.
#[derive(Debug, Clone, Default)]
pub struct DiffScope {
    /// Merged, ascending added-line spans keyed by scan-root-relative
    /// path — the same path form report occurrences carry.
    files: BTreeMap<PathBuf, Vec<LineSpan>>,
}

impl DiffScope {
    /// Records one file's added-line numbers, merging adjacent and
    /// overlapping lines into spans. Called once per file by the
    /// verifier; ingesting the same path twice merges the sets.
    pub(crate) fn insert_lines(&mut self, path: PathBuf, lines: impl IntoIterator<Item = u64>) {
        let spans = self.files.entry(path).or_default();
        let mut all: Vec<u64> = spans
            .iter()
            .flat_map(|span| span.start..=span.end)
            .chain(lines)
            .collect();
        all.sort_unstable();
        all.dedup();
        *spans = merge_into_spans(&all);
    }

    /// Total added lines across every file in the scope
    /// ([METRICS-DIFF-SCOPE] `added_loc`).
    #[must_use]
    pub fn added_line_total(&self) -> u64 {
        self.files
            .values()
            .flatten()
            .map(|span| span.end.saturating_sub(span.start).saturating_add(1))
            .sum()
    }

    /// True when 1-indexed `line` in `path` was added by the diff.
    #[must_use]
    pub fn contains(&self, path: &std::path::Path, line: u64) -> bool {
        self.files.get(path).is_some_and(|spans| {
            spans
                .iter()
                .any(|span| (span.start..=span.end).contains(&line))
        })
    }

    /// True when the inclusive line range `[start_line, end_line]` of
    /// `path` overlaps any added span ([OUTPUT-SCHEMA-DIFF-TAGS]
    /// `in_diff`).
    #[must_use]
    pub fn intersects(&self, path: &std::path::Path, start_line: u64, end_line: u64) -> bool {
        self.files.get(path).is_some_and(|spans| {
            spans
                .iter()
                .any(|span| span.start <= end_line && start_line <= span.end)
        })
    }

    /// Number of files carrying at least one added line.
    #[must_use]
    pub fn files_with_added_lines(&self) -> usize {
        self.files
            .values()
            .filter(|spans| !spans.is_empty())
            .count()
    }
}

/// Collapses a sorted, deduplicated line list into inclusive spans.
fn merge_into_spans(sorted_lines: &[u64]) -> Vec<LineSpan> {
    let mut spans: Vec<LineSpan> = Vec::new();
    for &line in sorted_lines {
        match spans.last_mut() {
            Some(last) if last.end.saturating_add(1) == line => last.end = line,
            _ => spans.push(LineSpan {
                start: line,
                end: line,
            }),
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    // [METRICS-DIFF-SCOPE] span mechanics: adjacent lines merge, gaps
    // split, and totals / membership / intersection all agree.
    #[test]
    fn spans_merge_count_and_intersect_consistently() {
        let mut scope = DiffScope::default();
        scope.insert_lines(PathBuf::from("src/a.rs"), [3, 4, 5, 9]);
        scope.insert_lines(PathBuf::from("src/b.rs"), [1]);
        assert_eq!(scope.added_line_total(), 5);
        assert_eq!(scope.files_with_added_lines(), 2);
        assert!(scope.contains(Path::new("src/a.rs"), 4));
        assert!(!scope.contains(Path::new("src/a.rs"), 6));
        assert!(scope.intersects(Path::new("src/a.rs"), 6, 9));
        assert!(!scope.intersects(Path::new("src/a.rs"), 6, 8));
        assert!(!scope.intersects(Path::new("src/c.rs"), 1, 100));
    }

    // [METRICS-DIFF-SCOPE]: re-ingesting a path unions its line sets.
    #[test]
    fn reinserting_a_path_unions_spans() {
        let mut scope = DiffScope::default();
        scope.insert_lines(PathBuf::from("src/a.rs"), [1, 2]);
        scope.insert_lines(PathBuf::from("src/a.rs"), [2, 3]);
        assert_eq!(scope.added_line_total(), 3);
        assert!(scope.intersects(Path::new("src/a.rs"), 3, 3));
    }
}
