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
pub(super) const MAX_TOOL_RESULT_BYTES: usize = 200 * 1024;

/// Caps `payload`'s serialised size. Returns `payload` unmodified
/// when it fits; otherwise truncates the inner `clusters` array
/// until it does and stamps the result with truncation metadata.
#[must_use]
pub(super) fn cap_tool_result(tool: &str, payload: Value) -> Value {
    let original_bytes = serialised_len(&payload);
    if original_bytes <= MAX_TOOL_RESULT_BYTES {
        return payload;
    }
    truncate_with_log(tool, payload, original_bytes)
}

/// Builds the truncated replacement value and emits the warning log.
fn truncate_with_log(tool: &str, payload: Value, original_bytes: usize) -> Value {
    let truncated = shrink_to_budget(payload);
    let final_bytes = serialised_len(&truncated);
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
fn serialised_len(value: &Value) -> usize {
    serde_json::to_vec(value).map_or(usize::MAX, |bytes| bytes.len())
}

/// Walks the payload and, if it carries a `clusters` array, drops
/// clusters from the tail until the serialised size fits the budget.
/// Falls back to a stub payload when no array can be trimmed.
fn shrink_to_budget(mut payload: Value) -> Value {
    if shrink_clusters_in_place(&mut payload) {
        stamp_truncated(&mut payload);
        return payload;
    }
    fallback_truncated_stub()
}

/// Truncates `payload.clusters` (top level) until the result fits
/// the budget. Returns `true` when the payload now fits, `false` when
/// there is no `clusters` array to shrink — or when dropping every
/// cluster still leaves it over budget, so the caller falls back to the
/// stub instead of returning an oversized result.
fn shrink_clusters_in_place(payload: &mut Value) -> bool {
    if !has_cluster_array(payload) {
        return false;
    }
    while serialised_len(payload) > MAX_TOOL_RESULT_BYTES {
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
    payload
        .as_object()
        .and_then(|object| object.get(CLUSTERS_KEY))
        .is_some_and(Value::is_array)
}

/// Pops one cluster off the tail of `payload.clusters`. Returns
/// `false` when the array is missing or already empty so the caller
/// can exit the shrink loop instead of spinning.
fn pop_one_cluster(payload: &mut Value) -> bool {
    let Some(object) = payload.as_object_mut() else {
        return false;
    };
    let Some(clusters) = object.get_mut(CLUSTERS_KEY).and_then(Value::as_array_mut) else {
        return false;
    };
    clusters.pop().is_some()
}

/// Adds the truncation marker fields to an existing object payload.
fn stamp_truncated(payload: &mut Value) {
    let Some(object) = payload.as_object_mut() else {
        return;
    };
    for (key, value) in truncation_marker_fields() {
        let _ = object.insert(key, value);
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
                "call {PAGINATED_TOOL_NAME} with smaller limit, or paginate via offset/limit"
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
        (NEXT_ACTION_KEY): format!("call {PAGINATED_TOOL_NAME} with smaller limit"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOP_OFFENDERS_TOOL: &str = "top-offenders";
    const TEST_CLUSTER_ID: &str = "id";

    /// Small payload under the cap passes through unmodified.
    #[test]
    fn cap_tool_result_passthrough_small_payload() {
        let payload = json!({(CLUSTERS_KEY): [{(TEST_CLUSTER_ID): "abc"}], "total": 1});
        let result = cap_tool_result(TOP_OFFENDERS_TOOL, payload.clone());
        assert_eq!(result, payload, "small payload must pass through");
    }

    /// Oversized payload with `clusters` array gets trimmed and stamped.
    #[test]
    fn cap_tool_result_trims_oversized_cluster_array() {
        let big_string = "x".repeat(1024);
        let cluster = json!({(TEST_CLUSTER_ID): "cluster", "blob": big_string});
        let clusters: Vec<Value> = (0..300).map(|_| cluster.clone()).collect();
        let payload = json!({(CLUSTERS_KEY): clusters});
        let result = cap_tool_result(TOP_OFFENDERS_TOOL, payload);
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
    }

    /// Payload without a `clusters` array falls back to the stub
    /// pointer instead of escaping the cap.
    #[test]
    fn cap_tool_result_falls_back_when_no_cluster_array() {
        let blob = "y".repeat(MAX_TOOL_RESULT_BYTES + 1024);
        let payload = json!({"text": blob});
        let result = cap_tool_result("schema-doc", payload);
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
