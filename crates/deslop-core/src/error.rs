//! Library-level error type. Uses `thiserror`; `anyhow` lives in the binary.

use std::{io, path::PathBuf};

use thiserror::Error;

/// Errors produced by `deslop-core`.
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
        /// Upstream TOML parse error. Boxed: it is 88 bytes, which alone
        /// pushed `CoreError` to clippy's `result_large_err` threshold and
        /// so widened every `Result<_, CoreError>` in the crate.
        #[source]
        source: Box<toml::de::Error>,
    },

    /// `[threshold] max_duplication_percent` in the exclusion config
    /// failed validation (not finite, outside `[0, 100]`, etc.).
    #[error("invalid threshold in {path}: {message}")]
    ConfigThreshold {
        /// Config path that carried the invalid threshold.
        path: PathBuf,
        /// Validator diagnostic message.
        message: String,
    },

    /// A pattern in the exclusion config was rejected by the
    /// `ignore::gitignore` compiler.
    #[error("invalid glob pattern {pattern:?} in {path}: {source}")]
    ConfigPattern {
        /// Config path that contained the bad pattern.
        path: PathBuf,
        /// The offending pattern string.
        pattern: String,
        /// Upstream error from `ignore::gitignore`. Boxed for the same
        /// reason as [`CoreError::ConfigParse`]: 64 bytes inline, beside a
        /// `PathBuf` and a `String`, is the widest variant this enum has.
        #[source]
        source: Box<ignore::Error>,
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

    /// The embedding pass failed in a way that must be surfaced to
    /// the user. Includes `--embeddings=required` probe failures and
    /// individual `embed` calls that the provider rejected.
    #[error("embedding pass failed: {message}")]
    Embedding {
        /// Human-readable description of the upstream failure
        /// (provider message, cache I/O error, etc.).
        message: String,
    },

    /// An explicit pair comparison named an endpoint that is not an exact
    /// fingerprint occurrence in the current analysis generation.
    #[error("unknown pair endpoint {path:?} at bytes {start_byte}..{end_byte}")]
    UnknownPairEndpoint {
        /// Workspace-relative or absolute endpoint path.
        path: PathBuf,
        /// Inclusive byte offset.
        start_byte: usize,
        /// Exclusive byte offset.
        end_byte: usize,
    },

    /// An explicit pair comparison repeated one occurrence instead of
    /// selecting two distinct endpoints.
    #[error("pair comparison requires two distinct endpoints")]
    SamePairEndpoint,

    /// `--debug-ast` was invoked on a file whose extension no
    /// registered [`crate::lang::LanguageParser`] claims.
    #[error("no language parser matches extension for {path}")]
    UnsupportedExtension {
        /// Offending path.
        path: PathBuf,
    },

    /// A `--diff` input is not well-formed unified diff text
    /// ([CLI-ARG-DIFF]). Carries the 1-indexed line of the diff text
    /// (not of any source file) so the user can find the defect.
    #[error("invalid unified diff at line {line}: {message}")]
    DiffParse {
        /// 1-indexed line within the diff text that failed to parse.
        line: usize,
        /// What the parser expected or refused.
        message: String,
    },

    /// A `--diff` input parsed but does not byte-match the scanned
    /// tree ([CLI-ARG-DIFF]): a context or added line disagrees with
    /// the file content at its new-side line number. Tagging against a
    /// stale diff would mislabel every downstream population, so the
    /// run is refused.
    #[error(
        "diff does not match the scanned tree: {path} differs at line {line}; \
         regenerate the diff against the analysed revision"
    )]
    DiffStale {
        /// Scan-root-relative path of the mismatching file.
        path: PathBuf,
        /// 1-indexed new-side line number where the bytes disagree.
        line: u64,
    },

    /// A source file's AST nests deeper than
    /// [`crate::lang::shared::MAX_AST_DEPTH`]. Pathological or
    /// machine-generated nesting (e.g. thousands of nested collection
    /// literals) would overflow the pipeline's recursive tree walks, so
    /// the file is rejected and skipped rather than aborting the whole
    /// run or crashing the long-lived LSP/MCP server. Carries no
    /// path so it is safe to log as a structured field.
    #[error("{language} source nests deeper than the {limit}-level AST depth limit")]
    AstTooDeep {
        /// Language id whose file exceeded the depth limit.
        language: &'static str,
        /// The configured maximum normalised-AST depth.
        limit: usize,
    },
}

#[cfg(test)]
mod tests;
