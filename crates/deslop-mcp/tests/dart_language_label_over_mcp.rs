//! Real-binary MCP regression for GH #164.
//!
//! Drives the actual `deslop-lsp` + `deslop-mcp` binaries (never a fake
//! server) against the `dart-mcp` fixture and asserts that a hand-written
//! Dart cluster reports `language: "dart"` over the `report-query` wire.
//!
//! The MCP page summary derived each cluster's language from a hand-maintained
//! extension → id map in `deslop-mcp` that omitted `.dart` (a drifted copy of
//! the renderer's mapping). Every Dart cluster therefore surfaced as
//! `language: "unknown"`, breaking the language label and the `report-query`
//! language filter on Dart repos even after the enum gained `dart` (#170/#198).

#![cfg(unix)]

use anyhow::Result;

use crate::common;
use common::language_label::assert_language_label_over_mcp;

#[test]
fn dart_clusters_report_dart_language_over_mcp() -> Result<()> {
    assert_language_label_over_mcp("dart-mcp", "dart", "dart", "issue #164")
}
