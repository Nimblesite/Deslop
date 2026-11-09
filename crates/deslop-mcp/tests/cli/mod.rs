//! End-to-end tests for `deslop-mcp`, grouped by the surface each
//! suite exercises.
//!
//! Drives the real binary over stdio with raw JSON-RPC frames —
//! exactly the contract that Claude Code / Cursor / Continue honour
//! in production. No mocking of the MCP framing anywhere; the only
//! non-binary inputs are the fixture source tree and the prepared
//! JSON payloads below.
//!
//! Covers [MCP-TESTING]: initialize + tools/list + every tool call +
//! resources/read + unparseable / unsupported-language / below-min-nodes
//! edge cases + path-traversal rejection + malformed-frame handling.
//! The shared harness lives in [`crate::common`]: [`common::mcp_child`]
//! owns the process plumbing, [`common::mcp_calls`] the request/response
//! helpers, [`common::mcp_pages`] the page-shape assertions and
//! [`common::mcp_wire`] the wire tokens.

mod binary;
mod cluster_by_id;
mod duplicates;
mod duplicates_filters;
mod find_similar;
mod path_safety;
mod protocol;
mod reload;
mod resources;
mod session;
mod tools_list;
mod validation;
