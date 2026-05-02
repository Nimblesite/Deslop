//! Server → client notification helpers ([MCP-NOTIFICATIONS]).
//!
//! Implements the two notifications the MCP server pushes whenever the
//! analysis report changes:
//! - `notifications/resources/updated` — standard MCP; clients that
//!   called `resources/subscribe` on `deslop://report` get re-notified.
//! - `notifications/deslop/reportChanged` — custom; mirrors the LSP
//!   `deslop/reportChanged` notification so any listener can react
//!   without polling tool calls.
//!
//! Both frames are written under a single mutex acquisition so they
//! always arrive back-to-back.  Errors are silently swallowed — the
//! notification path must never crash the analysis or embedding threads.

use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use serde_json::{json, Value};

use crate::protocol::JSONRPC_VERSION;

/// Cloneable handle for pushing JSON-RPC notifications to the client.
///
/// The `Arc<Mutex<…>>` lets the server loop and background worker
/// threads share the same writer while serialising all writes under
/// one lock so frames never interleave on the wire.
pub type NotificationSender = Arc<Mutex<Box<dyn Write + Send>>>;

/// MCP standard resource-updated notification method.
const METHOD_RESOURCES_UPDATED: &str = "notifications/resources/updated";

/// Custom report-changed notification (mirrors LSP `deslop/reportChanged`).
const METHOD_REPORT_CHANGED: &str = "notifications/deslop/reportChanged";

/// URI of the primary MCP resource ([MCP-RESOURCES]).
const DESLOP_REPORT_URI: &str = "deslop://report";

/// Pushes `notifications/resources/updated` and
/// `notifications/deslop/reportChanged` to the connected MCP client.
///
/// Both frames are written under one mutex acquisition so they arrive
/// consecutively without interleaving with responses or other
/// notifications from background threads.
pub(crate) fn push_report_changed(sender: &NotificationSender, generation: u64) {
    let resources_updated = json!({
        "jsonrpc": JSONRPC_VERSION,
        "method": METHOD_RESOURCES_UPDATED,
        "params": { "uri": DESLOP_REPORT_URI }
    });
    let report_changed = json!({
        "jsonrpc": JSONRPC_VERSION,
        "method": METHOD_REPORT_CHANGED,
        "params": { "generation": generation }
    });
    let Ok(mut writer) = sender.lock() else {
        return;
    };
    write_frame(&mut **writer, &resources_updated);
    write_frame(&mut **writer, &report_changed);
}

/// Serialises `value` as a newline-terminated JSON frame and flushes `writer`.
fn write_frame(writer: &mut dyn Write, value: &Value) {
    let Ok(bytes) = serde_json::to_vec(value) else {
        return;
    };
    let _ = writer.write_all(&bytes);
    let _ = writer.write_all(b"\n");
    let _ = writer.flush();
}
