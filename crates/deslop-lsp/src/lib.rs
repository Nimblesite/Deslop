//! Language Server Protocol shell for `Deslop` ([LSP-TRANSPORT]).
//!
//! Forwards every request to the live [`deslop_core::live::LiveApi`]
//! implementation via [`backend::LspBackend`]. The library half of the
//! crate is exported so the test harness can drive the backend without
//! spawning the binary, but the production path is always the
//! `deslop-lsp` binary defined in `src/main.rs`.

pub mod app;
pub mod backend;
mod cache_seed;
pub mod code_action;
pub mod code_lens;
pub mod commands;
pub mod custom_methods;
pub mod diagnostics;
pub mod file_watch;
mod help;
pub mod ipc;
pub mod navigation;
pub mod notifications;
pub mod observability;
mod parent_process;
pub mod position;
pub mod presentation;
mod profiling;
pub mod server;
mod threshold_warning;

pub use backend::LspBackend;
pub use server::{run_stdio, ServeEnd};
