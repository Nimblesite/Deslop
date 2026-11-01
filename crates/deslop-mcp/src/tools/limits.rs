//! Defensive size cap for `tools/call` responses.
//!
//! Background ([](https://github.com/Nimblesite/Deslop/issues/136)):
//! Codex's `rmcp_client` crashes its `app-server` when fed a
//! multi-hundred-KB tool result. The exact ceiling isn't documented,
//! but most JSON-RPC clients tolerate well under 512 KB per frame.
//! We pick **200 KB** as a conservative shared budget — well under
//! that ceiling, well above any sane single-cluster payload, and
//! roomy enough that small top-offender pages and slim report pages
//! pass through untouched.
//!
//! When a payload exceeds the cap the inner cluster array is
//! truncated and the payload is augmented with:
//!
//! - `"truncated": true`
//! - `"truncated_reason"`: human-readable explanation
//! - `"truncated_at_bytes"`: the byte threshold that triggered the cap
//! - `"next_action"`: pointer to the paginated `duplicates` tool
//!
//! Heavy structured logging (`tracing::warn!`) accompanies every
//! truncation so operators can size their corpora — payloads that
//! routinely trip the cap signal that an agent should be paginating
//! via `duplicates` instead of asking for everything at once.

use serde_json::{json, Value};
use tracing::warn;

use crate::protocol::{
    jsonrpc_error, jsonrpc_response_ok, serialise_frame, ErrorCode, JsonRpcError, RequestId,
    MAX_RESPONSE_FRAME_BYTES,
};

#[cfg(test)]
#[path = "limits/wire_budget.rs"]
mod wire_budget_tests;

/// Payload field holding the cluster array a wire cap trims.
const CLUSTERS_KEY: &str = "clusters";
/// Payload field flagging that the wire cap removed content.
const TRUNCATED_KEY: &str = "truncated";
/// Payload field explaining why content was removed.
const TRUNCATED_REASON_KEY: &str = "truncated_reason";
/// Payload field recording the byte budget that was hit.
const TRUNCATED_AT_BYTES_KEY: &str = "truncated_at_bytes";
/// Payload field telling the caller how to retrieve the remainder.
const NEXT_ACTION_KEY: &str = "next_action";
#[cfg(test)]
/// Reason fragment identifying the MCP wire cap as the trimmer.
const WIRE_CAP_REASON_FRAGMENT: &str = "MCP wire cap";
/// Name of the paginated tool a truncated payload points callers
/// at. The normative cutover ([MCP-TOOLS]) deleted `report-get`; a
/// marker pointing there would strand the agent on a tool that no
/// longer exists.
const PAGINATED_TOOL_NAME: &str = "duplicates";
/// The retired tool name the marker must never point at again.
#[cfg(test)]
const RETIRED_REPORT_GET_TOOL_NAME: &str = "report-get";

/// Maximum size, in serialised JSON bytes, of any single MCP
/// `tools/call` result envelope ([MCP-RESULT-SIZE-CAP]).
///
/// Picked conservatively: most JSON-RPC clients tolerate <512 KB per
/// frame, Codex's `rmcp_client` is known to choke earlier. At 200 KB
/// a top-offenders `n=5` / `max_occurrences=15` page lands well under
/// the cap on every fixture we ship.
pub(super) const MAX_TOOL_RESULT_BYTES: usize = MAX_RESPONSE_FRAME_BYTES;
/// Explanation shared by the transport guard and standalone dispatch error.
pub(crate) const REQUEST_ID_BUDGET_MESSAGE: &str =
    "request identifier exceeds MCP response byte budget";

/// Reject identifiers that cannot leave room for the bounded fallback result.
pub(crate) fn request_id_fits_response_budget(request_id: &RequestId) -> bool {
    wire_serialised_len(&fallback_truncated_stub(), request_id) <= MAX_TOOL_RESULT_BYTES
}

#[cfg(test)]
pub(super) fn cap_tool_result(tool: &str, payload: Value) -> anyhow::Result<Value> {
    const TEST_REQUEST_ID: RequestId = RequestId::Number(0);
    cap_tool_result_for_request(tool, payload, &TEST_REQUEST_ID)
        .map_err(|error| anyhow::anyhow!("{error:?}"))
}

