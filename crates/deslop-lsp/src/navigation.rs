//! Pure path/URI geometry helpers for the LSP backend
//! ([LSP-CAPABILITIES]). Extracted from `backend.rs` to keep that file
//! under the 500-line budget; every function here is side-effect-free.

use std::path::PathBuf;

use tower_lsp::lsp_types::FileEvent;

use crate::backend::url_to_path;

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

    use tower_lsp::lsp_types::{FileChangeType, FileEvent, Url};

    use super::paths_from_file_events;

    /// A path in an imagined repository, absolute on this host.
    ///
    /// `Url::from_file_path` refuses a relative path, and `/repo/A.cs` is
    /// relative on Windows: a path with no volume names a location on
    /// whatever drive the caller happens to be on. The crate directory is
    /// absolute everywhere and fixed for a checkout, so the test stays
    /// deterministic without asking the environment anything.
    fn in_repo(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("repo")
            .join(name)
    }

    #[test]
    fn paths_from_file_events_keeps_file_uris_and_drops_non_file_uris(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (path_a, path_b) = (in_repo("A.cs"), in_repo("B.cs"));
        let file_a = Url::from_file_path(&path_a).map_err(|()| "A.cs is a file url")?;
        let file_b = Url::from_file_path(&path_b).map_err(|()| "B.cs is a file url")?;
        let non_file = Url::parse("untitled:Untitled-1")?;
        let events = vec![
            FileEvent::new(file_a, FileChangeType::CHANGED),
            FileEvent::new(non_file, FileChangeType::CREATED),
            FileEvent::new(file_b, FileChangeType::DELETED),
        ];
        assert_eq!(
            paths_from_file_events(&events),
            vec![path_a, path_b],
            "only file: URIs survive, preserving event order",
        );
        Ok(())
    }
}
