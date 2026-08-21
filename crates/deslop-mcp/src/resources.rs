//! MCP `resources/list` + `resources/read` implementation.
//!
//! Two canonical resources per [MCP-RESOURCES]:
//!
//! - `deslop://report` — the current canonical JSON report.
//! - `deslop://schema` — the embedded `schema_doc` markdown block
//!   (same content as the LSP virtual doc).
//!
//! Resources are listed via `resources/list`. Their content refreshes
//! every time `resources/read` is called — the daemon always returns
//! the latest snapshot.

use serde_json::{json, Value};

use crate::{
    backend::McpBackend,
    protocol::{jsonrpc_error, ErrorCode, JsonRpcError},
    tools::backend_to_rpc,
};

/// URI of the live report resource.
pub const REPORT_URI: &str = "deslop://report";
/// URI of the schema documentation resource.
pub const SCHEMA_URI: &str = "deslop://schema";

/// MIME type for the report (canonical JSON).
const REPORT_MIME: &str = "application/json";
/// MIME type for the schema doc (markdown).
const SCHEMA_MIME: &str = "text/markdown";
const RESOURCE_URI_FIELD: &str = "uri";
const MIME_TYPE_FIELD: &str = "mimeType";

/// Renders the `resources/list` response payload per MCP spec.
#[must_use]
pub fn resources_list_payload() -> Value {
    json!({
        "resources": [
            {
                (RESOURCE_URI_FIELD): REPORT_URI,
                "name": "Deslop live report",
                "description": "Current duplication report, canonical JSON. Refreshed on every analysis pass.",
                (MIME_TYPE_FIELD): REPORT_MIME,
            },
            {
                (RESOURCE_URI_FIELD): SCHEMA_URI,
                "name": "Deslop report schema",
                "description": "Markdown describing the report schema — field definitions, signal semantics, clone taxonomy.",
                (MIME_TYPE_FIELD): SCHEMA_MIME,
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
                    (RESOURCE_URI_FIELD): REPORT_URI,
                    (MIME_TYPE_FIELD): REPORT_MIME,
                    "text": text,
                }]
            }))
        }
        SCHEMA_URI => {
            let report = backend.report_get().map_err(backend_to_rpc)?;
            Ok(json!({
                "contents": [{
                    (RESOURCE_URI_FIELD): SCHEMA_URI,
                    (MIME_TYPE_FIELD): SCHEMA_MIME,
                    "text": report.schema_doc.clone(),
                }]
            }))
        }
        other => Err(jsonrpc_error(
            ErrorCode::InvalidParams,
            format!("unknown resource uri {other:?}"),
        )),
    }
}
