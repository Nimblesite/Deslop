//! Process adapter for the `deslop-lsp` application layer.

use std::{env, process::ExitCode};

use anyhow::Result;
use deslop_lsp::app::LspStartup;

fn main() -> ExitCode {
    deslop_lsp::app::run_process(env::args(), std::io::stdout(), run_stdio_process)
}

/// Thin process-only adapter that binds the app layer to real stdio.
fn run_stdio_process(startup: LspStartup) -> Result<()> {
    deslop_lsp::app::run_startup_with(startup, deslop_lsp::run_stdio)
}
