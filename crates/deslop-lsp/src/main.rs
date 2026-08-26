//! Process adapter for the `deslop-lsp` application layer.
//!
//! [LIVE-BINARY] `deslop-lsp` is the single executable that owns the live
//! `AnalysisSession`, file watcher, and scheduler; the LSP client and the
//! agent-facing MCP both delegate to this process.

use std::{env, process::ExitCode};

use anyhow::Result;
use deslop_lsp::app::LspStartup;

fn main() -> ExitCode {
    deslop_lsp::app::run_process(env::args(), std::io::stdout(), run_stdio_process)
}

/// Thin process-only adapter that binds the app layer to real stdio, and
/// reports the exit code the base protocol fixes for how the session ended
/// ([LSP-LIFECYCLE]).
fn run_stdio_process(startup: LspStartup) -> Result<ExitCode> {
    deslop_lsp::app::run_startup_with(startup, deslop_lsp::run_stdio)
        .map(deslop_lsp::ServeEnd::exit_code)
}
