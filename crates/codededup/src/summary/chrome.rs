//! Preamble + finish footers. Contains the "about to do", "done", and
//! "failed" renderers. The body of the summary block (cluster counts,
//! top-N list) lives in [`super::body`].

use std::path::Path;

use crate::logging::LogSink;

use super::{theme::Theme, ColorChoice, PreambleKnobs, WrittenArtefacts};

/// Prints the "about to do" line. Plain English by default; technical
/// knobs (min-nodes, incremental, etc.) are hidden unless `--technical`
/// is set.
pub fn preamble(
    color: ColorChoice,
    scan_path: &Path,
    output_base: &Path,
    log_sink: &LogSink,
    knobs: &PreambleKnobs<'_>,
) {
    let theme = Theme::pick(color);
    eprintln!(
        "{bold}codededup{reset} scanning {cyan}{path}{reset} for duplicated code...",
        bold = theme.bold,
        reset = theme.reset,
        cyan = theme.cyan,
        path = scan_path.display(),
    );
    if knobs.technical {
        eprintln!(
            "  {dim}min-nodes={min_nodes}, embeddings={embeddings}, incremental={incremental}{reset}",
            dim = theme.dim,
            reset = theme.reset,
            min_nodes = knobs.min_nodes,
            embeddings = knobs.embedding_mode,
            incremental = knobs.incremental,
        );
    }
    eprintln!(
        "  {dim}report → {output}.{{json,txt,html}}{reset}",
        dim = theme.dim,
        reset = theme.reset,
        output = output_base.display(),
    );
    match log_sink {
        LogSink::File(path) => eprintln!(
            "  {dim}log    → {path}{reset}",
            dim = theme.dim,
            reset = theme.reset,
            path = path.display(),
        ),
        LogSink::Console => eprintln!(
            "  {dim}log    → stderr (--log-to-console){reset}",
            dim = theme.dim,
            reset = theme.reset,
        ),
    }
}

/// Prints the "wrote these files" footer on a successful run.
pub fn finish_ok(color: ColorChoice, written: &WrittenArtefacts<'_>) {
    let theme = Theme::pick(color);
    eprintln!();
    eprintln!(
        "{green}✔{reset} {bold}done{reset}",
        green = theme.green,
        bold = theme.bold,
        reset = theme.reset,
    );
    for path in written.reports {
        eprintln!(
            "    {dim}report{reset} {path}",
            dim = theme.dim,
            reset = theme.reset,
            path = path.display(),
        );
    }
    if let Some(log_path) = written.log {
        eprintln!(
            "    {dim}log   {reset} {path}",
            dim = theme.dim,
            reset = theme.reset,
            path = log_path.display(),
        );
    }
}

/// Prints a red "run failed" footer with the log location so the
/// operator knows where to look.
pub fn finish_err(color: ColorChoice, log_sink: &LogSink, error: &dyn std::fmt::Display) {
    let theme = Theme::pick(color);
    eprintln!();
    eprintln!(
        "{red}✘{reset} {bold}failed{reset}: {error}",
        red = theme.red,
        bold = theme.bold,
        reset = theme.reset,
    );
    if let LogSink::File(path) = log_sink {
        eprintln!(
            "    {dim}log{reset} {path}",
            dim = theme.dim,
            reset = theme.reset,
            path = path.display(),
        );
    }
}
