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

    /// Configuration file was present but could not be parsed as valid
    /// TOML. See [`crate::config`].
    #[error("failed to parse exclusion config {path}: {source}")]
    ConfigParse {
        /// Config path that failed to parse.
        path: PathBuf,
        /// Upstream TOML parse error.
        #[source]
        source: toml::de::Error,
    },

    /// A pattern in the exclusion config was rejected by the
    /// `ignore::gitignore` compiler.
    #[error("invalid glob pattern {pattern:?} in {path}: {source}")]
    ConfigPattern {
        /// Config path that contained the bad pattern.
        path: PathBuf,
        /// The offending pattern string.
        pattern: String,
        /// Upstream error from `ignore::gitignore`.
        #[source]
        source: ignore::Error,
    },

    /// Report JSON supplied via `--from-report` could not be parsed.
    #[error("failed to deserialise report {path}: {source}")]
    ReportDeserialize {
        /// Path of the JSON report that failed to parse.
        path: PathBuf,
        /// Upstream `serde_json` error.
        #[source]
        source: serde_json::Error,
    },
}
