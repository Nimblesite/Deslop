//! [UX-NO-COLOR] Whether a binary's stderr should carry ANSI colour.
//!
//! Cross-binary server glue, like [`crate::process`]: the CLI, the LSP and
//! the MCP all write log lines to stderr, and an editor or agent reads that
//! stderr through a pipe into a log file. Colour there splits every
//! `key=value` field with escape codes, so a reader cannot grep for them.
//! One rule, shared so the three binaries cannot drift.

use std::io::IsTerminal as _;

/// Turns colour off whatever else holds ([UX-COLOR-NO-COLOR-ENV]).
const NO_COLOR_ENV: &str = "NO_COLOR";
/// Turns colour on without a terminal, for CI logs that want it
/// ([UX-COLOR-FORCE]).
const FORCE_COLOR_ENV: &str = "DESLOP_FORCE_COLOR";

/// True when stderr should carry ANSI colour: never with `NO_COLOR` set,
/// always with `DESLOP_FORCE_COLOR` set, and otherwise only when stderr is
/// a terminal.
#[must_use]
pub fn stderr_wants_colour() -> bool {
    if std::env::var_os(NO_COLOR_ENV).is_some() {
        return false;
    }
    if std::env::var_os(FORCE_COLOR_ENV).is_some() {
        return true;
    }
    std::io::stderr().is_terminal()
}
