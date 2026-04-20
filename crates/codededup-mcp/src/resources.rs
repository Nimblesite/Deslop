//! MCP `resources/list` + `resources/read` implementation.
//!
//! Two canonical resources per [MCP-RESOURCES]:
//!
//! - `codededup://report` — the current canonical JSON report.
//! - `codededup://schema` — the embedded `schema_doc` markdown block
//!   (same content as the LSP virtual doc).
//!
//! Resources are listed via `resources/list`. Their content refreshes
//! every time `resources/read` is called — the daemon always returns
//! the latest snapshot.

use serde_json::{json, Value};

use crate::{
    backend::McpBackend,
    protocol::{ErrorCode, JsonRpcError},
    tools::backend_to_rpc,
};

/// URI of the live report resource.
pub const REPORT_URI: &str = "codededup://report";
/// URI of the schema documentation resource.
pub const SCHEMA_URI: &str = "codededup://schema";

/// MIME type for the report (canonical JSON).
const REPORT_MIME: &str = "application/json";
/// MIME type for the schema doc (markdown).
const SCHEMA_MIME: &str = "text/markdown";

/// Renders the `resources/list` response payload per MCP spec.
#[must_use]
pub fn resources_list_payload() -> Value {
    json!({
        "resources": [
            {
                "uri": REPORT_URI,
                "name": "CodeDedup live report",
                "description": "Current duplication report, canonical JSON. Refreshed on every analysis pass.",
                "mimeType": REPORT_MIME,
            },
            {
                "uri": SCHEMA_URI,
                "name": "CodeDedup report schema",
                "description": "Markdown describing the report schema — field definitions, signal semantics, clone taxonomy.",
                "mimeType": SCHEMA_MIME,
            }
        ]
    })
}

/// Renders a `resources/read` response for `uri`.
///
/// # Errors
///
/// Returns [`ErrorCode::InvalidParams`] for an unknown URI and
/// whatever [`crate::backend::McpBackend::report_get`] surfaces for
/// backend failures.
pub fn read_resource(backend: &dyn McpBackend, uri: &str) -> Result<Value, JsonRpcError> {
    match uri {
        REPORT_URI => {
            let report = backend.report_get().map_err(backend_to_rpc)?;
            let text = serde_json::to_string_pretty(&*report).unwrap_or_else(|_| "{}".to_owned());
            Ok(json!({
                "contents": [{
                    "uri": REPORT_URI,
                    "mimeType": REPORT_MIME,
                    "text": text,
                }]
            }))
        }
        SCHEMA_URI => {
            let report = backend.report_get().map_err(backend_to_rpc)?;
            Ok(json!({
                "contents": [{
                    "uri": SCHEMA_URI,
                    "mimeType": SCHEMA_MIME,
                    "text": report.schema_doc.clone(),
                }]
            }))
        }
        other => Err(JsonRpcError::new(
            ErrorCode::InvalidParams,
            format!("unknown resource uri {other:?}"),
        )),
    }
}
