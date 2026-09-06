//! Real-binary MCP regression for GH #270.
//!
//! Drives the actual `deslop-lsp` + `deslop-mcp` binaries (never a fake
//! server) against the `fsharp-mcp` fixture and asserts that a hand-written
//! F# cluster reports `language: "fsharp"` over the `report-query` wire.
//!
//! On a real F# repo (fantomas) every `report-query` cluster surfaced as
//! `language: "unknown"` — a recurrence of the #164/#170/#198 drift for a
//! language the analyzer actually ran (`session-config` lists `fsharp`). The
//! per-cluster language label must be driven by the same parser registry as
//! `session-config`, so every analyzed language classifies, not just C#/Rust/Py.

#![cfg(unix)]

use anyhow::Result;

mod common;
use common::language_label::assert_language_label_over_mcp;

#[test]
fn fsharp_clusters_report_fsharp_language_over_mcp() -> Result<()> {
    assert_language_label_over_mcp("fsharp-mcp", "fs", "fsharp", "issue #270")
}
