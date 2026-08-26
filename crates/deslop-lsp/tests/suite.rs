//! [CI-RELEASE-BUILD] [TEST-ONE-BINARY] The crate's whole integration
//! suite, linked once.
//!
//! Cargo builds one executable per `tests/*.rs`, and each one statically
//! links the entire workspace under the release profile. At 176 files that
//! was 176 whole-program links per CI run — the bulk of a 20-minute compile
//! that cancelled every Rust shard at its cap. Declaring the suites as
//! modules of a single binary links them once instead.
//!
//! Each module below is a former `tests/*.rs`, unchanged apart from its
//! `mod common;` line: the shared helpers are declared here, once, so
//! `crate::common::…` still resolves from every suite.

/// Shared fixture helpers, declared once for every suite below.
mod common;

/// The HTTP mock for the embedding provider, declared once for the whole
/// suite. Twelve files used to `#[path]`-include it; as one binary that is
/// the same file loaded twelve times (`clippy::duplicate_mod`), so it is
/// declared here and reached as `crate::mock_ollama` everywhere.
#[path = "../../deslop/tests/cli/mock_ollama.rs"]
mod mock_ollama;

#[path = "app.rs"]
mod app;
#[path = "cli.rs"]
mod cli;
#[path = "code_action.rs"]
mod code_action;
#[path = "code_action_refusal.rs"]
mod code_action_refusal;
#[path = "cpu_throttle_knob.rs"]
mod cpu_throttle_knob;
#[path = "dependency_reactivity.rs"]
mod dependency_reactivity;
#[path = "editor_non_interference.rs"]
mod editor_non_interference;
#[path = "embedding_failure_progress.rs"]
mod embedding_failure_progress;
#[path = "execute_command.rs"]
mod execute_command;
#[path = "history_determinism.rs"]
mod history_determinism;
#[path = "lifecycle.rs"]
mod lifecycle;
#[path = "lsp_embedding_determinism.rs"]
mod lsp_embedding_determinism;
#[path = "lsp_workspace_scoping.rs"]
mod lsp_workspace_scoping;
#[path = "notifications.rs"]
mod notifications;
#[path = "observability_heartbeat.rs"]
mod observability_heartbeat;
#[path = "ollama_fallback.rs"]
mod ollama_fallback;
#[path = "state_file_and_ipc.rs"]
mod state_file_and_ipc;
#[path = "virtual_document.rs"]
mod virtual_document;
