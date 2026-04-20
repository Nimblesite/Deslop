//! Batch pipeline entry point used by the CLI.
//!
//! Thin wrapper over [`super::session::PipelineSession::initialise`] —
//! batch is "incremental starting from an empty cache." Keeping the
//! two paths identical is a [CLAUDE.md DRY] invariant.

use std::path::Path;

use crate::{error::CoreError, render::render_ast_dump, report::Report, state::FileRegistry};

use super::{
    config::PipelineConfig,
    corpus::{default_parsers, read_source},
    session::PipelineSession,
};

/// Runs the full analysis pipeline and returns a rendered report.
///
/// # Errors
///
/// Returns [`CoreError::Io`] when a discovered source file cannot be read,
/// [`CoreError::ConfigParse`] / [`CoreError::ConfigPattern`] when the
/// exclusion config is malformed, and propagates any [`CoreError`] from
/// the language parser.
pub fn run(config: &PipelineConfig<'_>) -> Result<Report, CoreError> {
    let embedding = super::config::EmbeddingSettings {
        mode: config.embedding.mode,
        provider: config.embedding.provider,
    };
    let (_session, report) = PipelineSession::initialise(
        config.root.clone(),
        config.min_nodes,
        config.incremental,
        config.config_path.clone(),
        embedding,
    )?;
    Ok(report)
}

/// Parses `path` with the matching language parser and returns a
/// deterministic text dump of the normalised AST ([PIPELINE-NORMALIZE-AST]).
/// Used by the CLI's `--debug-ast` flag — developer tool, not part
/// of the analysis pipeline.
///
/// # Errors
///
/// - [`CoreError::Io`] if the file cannot be read.
/// - [`CoreError::UnsupportedExtension`] if no registered parser
///   claims the file's extension.
/// - Any parser error forwarded from
///   [`crate::lang::LanguageParser::parse_and_normalize`].
pub fn debug_ast_dump(path: &Path) -> Result<String, CoreError> {
    let parsers = default_parsers();
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_lowercase)
        .unwrap_or_default();
    let Some(parser) = parsers
        .iter()
        .find(|parser| parser.file_extensions().iter().any(|e| *e == extension))
    else {
        return Err(CoreError::UnsupportedExtension {
            path: path.to_path_buf(),
        });
    };
    let source = read_source(path)?;
    let mut registry = FileRegistry::new();
    let file_id = registry.register(path.to_path_buf());
    let tree = parser.parse_and_normalize(&source, file_id)?;
    Ok(render_ast_dump(&tree))
}
