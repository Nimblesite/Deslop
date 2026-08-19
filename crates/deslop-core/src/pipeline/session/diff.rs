//! Diff ingestion glue for [`super::PipelineSession`]
//! ([CLI-ARG-DIFF]).
//!
//! Bridges a parsed unified diff to [`crate::diff_scope::verify`]:
//! projects the session's live corpus into the scan-root-relative
//! `path → bytes` map the verifier byte-checks against, resolving diff
//! paths against the invoker's working directory the same way CI
//! produces them.

use std::{collections::BTreeMap, path::PathBuf};

use crate::{
    diff_scope::{build_diff_scope, ParsedDiff},
    error::CoreError,
    report_render::relative_to_scan_root,
};

use super::PipelineSession;

impl PipelineSession {
    /// Verifies `parsed` against the freshly analysed corpus and
    /// stores the resulting scope for every subsequent render.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::DiffStale`] when a context or added line
    /// disagrees with the analysed bytes, and [`CoreError::Io`] when
    /// the working directory needed to resolve diff paths cannot be
    /// read.
    pub(super) fn attach_diff(&mut self, parsed: &ParsedDiff) -> Result<(), CoreError> {
        let cwd = std::env::current_dir().map_err(|source| CoreError::Io {
            path: PathBuf::from("."),
            source,
        })?;
        let cwd = std::fs::canonicalize(&cwd).unwrap_or(cwd);
        let corpus: BTreeMap<PathBuf, &[u8]> = self
            .live_paths
            .iter()
            .filter_map(|(file_id, absolute)| {
                let bytes = self.sources.get(file_id)?;
                Some((
                    relative_to_scan_root(absolute, &self.root),
                    bytes.as_slice(),
                ))
            })
            .collect();
        self.diff_scope = Some(build_diff_scope(parsed, &cwd, &self.root, &corpus)?);
        Ok(())
    }
}
