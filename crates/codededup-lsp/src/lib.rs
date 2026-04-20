//! Language Server Protocol shell for `CodeDedup` ([LSP-TRANSPORT]).
//!
//! Forwards every request to the live [`codededup_core::live::LiveApi`]
//! implementation via [`backend::LspBackend`]. The library half of the
//! crate is exported so the test harness can drive the backend without
//! spawning the binary, but the production path is always the
//! `codededup-lsp` binary defined in `src/main.rs`.

pub mod backend;
pub mod custom_methods;
pub mod diagnostics;

pub use backend::{run_stdio, LspBackend};
