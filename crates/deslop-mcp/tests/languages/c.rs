//! [LANG-CAND-C] Real LSP/MCP binaries publish native C findings: a sorted
//! insert renamed and retyped into a priority queue reaches an agent as `c`.

#![cfg(unix)]

use crate::common::language_label::assert_language_label_over_mcp;
use anyhow::Result;

const FIXTURE: &str = "c-mcp";
const EXTENSION: &str = "c";
const LANGUAGE: &str = "c";
const CONTRACT: &str = "[LANG-CAND-C]";

#[test]
fn c_clusters_keep_their_language_over_mcp() -> Result<()> {
    assert_language_label_over_mcp(FIXTURE, EXTENSION, LANGUAGE, CONTRACT)
}
