//! Library-level error type. Uses `thiserror`; `anyhow` lives in the binary.

use std::{io, path::PathBuf};

use thiserror::Error;

/// Errors produced by `codededup-core`.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Tree-sitter rejected the selected grammar.
    #[error("failed to load tree-sitter grammar for {language}: {source}")]
    GrammarLoad {
        /// Language id for which grammar loading failed.
        language: &'static str,
        /// Upstream error from tree-sitter.
        #[source]
        source: tree_sitter::LanguageError,
    },

    /// Tree-sitter could not parse the source at all (hit a timeout or
    /// cancelled).
    #[error("tree-sitter failed to produce a parse tree for {language}")]
    ParseFailed {
        /// Language id whose parser failed.
        language: &'static str,
    },

    /// I/O failure while reading a source file.
    #[error("failed to read {path}: {source}")]
    Io {
        /// Path whose read failed.
        path: PathBuf,
        /// Upstream I/O error.
        #[source]
        source: io::Error,
    },
}