/// [MCP-RESULT-SIZE-CAP] Cap the complete frame using its actual request identifier.
pub(super) fn cap_tool_result_for_request(
    tool: &str,
    payload: Value,
    request_id: &RequestId,
) -> Result<Value, JsonRpcError> {
    let original_bytes = wire_serialised_len(&payload, request_id);
    if original_bytes <= MAX_TOOL_RESULT_BYTES {
        return Ok(payload);
    }
    let capped = truncate_with_log(tool, payload, original_bytes, request_id);
    if wire_serialised_len(&capped, request_id) > MAX_TOOL_RESULT_BYTES {
        return Err(jsonrpc_error(
            ErrorCode::InvalidParams,
            REQUEST_ID_BUDGET_MESSAGE,
        ));
    }
    Ok(capped)
}

/// Builds the truncated replacement value and emits the warning log.
fn truncate_with_log(
    tool: &str,
    payload: Value,
    original_bytes: usize,
    request_id: &RequestId,
) -> Value {
    let truncated = shrink_to_budget(payload, request_id);
    let final_bytes = wire_serialised_len(&truncated, request_id);
    warn!(
        tool = tool,
        original_bytes = original_bytes,
        truncated_to = final_bytes,
        cap_bytes = MAX_TOOL_RESULT_BYTES,
        "mcp_tool_result_truncated"
    );
    truncated
}

/// Computes the serialised byte length of `value`. Falls back to a
/// large sentinel on serde failure so the cap path engages rather
/// than letting an unserialisable payload escape unchecked.
#[cfg(test)]
fn serialised_len(value: &Value) -> usize {
    serde_json::to_vec(value).map_or(usize::MAX, |bytes| bytes.len())
}

/// Measure the same generated JSON-RPC envelope and newline written by the server.
fn wire_serialised_len(payload: &Value, request_id: &RequestId) -> usize {
    let result = crate::tools::wrap_tool_result(payload);
    let response = jsonrpc_response_ok(request_id.clone(), result);
    serialise_frame(&response).map_or(usize::MAX, |bytes| bytes.len())
}

/// Walks the payload and, if it carries a `clusters` array, drops
/// clusters from the tail until the serialised size fits the budget.
/// Falls back to a stub payload when no array can be trimmed.
fn shrink_to_budget(mut payload: Value, request_id: &RequestId) -> Value {
    stamp_truncated(&mut payload);
    if shrink_clusters_in_place(&mut payload, request_id) {
        return payload;
    }
    fallback_truncated_stub()
}

/// Truncates `payload.clusters` (top level) until the result fits
/// the budget. Returns `true` when the payload now fits, `false` when
/// there is no `clusters` array to shrink — or when dropping every
/// cluster still leaves it over budget, so the caller falls back to the
/// stub instead of returning an oversized result.
fn shrink_clusters_in_place(payload: &mut Value, request_id: &RequestId) -> bool {
    if !has_cluster_array(payload) {
        return false;
    }
    while wire_serialised_len(payload, request_id) > MAX_TOOL_RESULT_BYTES {
        // The cluster array is empty and the payload still breaches the
        // cap: the non-cluster content alone is oversized. Reporting
        // success here would stamp `truncated: true` on a payload that
        // still blows the agent's wire budget.
        if !pop_one_cluster(payload) {
            return false;
        }
    }
    true
}

/// Probes whether `payload` is a JSON object carrying a `clusters` array.
fn has_cluster_array(payload: &Value) -> bool {
    direct_clusters(payload) || payload.get("page").is_some_and(direct_clusters)
}

/// A report page can be top-level or inside a rescan envelope.
fn direct_clusters(payload: &Value) -> bool {
    payload.get(CLUSTERS_KEY).is_some_and(Value::is_array)
}

/// Pops one cluster off the tail of `payload.clusters`. Returns
/// `false` when the array is missing or already empty so the caller
/// can exit the shrink loop instead of spinning.
fn pop_one_cluster(payload: &mut Value) -> bool {
    let container = if direct_clusters(payload) {
        Some(payload)
    } else {
        payload.get_mut("page")
    };
    let Some(container) = container else {
        return false;
    };
    let Some(clusters) = container
        .get_mut(CLUSTERS_KEY)
        .and_then(Value::as_array_mut)
    else {
        return false;
    };
    let removed = clusters.pop().is_some();
    let returned = clusters.len();
    reconcile_cursor(container, returned);
    removed
}

