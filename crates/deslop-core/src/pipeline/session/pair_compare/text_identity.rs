//! Raw and whitespace-folded identity of an explicit pair.

use std::collections::HashMap;

use crate::report::PairTextIdentity;
use crate::{fingerprint::Fingerprint, state::FileId};

/// Whether two known-language occurrences have exactly the same source bytes.
pub(crate) fn same_source_bytes_and_language(
    left: &Fingerprint,
    right: &Fingerprint,
    sources: &HashMap<FileId, Vec<u8>>,
    languages: &HashMap<FileId, &'static str>,
) -> bool {
    let same_language = languages
        .get(&left.file_id)
        .zip(languages.get(&right.file_id))
        .is_some_and(|(first, second)| first == second);
    let snippet = |found: &Fingerprint| {
        sources
            .get(&found.file_id)?
            .get(found.byte_range.start..found.byte_range.end)
    };
    same_language
        && snippet(left)
            .zip(snippet(right))
            .is_some_and(|(first, second)| first == second)
}

/// Raw identity stays precise for refactoring; classification folds whitespace.
#[derive(Clone, Copy)]
pub(super) struct SourceIdentity {
    /// Whether an edit can preserve the exact source representation.
    pub(super) raw: PairTextIdentity,
    /// [CLONE-BUCKETS-IDENTICAL] Equality after the shared ASCII-whitespace fold.
    pub(super) identical: bool,
}

impl SourceIdentity {
    /// Measures both forms from the same source slices.
    pub(super) fn measure(left: &str, right: &str) -> Self {
        let raw = if left == right {
            PairTextIdentity::ByteIdentical
        } else if same_lines_ignoring_indentation(left, right) {
            PairTextIdentity::IndentationOnly
        } else {
            PairTextIdentity::Different
        };
        let fold = crate::report_render::canonicalise_whitespace;
        Self {
            raw,
            identical: left == right || fold(left.as_bytes()) == fold(right.as_bytes()),
        }
    }
}

/// Whether two snippets hold the same lines once each line's leading
/// whitespace is dropped. Line endings are consumed with the line and a
/// missing final line break adds no line, so neither counts as a
/// difference.
fn same_lines_ignoring_indentation(left: &str, right: &str) -> bool {
    left.lines()
        .map(str::trim_start)
        .eq(right.lines().map(str::trim_start))
}
