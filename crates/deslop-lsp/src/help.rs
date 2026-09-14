//! `--help` output for the `deslop-lsp` binary ([LSP-CLI-HELP]).
//!
//! `deslop` and `deslop-mcp` get help from clap. `deslop-lsp` reads argv by
//! hand — the workspace root is the first argument that is not a flag, because
//! editor clients inject `--stdio` wherever they like — so nothing was
//! answering `--help`: it fell through to the workspace-root parser, which
//! logged an error and exited 1 (issue #475). The text is built here from the
//! same constants the parser matches on, so a flag cannot change without its
//! help entry changing with it.

use crate::app::{
    IPC_TRANSPORT_FLAG, NICE_FLAG, RANKING_STRUCTURAL_ONLY_FLAG, WORKER_THREADS_FLAG,
};

/// Component id printed by `--version` and named in usage text.
pub(crate) const BINARY_NAME: &str = "deslop-lsp";

/// Long form of the help request.
const HELP_FLAG: &str = "--help";

/// Short form of the help request, as clap spells it for the sibling binaries.
const HELP_SHORT_FLAG: &str = "-h";

/// One-line summary, matching the `about` strings on the sibling binaries.
const ABOUT: &str =
    "Language Server Protocol server exposing Deslop live duplicate analysis to editors.";

/// Width clap reserves for the short-flag column (`-h, `), so long-only
/// options line up underneath it exactly as they do for `deslop` and
/// `deslop-mcp`.
const SHORT_FLAG_COLUMN: usize = 4;

/// Column the option descriptions start in, wide enough for the longest flag.
const DESCRIPTION_COLUMN: usize = 40;

/// Every accepted option: short flag, long flag, value placeholder, and what
/// it does.
const OPTIONS: &[(&str, &str, &str, &str)] = &[
    (
        "",
        WORKER_THREADS_FLAG,
        "<COUNT>",
        "Cap the analysis worker threads [default: Tokio's own sizing]",
    ),
    (
        "",
        NICE_FLAG,
        "<NICE>",
        "Lower the analysis threads' priority, -20..=19 [default: 0]",
    ),
    (
        "",
        IPC_TRANSPORT_FLAG,
        "<unix|tcp>",
        "Transport for the MCP bridge [default: platform]",
    ),
    (
        "",
        RANKING_STRUCTURAL_ONLY_FLAG,
        "<POLICY>",
        "Restrict ranking to structural evidence [default: .deslop.toml]",
    ),
    (
        "",
        "--stdio",
        "",
        "Accepted from editor clients; stdio is the only transport",
    ),
    (
        "",
        "--debug",
        "",
        "Accepted from editor clients; set RUST_LOG for verbosity",
    ),
    ("-h", HELP_FLAG, "", "Print help"),
    ("-V", "--version", "", "Print version"),
];

/// Returns whether argv asks for help.
pub(crate) fn requests_help(args: &[String]) -> bool {
    args.iter()
        .skip(1)
        .any(|arg| matches!(arg.as_str(), HELP_FLAG | HELP_SHORT_FLAG))
}

/// Renders one option row into clap's aligned two-column layout.
fn option_row((short, long, value, description): &(&str, &str, &str, &str)) -> String {
    let prefix = if short.is_empty() {
        String::new()
    } else {
        format!("{short}, ")
    };
    let flags = format!("{prefix:<SHORT_FLAG_COLUMN$}{long}");
    let head = if value.is_empty() {
        flags
    } else {
        format!("{flags} {value}")
    };
    format!("  {head:<DESCRIPTION_COLUMN$}{description}")
}

/// Builds the exact bytes `--help` writes to stdout.
pub(crate) fn help_output() -> String {
    let options = OPTIONS
        .iter()
        .map(option_row)
        .collect::<Vec<String>>()
        .join("\n");
    format!(
        "{ABOUT}\n\nUsage: {BINARY_NAME} [OPTIONS] <WORKSPACE_ROOT>\n\nArguments:\n  \
         <WORKSPACE_ROOT>  Directory to analyse; the first argument that is not a flag\n\n\
         Options:\n{options}\n"
    )
}
