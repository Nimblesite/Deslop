//! Pure path/URI geometry helpers for the LSP backend
//! ([LSP-CAPABILITIES]). Extracted from `backend.rs` to keep that file
//! under the 500-line budget; every function here is side-effect-free.

use std::path::{Path, PathBuf};

use tower_lsp::lsp_types::FileEvent;

use crate::backend::url_to_path;

/// Picks the occurrence the caller should jump to from a cursor at
/// `(cursor_path, cursor_byte)`. Prefers the first occurrence that is
/// NOT the one the cursor sits in; falls back to the first occurrence
/// overall when every member lives in the same byte range. Resolves
/// relative occurrence paths against `workspace_root` before comparing.
#[must_use]
pub fn pick_canonical<'a>(
    occurrences: &'a [deslop_core::report::ReportOccurrence],
    workspace_root: &Path,
    cursor_path: &Path,
    cursor_byte: usize,
) -> Option<&'a deslop_core::report::ReportOccurrence> {
    occurrences
        .iter()
        .find(|occurrence| {
            let absolute = absolute_path(workspace_root, &occurrence.path);
            !(absolute == cursor_path
                && occurrence.start_byte <= cursor_byte
                && cursor_byte < occurrence.end_byte)
        })
        .or_else(|| occurrences.first())
}

/// Joins `path` onto `workspace_root` when `path` is relative. Returns
/// `path` unchanged when it is already absolute.
#[must_use]
pub fn absolute_path(workspace_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    }
}

/// Extracts filesystem paths from watched-file events.
#[must_use]
pub fn paths_from_file_events(events: &[FileEvent]) -> Vec<PathBuf> {
    events
        .iter()
        .filter_map(|event| url_to_path(&event.uri))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use deslop_core::report::ReportOccurrence;
    use tower_lsp::lsp_types::{FileChangeType, FileEvent, Url};

    use super::{absolute_path, paths_from_file_events, pick_canonical};

    fn occurrence(path: &str, start_byte: usize, end_byte: usize) -> ReportOccurrence {
        ReportOccurrence {
            path: PathBuf::from(path),
            start_byte,
            end_byte,
            start_line: 0,
            end_line: 0,
            hidden: false,
        }
    }

    #[test]
    fn absolute_path_joins_relative_and_preserves_absolute() {
        let root = Path::new("/repo");
        assert_eq!(
            absolute_path(root, Path::new("src/a.rs")),
            PathBuf::from("/repo/src/a.rs"),
            "a relative occurrence path resolves against the workspace root",
        );
        assert_eq!(
            absolute_path(root, Path::new("/elsewhere/b.rs")),
            PathBuf::from("/elsewhere/b.rs"),
            "an already-absolute path is returned unchanged",
        );
    }

    #[test]
    fn pick_canonical_prefers_an_occurrence_away_from_the_cursor() {
        let root = Path::new("/repo");
        let occurrences = vec![
            occurrence("/repo/A.cs", 0, 10),
            occurrence("/repo/B.cs", 0, 10),
        ];
        let chosen = pick_canonical(&occurrences, root, Path::new("/repo/A.cs"), 5);
        assert_eq!(
            chosen.map(|occ| occ.path.clone()),
            Some(PathBuf::from("/repo/B.cs")),
            "a cursor inside A.cs must jump to the peer occurrence B.cs",
        );
    }

    #[test]
    fn pick_canonical_resolves_relative_occurrence_paths_against_root() {
        let root = Path::new("/repo");
        let occurrences = vec![occurrence("A.cs", 0, 10), occurrence("B.cs", 0, 10)];
        let chosen = pick_canonical(&occurrences, root, Path::new("/repo/A.cs"), 3);
        assert_eq!(
            chosen.map(|occ| occ.path.clone()),
            Some(PathBuf::from("B.cs")),
            "relative A.cs resolves to /repo/A.cs (the cursor), so B.cs is chosen",
        );
    }

    #[test]
    fn pick_canonical_falls_back_to_the_first_when_all_sit_under_the_cursor() {
        let root = Path::new("/repo");
        let occurrences = vec![occurrence("/repo/A.cs", 0, 10)];
        let chosen = pick_canonical(&occurrences, root, Path::new("/repo/A.cs"), 2);
        assert_eq!(
            chosen.map(|occ| occ.path.clone()),
            Some(PathBuf::from("/repo/A.cs")),
            "the sole occurrence is returned even when the cursor sits in it",
        );
    }

    #[test]
    fn pick_canonical_returns_none_without_occurrences() {
        assert!(pick_canonical(&[], Path::new("/repo"), Path::new("/repo/A.cs"), 0).is_none());
    }

    #[test]
    fn paths_from_file_events_keeps_file_uris_and_drops_non_file_uris(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_a = Url::from_file_path("/repo/A.cs").map_err(|()| "A.cs is a file url")?;
        let file_b = Url::from_file_path("/repo/B.cs").map_err(|()| "B.cs is a file url")?;
        let non_file = Url::parse("untitled:Untitled-1")?;
        let events = vec![
            FileEvent::new(file_a, FileChangeType::CHANGED),
            FileEvent::new(non_file, FileChangeType::CREATED),
            FileEvent::new(file_b, FileChangeType::DELETED),
        ];
        assert_eq!(
            paths_from_file_events(&events),
            vec![PathBuf::from("/repo/A.cs"), PathBuf::from("/repo/B.cs")],
            "only file: URIs survive, preserving event order",
        );
        Ok(())
    }
}
