//! ANSI theme. Empty strings when colour is disabled so the same
//! `eprintln!` templates work unchanged across both modes.

use super::ColorChoice;

/// ANSI escape strings; all empty when [`ColorChoice::Never`].
#[derive(Debug, Clone, Copy)]
pub(super) struct Theme {
    /// Bold start.
    pub bold: &'static str,
    /// Dim / faint start.
    pub dim: &'static str,
    /// Foreground green.
    pub green: &'static str,
    /// Foreground red.
    pub red: &'static str,
    /// Foreground cyan (used for paths).
    pub cyan: &'static str,
    /// Reset — always emitted after any style change.
    pub reset: &'static str,
}

impl Theme {
    /// Returns the ANSI theme for `choice`. `Never` yields an empty
    /// theme so the same `eprintln!` templates work unchanged.
    pub(super) const fn pick(choice: ColorChoice) -> Self {
        match choice {
            ColorChoice::Always => Self {
                bold: "\x1b[1m",
                dim: "\x1b[2m",
                green: "\x1b[32m",
                red: "\x1b[31m",
                cyan: "\x1b[36m",
                reset: "\x1b[0m",
            },
            ColorChoice::Never => Self {
                bold: "",
                dim: "",
                green: "",
                red: "",
                cyan: "",
                reset: "",
            },
        }
    }
}
