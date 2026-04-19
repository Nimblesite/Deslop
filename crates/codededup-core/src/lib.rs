//! Core analysis library for CodeDedup.
//!
//! Exposes the pipeline used by the `codededup` CLI and (eventually) an LSP
//! front-end. Downstream binaries should depend on this crate rather than
//! reimplementing any pipeline stage.

/// Semantic version of the `codededup-core` library.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
