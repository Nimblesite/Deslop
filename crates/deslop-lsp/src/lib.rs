//! Language Server Protocol shell for `Deslop` ([LSP-TRANSPORT]).
//!
//! Forwards every request to the live [`deslop_core::live::LiveApi`]
//! implementation via [`backend::LspBackend`]. The library half of the
//! crate is exported so the test harness can drive the backend without
//! spawning the binary, but the production path is always the
//! `deslop-lsp` binary defined in `src/main.rs`.

pub mod app;
pub mod backend;
pub mod code_lens;
pub mod commands;
pub mod custom_methods;
pub mod diagnostics;
pub mod file_watch;
pub mod hover;
pub mod ipc;
pub mod position;
pub mod presentation;
pub mod server;

pub use backend::LspBackend;
pub use server::run_stdio;
