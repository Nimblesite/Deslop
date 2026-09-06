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
    use std::path::PathBuf;

    use tower_lsp::lsp_types::{FileChangeType, FileEvent, Url};

    use super::paths_from_file_events;

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
