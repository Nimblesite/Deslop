//! Process adapter for the `deslop-lsp` application layer.

use std::{env, process::ExitCode};

fn main() -> ExitCode {
    deslop_lsp::app::run_process(
        env::args(),
        std::io::stdout(),
        deslop_lsp::app::run_stdio_process,
    )
}