/// Byte trimming must never leave the occurrence-budget cursor claiming dropped rows.
fn reconcile_cursor(container: &mut Value, returned: usize) {
    let page = container.get("page").cloned().and_then(|value| {
        serde_json::from_value::<deslop_core::wire_generated::ReportPageInfo>(value).ok()
    });
    let total = container
        .get("total_clusters")
        .and_then(Value::as_u64)
        .and_then(|count| usize::try_from(count).ok());
    if let (Some(page), Some(total), Some(value)) = (page, total, container.get_mut("page")) {
        *value = json!(crate::page::page_info(
            page.offset,
            page.limit,
            returned,
            total
        ));
    }
}

/// Adds the truncation marker fields to an existing object payload.
fn stamp_truncated(payload: &mut Value) {
    let Some(object) = payload.as_object_mut() else {
        return;
    };
    for (key, value) in truncation_marker_fields() {
        let _ = object.insert(key, value);
    }
    if let Some(page) = object.get_mut("page").filter(|page| direct_clusters(page)) {
        stamp_truncated(page);
    }
}

/// Returns the four `(key, value)` pairs that mark a payload as
/// truncated. Centralised so the in-place [`stamp_truncated`] path
/// and the [`fallback_truncated_stub`] stay in sync.
fn truncation_marker_fields() -> [(String, Value); 4] {
    [
        (TRUNCATED_KEY.to_owned(), Value::Bool(true)),
        (
            TRUNCATED_REASON_KEY.to_owned(),
            Value::String(format!(
                "result exceeded {MAX_TOOL_RESULT_BYTES} byte MCP wire cap; clusters trimmed"
            )),
        ),
        (
            TRUNCATED_AT_BYTES_KEY.to_owned(),
            Value::from(MAX_TOOL_RESULT_BYTES),
        ),
        (
            NEXT_ACTION_KEY.to_owned(),
            Value::String(format!(
                "call {PAGINATED_TOOL_NAME} with detail: summary, include_per_file: false, and a smaller limit; follow page.next_offset when it advances"
            )),
        ),
    ]
}

