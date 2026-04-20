//! Human-facing stderr output: [`preamble`], [`summary`],
//! [`finish_ok`] / [`finish_err`]. Plain English by default;
//! `--technical` switches to the researcher view (signal letters,
//! taxonomy IDs, AST node counts).

mod body;
mod chrome;
mod theme;

use std::{io::IsTerminal as _, path::Path};

pub use body::summary;
pub use chrome::{finish_err, finish_ok, preamble};

/// Global colour policy for the current run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChoice {
    /// Emit ANSI escapes.
    Always,
    /// Never emit ANSI escapes.
    Never,
}

impl ColorChoice {
    /// Resolves the effective colour choice from the user-supplied
    /// `--no-color` flag, the `NO_COLOR` environment variable, the
    /// `CODEDEDUP_FORCE_COLOR` override (tests and CI logs that want
    /// ANSI even without a TTY), and the stderr TTY state, in that
    /// precedence order.
    #[must_use]
    pub fn resolve(force_off: bool) -> Self {
        if force_off {
            return Self::Never;
        }
        if std::env::var_os("NO_COLOR").is_some() {
            return Self::Never;
        }
        if std::env::var_os("CODEDEDUP_FORCE_COLOR").is_some() {
            return Self::Always;
        }
        if std::io::stderr().is_terminal() {
            Self::Always
        } else {
            Self::Never
        }
    }
}

/// CLI knobs surfaced in the preamble.
#[derive(Debug)]
pub struct PreambleKnobs<'a> {
    /// Value of `--min-nodes`.
    pub min_nodes: u32,
    /// String form of `--embeddings`.
    pub embedding_mode: &'a str,
    /// Whether the incremental-cache path is enabled.
    pub incremental: bool,
    /// When true, the preamble + summary surface the researcher-jargon
    /// view (signal letters, AST node counts, taxonomy IDs). When
    /// false (default), output is plain English.
    pub technical: bool,
}

/// Paths written by a successful run, grouped so [`finish_ok`] stays
/// under the 7-argument function budget.
#[derive(Debug)]
pub struct WrittenArtefacts<'a> {
    /// On-disk report paths (JSON / text / HTML, in whatever subset
    /// was enabled).
    pub reports: &'a [std::path::PathBuf],
    /// Path to the log file; `None` when logs went to stderr.
    pub log: Option<&'a Path>,
}
