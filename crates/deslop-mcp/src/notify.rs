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
//! When both frames are sent, they arrive back-to-back under one lock.

use std::{
    fmt,
    io::{self, Write},
    sync::{Arc, Mutex},
};

use serde_json::{json, Value};

use crate::{protocol::JSONRPC_VERSION, resources::REPORT_URI};

/// Cloneable handle for pushing JSON-RPC notifications to the client.
///
/// Responses and notifications share a writer and subscription state so
/// a report update cannot overtake the reply that changes the subscription.
#[derive(Clone)]
pub struct NotificationSender(Arc<Mutex<ClientOutput>>);

impl fmt::Debug for NotificationSender {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NotificationSender")
            .finish_non_exhaustive()
    }
}

/// Output and subscription for a single MCP connection.
struct ClientOutput {
    /// Destination for JSON-RPC frames.
    writer: Box<dyn Write + Send>,
    /// Whether this connection requested standard updates for the report.
    report_subscribed: bool,
}

impl NotificationSender {
    /// Starts a connection with its report subscription off.
    pub(crate) fn new(writer: impl Write + Send + 'static) -> Self {
        Self(Arc::new(Mutex::new(ClientOutput {
            writer: Box::new(writer),
            report_subscribed: false,
        })))
    }

    /// Flushes the reply before applying its subscription change under the same lock.
    pub(crate) fn write_response(
        &self,
        bytes: &[u8],
        subscription: Option<bool>,
    ) -> io::Result<()> {
        let mut output = self
            .0
            .lock()
            .map_err(|_| io::Error::other("mcp stdout mutex poisoned"))?;
        output.writer.write_all(bytes)?;
        output.writer.flush()?;
        if let Some(subscribed) = subscription {
            output.report_subscribed = subscribed;
        }
        Ok(())
    }
}

/// MCP standard resource-updated notification method.
const METHOD_RESOURCES_UPDATED: &str = "notifications/resources/updated";

/// Custom report-changed notification (mirrors LSP `deslop/reportChanged`).
const METHOD_REPORT_CHANGED: &str = "notifications/deslop/reportChanged";

/// Pushes the custom report change and, for subscribers, the standard update.
pub(crate) fn push_report_changed(sender: &NotificationSender, generation: u64) {
    let Ok(mut output) = sender.0.lock() else {
        return;
    };
    if output.report_subscribed {
        let resources_updated = json!({
            "jsonrpc": JSONRPC_VERSION,
            "method": METHOD_RESOURCES_UPDATED,
            "params": { "uri": REPORT_URI }
        });
        write_frame(&mut *output.writer, &resources_updated);
    }
    let report_changed = json!({
        "jsonrpc": JSONRPC_VERSION,
        "method": METHOD_REPORT_CHANGED,
        "params": { "generation": generation }
    });
    write_frame(&mut *output.writer, &report_changed);
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
