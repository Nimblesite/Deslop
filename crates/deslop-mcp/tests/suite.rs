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

#[path = "cli.rs"]
mod cli;
#[path = "dart_generated_fp_over_mcp.rs"]
mod dart_generated_fp_over_mcp;
#[path = "dart_language_label_over_mcp.rs"]
mod dart_language_label_over_mcp;
#[path = "fsharp_language_label_over_mcp.rs"]
mod fsharp_language_label_over_mcp;
#[path = "issue_135_rescan_generation.rs"]
mod issue_135_rescan_generation;
#[path = "issue_136_codex_payload_size.rs"]
mod issue_136_codex_payload_size;
#[path = "issue_137_ipc_freshness.rs"]
mod issue_137_ipc_freshness;
#[path = "issue_148_version_mismatch.rs"]
mod issue_148_version_mismatch;
#[path = "issue_149_ui_mcp_agreement.rs"]
mod issue_149_ui_mcp_agreement;
#[path = "issue_151_socket_path_in_lsp_missing_error.rs"]
mod issue_151_socket_path_in_lsp_missing_error;
#[path = "issue_153_rescan_freshness.rs"]
mod issue_153_rescan_freshness;
#[path = "issue_156_cluster_id_stale_offsets.rs"]
mod issue_156_cluster_id_stale_offsets;
#[path = "issue_157_lsp_not_running_recovery_data.rs"]
mod issue_157_lsp_not_running_recovery_data;
#[path = "issue_255_language_enum_tracks_engine.rs"]
mod issue_255_language_enum_tracks_engine;
#[path = "lsp_integration.rs"]
mod lsp_integration;
#[path = "merge_plan.rs"]
mod merge_plan;
#[path = "orphan_exit.rs"]
mod orphan_exit;
#[path = "state_file_backend.rs"]
mod state_file_backend;
#[path = "wrong_root.rs"]
mod wrong_root;