/// Replacement payload for results that can't be trimmed (no
/// `clusters` array). Keeps the response useful by pointing the
/// agent at the paginated path.
fn fallback_truncated_stub() -> Value {
    json!({
        (TRUNCATED_KEY): true,
        (TRUNCATED_REASON_KEY): format!(
            "result exceeded {MAX_TOOL_RESULT_BYTES} byte MCP wire cap"
        ),
        (TRUNCATED_AT_BYTES_KEY): MAX_TOOL_RESULT_BYTES,
        (NEXT_ACTION_KEY): format!("call {PAGINATED_TOOL_NAME} with detail: summary, include_per_file: false, and a smaller limit"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOP_OFFENDERS_TOOL: &str = "top-offenders";
    const TEST_CLUSTER_ID: &str = "id";
    const WIRE_CLUSTER_KIND: &str = "identical";
    const WIRE_CLUSTER_SEVERITY: &str = "warning";
    const FIRST_RANK: usize = 1;

    /// [MCP-RESULT-SIZE-CAP] Budget the serialized JSON-RPC frame, including escaped text.
    #[test]
    fn complete_tool_response_fits_the_wire_budget() -> anyhow::Result<()> {
        const REQUEST_ID: u64 = 1;
        let payload = near_budget_cluster_page()?;
        let inner_bytes = serialised_len(&payload);
        assert!(inner_bytes < MAX_TOOL_RESULT_BYTES);
        let capped = cap_tool_result(PAGINATED_TOOL_NAME, payload)?;
        let wrapped = crate::tools::wrap_tool_result(&capped);
        let id = serde_json::from_value(json!(REQUEST_ID))?;
        let response = crate::protocol::jsonrpc_response_ok(id, wrapped);
        let wire_bytes = serde_json::to_vec(&response)?.len();
        assert!(wire_bytes <= MAX_TOOL_RESULT_BYTES,
            "complete MCP frame is {wire_bytes} bytes, exceeding {MAX_TOOL_RESULT_BYTES}; inner payload was {inner_bytes}");
        assert_eq!(capped.get(TRUNCATED_KEY), Some(&json!(true)));
        assert!(capped
            .get(CLUSTERS_KEY)
            .and_then(Value::as_array)
            .is_some_and(|rows| !rows.is_empty()));
        Ok(())
    }

    pub(super) fn near_budget_cluster_page() -> serde_json::Result<Value> {
        const CLUSTER_COUNT: usize = 16;
        const COPY_COUNT: usize = 24;
        const NODE_COUNT: usize = 58;
        const OFFSET: usize = 0;
        let clusters = (FIRST_RANK..=CLUSTER_COUNT).map(|rank| {
            serde_json::from_value::<deslop_core::wire_generated::ReportCluster>(json!({
                "id": format!("{rank:016x}"), "rank": rank, "kind": WIRE_CLUSTER_KIND, "severity": WIRE_CLUSTER_SEVERITY,
                "mass": NODE_COUNT, "canonical_node_count": NODE_COUNT, "occurrence_count": COPY_COUNT,
                "occurrences_total": COPY_COUNT, "occurrences_truncated": false,
                "occurrences": (0..COPY_COUNT).map(wire_budget_occurrence).collect::<Vec<_>>()
            }))
        }).collect::<serde_json::Result<Vec<_>>>()?;
        Ok(
            json!({"clusters": clusters, "total_clusters": CLUSTER_COUNT,
            "page": {"offset": OFFSET, "limit": CLUSTER_COUNT, "returned": CLUSTER_COUNT,
                "has_more": false, "next_offset": null}}),
        )
    }

    fn wire_budget_occurrence(index: usize) -> Value {
        const PATH_SEGMENT: &str = "/component";
        const PATH_DEPTH: usize = 32;
        const START_BYTE: usize = 0;
        const END_BYTE: usize = 383;
        const START_LINE: usize = 1;
        const END_LINE: usize = 19;
        let path = format!(
            "/workspace{}/Calculation{index}.cs",
            PATH_SEGMENT.repeat(PATH_DEPTH)
        );
        json!({"path": path, "start_byte": START_BYTE, "end_byte": END_BYTE,
            "start_line": START_LINE, "end_line": END_LINE, "hidden": false})
    }

    /// [MCP-PAGE-CONTINUATION] A rescan's inner page cannot deny outer truncation.
    #[test]
    fn nested_rescan_page_discloses_byte_trimming_in_its_own_metadata() -> anyhow::Result<()> {
        const CLUSTER_COUNT: usize = 300;
        const BLOB_BYTES: usize = 1024;
        const OFFSET: usize = 0;
        let clusters = vec![json!({"blob": "x".repeat(BLOB_BYTES)}); CLUSTER_COUNT];
        let payload = json!({"page": {"clusters": clusters, "total_clusters": CLUSTER_COUNT,
            "truncated": false, "page": {"offset": OFFSET, "limit": CLUSTER_COUNT,
                "returned": CLUSTER_COUNT, "has_more": false, "next_offset": null}}});
        let capped = cap_tool_result("rescan", payload)?;
        assert_eq!(capped.get(TRUNCATED_KEY), Some(&json!(true)));
        assert_eq!(capped.pointer("/page/truncated"), Some(&json!(true)));
        assert_eq!(capped.pointer("/page/page/has_more"), Some(&json!(true)));
        assert!(serialised_len(&capped) <= MAX_TOOL_RESULT_BYTES);
        Ok(())
    }

    /// [MCP-RESULT-SIZE-CAP] Metadata and page cursors are part of the wire budget.
    #[test]
    fn byte_budget_includes_markers_and_updates_the_returned_page() -> anyhow::Result<()> {
        const BLOB_BYTES: usize = 512;
        const CLUSTER_COUNT: usize = 2;
        const OFFSET: usize = 0;
        const HEADROOM: usize = 8;
        let cluster = json!({"blob": "x".repeat(BLOB_BYTES)});
        let mut payload = json!({"clusters": [cluster.clone(), cluster], "padding": "",
            "total_clusters": CLUSTER_COUNT, "page": {"offset": OFFSET, "limit": CLUSTER_COUNT,
                "returned": CLUSTER_COUNT, "has_more": false, "next_offset": null}});
        let padding = MAX_TOOL_RESULT_BYTES
            .saturating_sub(serialised_len(&payload))
            .saturating_add(BLOB_BYTES)
            .saturating_sub(HEADROOM);
        if let Some(value) = payload.get_mut("padding") {
            *value = json!("p".repeat(padding));
        }
        let capped = cap_tool_result(PAGINATED_TOOL_NAME, payload)?;
        assert!(serialised_len(&capped) <= MAX_TOOL_RESULT_BYTES);
        let returned = capped
            .get(CLUSTERS_KEY)
            .and_then(Value::as_array)
            .map(Vec::len);
        assert_eq!(
            capped.pointer("/page/returned").and_then(Value::as_u64),
            returned.and_then(|count| u64::try_from(count).ok())
        );
        assert_eq!(capped.get(TRUNCATED_KEY), Some(&json!(true)));
        Ok(())
    }

    /// Small payload under the cap passes through unmodified.
    #[test]
    fn cap_tool_result_passthrough_small_payload() -> anyhow::Result<()> {
        let payload = json!({(CLUSTERS_KEY): [{(TEST_CLUSTER_ID): "abc"}], "total": 1});
        let result = cap_tool_result(TOP_OFFENDERS_TOOL, payload.clone())?;
        assert_eq!(result, payload, "small payload must pass through");
        Ok(())
    }

    /// Oversized payload with `clusters` array gets trimmed and stamped.
    #[test]
    fn cap_tool_result_trims_oversized_cluster_array() -> anyhow::Result<()> {
        let big_string = "x".repeat(1024);
        let cluster = json!({(TEST_CLUSTER_ID): "cluster", "blob": big_string});
        let clusters: Vec<Value> = (0..300).map(|_| cluster.clone()).collect();
        let payload = json!({(CLUSTERS_KEY): clusters});
        let result = cap_tool_result(TOP_OFFENDERS_TOOL, payload)?;
        let bytes = serialised_len(&result);
        assert!(
            bytes <= MAX_TOOL_RESULT_BYTES,
            "trimmed payload must fit cap: got {bytes} bytes",
        );
        assert!(result.is_object(), "trimmed payload must be a JSON object");
        assert_eq!(
            result.get(TRUNCATED_KEY),
            Some(&Value::Bool(true)),
            "truncated marker must be present",
        );
        let reason = result
            .get(TRUNCATED_REASON_KEY)
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(
            reason.contains(WIRE_CAP_REASON_FRAGMENT),
            "reason must explain the cap: got {reason:?}",
        );
        let next_action = result
            .get(NEXT_ACTION_KEY)
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(
            next_action.contains(PAGINATED_TOOL_NAME),
            "next_action must point at duplicates: got {next_action:?}",
        );
        assert!(
            !next_action.contains(RETIRED_REPORT_GET_TOOL_NAME),
            "next_action must not point at the deleted report-get tool: got {next_action:?}",
        );
        Ok(())
    }

    /// Payload without a `clusters` array falls back to the stub
    /// pointer instead of escaping the cap.
    #[test]
    fn cap_tool_result_falls_back_when_no_cluster_array() -> anyhow::Result<()> {
        let blob = "y".repeat(MAX_TOOL_RESULT_BYTES + 1024);
        let payload = json!({"text": blob});
        let result = cap_tool_result("schema-doc", payload)?;
        assert!(result.is_object(), "stub must be a JSON object");
        assert_eq!(
            result.get(TRUNCATED_KEY),
            Some(&Value::Bool(true)),
            "stub must mark truncated",
        );
        assert!(
            result.get(NEXT_ACTION_KEY).is_some(),
            "stub fallback must include next_action",
        );
        assert!(
            serialised_len(&result) < MAX_TOOL_RESULT_BYTES,
            "fallback stub must be far below the cap",
        );
        Ok(())
    }

    /// `has_cluster_array` only returns true for object payloads
    /// whose top-level `clusters` field is a JSON array.
    #[test]
    fn has_cluster_array_only_matches_object_with_array_clusters() {
        assert!(has_cluster_array(&json!({(CLUSTERS_KEY): []})));
        assert!(has_cluster_array(
            &json!({(CLUSTERS_KEY): [{(TEST_CLUSTER_ID): "a"}]})
        ));
        assert!(!has_cluster_array(&json!({(CLUSTERS_KEY): "string"})));
        assert!(!has_cluster_array(&json!({"other": []})));
        assert!(!has_cluster_array(&json!([1, 2, 3])));
        assert!(!has_cluster_array(&Value::Null));
    }

    /// Per-cluster pop drains the tail and reports completion.
    #[test]
    fn pop_one_cluster_drains_tail_then_returns_false() {
        let mut payload = json!({(CLUSTERS_KEY): [1, 2, 3]});
        assert!(pop_one_cluster(&mut payload));
        assert!(pop_one_cluster(&mut payload));
        assert!(pop_one_cluster(&mut payload));
        assert!(
            !pop_one_cluster(&mut payload),
            "pop must return false on empty",
        );
    }
}
