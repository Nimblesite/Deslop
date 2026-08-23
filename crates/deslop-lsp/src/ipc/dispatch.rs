//! JSON-RPC method routing for the IPC server ([LSP-IPC]).
//!
//! Transport-agnostic: every function here receives parsed JSON and a
//! [`LiveService`] handle; the surrounding connection handling (Unix
//! socket or TCP loopback, [LIVE-IPC-TCP]) lives in [`super`].

use std::{path::Path, sync::Arc};

use deslop_core::live::{LiveApi, LiveService};
use serde_json::{json, Value};
use tokio::runtime::Handle;

/// JSON-RPC error-object field carrying the numeric code.
const RPC_CODE_KEY: &str = "code";
/// JSON-RPC error-object field carrying the human-readable message.
const RPC_MESSAGE_KEY: &str = "message";
/// JSON-RPC `Method not found` error code.
const METHOD_NOT_FOUND_CODE: i32 = -32_601;
/// JSON-RPC `Invalid params` error code.
const INVALID_PARAMS_CODE: i32 = -32_602;
/// JSON-RPC `Internal error` error code.
const INTERNAL_ERROR_CODE: i32 = -32_603;
/// Request parameter naming the file a report is scoped to.
const PATH_PARAM: &str = "path";
/// Request parameter naming the cluster id a request is scoped to.
const ID_PARAM: &str = "id";

/// Routes a JSON-RPC method to the appropriate [`LiveService`] call.
pub(super) fn dispatch(
    method: &str,
    params: &Value,
    service: &Arc<LiveService>,
    handle: &Handle,
) -> Result<Value, Value> {
    match method {
        "report/get" => dispatch_report_get(service, handle),
        "report/forFile" => dispatch_report_for_file(params, service, handle),
        "report/forRange" => dispatch_report_for_range(params, service, handle),
        "cluster/byId" => dispatch_cluster_by_id(params, service, handle),
        "merge/plan" => dispatch_merge_plan(params, service, handle),
        "session/config" => dispatch_session_config(service, handle),
        "duplicates/findSimilar" => dispatch_find_similar(params, service, handle),
        "embedding/listModels" => dispatch_list_models(service, handle),
        "embedding/setModel" => dispatch_set_model(params, service, handle),
        crate::commands::REFRESH_REPORT => dispatch_refresh_report(service, handle),
        _ => Err(rpc_error(METHOD_NOT_FOUND_CODE, "method not found")),
    }
}

/// Delegates `report/get` to [`LiveApi::report_get`].
fn dispatch_report_get(service: &Arc<LiveService>, handle: &Handle) -> Result<Value, Value> {
    let report = handle.block_on(service.report_get());
    serde_json::to_value(report.as_ref()).map_err(|err| rpc_serialise_error(&err))
}

/// Delegates `report/forFile` to [`LiveApi::report_for_file`].
fn dispatch_report_for_file(
    params: &Value,
    service: &Arc<LiveService>,
    handle: &Handle,
) -> Result<Value, Value> {
    let path = required_str_param(params, PATH_PARAM)?;
    let file_report = handle.block_on(service.report_for_file(Path::new(path)));
    serde_json::to_value(&file_report).map_err(|err| rpc_serialise_error(&err))
}

/// Delegates `report/forRange` to [`LiveApi::report_for_range`].
fn dispatch_report_for_range(
    params: &Value,
    service: &Arc<LiveService>,
    handle: &Handle,
) -> Result<Value, Value> {
    let path = required_str_param(params, PATH_PARAM)?;
    let start_byte = required_u64_param(params, "start_byte")?;
    let end_byte = required_u64_param(params, "end_byte")?;
    let start = usize::try_from(start_byte)
        .map_err(|_| rpc_error(INVALID_PARAMS_CODE, "start_byte overflow"))?;
    let end = usize::try_from(end_byte)
        .map_err(|_| rpc_error(INVALID_PARAMS_CODE, "end_byte overflow"))?;
    let clusters = handle.block_on(service.report_for_range(Path::new(path), start, end));
    serde_json::to_value(&clusters).map_err(|err| rpc_serialise_error(&err))
}

/// Delegates `cluster/byId` to [`LiveApi::cluster_by_id`].
fn dispatch_cluster_by_id(
    params: &Value,
    service: &Arc<LiveService>,
    handle: &Handle,
) -> Result<Value, Value> {
    let id = required_str_param(params, ID_PARAM)?;
    let cluster = handle
        .block_on(service.cluster_by_id(id))
        .map_err(|error| rpc_error(INTERNAL_ERROR_CODE, error.to_string()))?;
    serde_json::to_value(&cluster).map_err(|err| rpc_serialise_error(&err))
}

/// Delegates `merge/plan` to [`LiveApi::merge_plan`]
/// ([AUTOFIX-MERGE-MCP]).
fn dispatch_merge_plan(
    params: &Value,
    service: &Arc<LiveService>,
    handle: &Handle,
) -> Result<Value, Value> {
    let id = required_str_param(params, ID_PARAM)?;
    let plan = handle
        .block_on(service.merge_plan(id))
        .map_err(|error| rpc_error(INTERNAL_ERROR_CODE, error.to_string()))?;
    serde_json::to_value(&plan).map_err(|err| rpc_serialise_error(&err))
}

/// Delegates `session/config` to [`LiveApi::session_config`].
fn dispatch_session_config(service: &Arc<LiveService>, handle: &Handle) -> Result<Value, Value> {
    let config = handle.block_on(service.session_config());
    serde_json::to_value(&config).map_err(|err| rpc_serialise_error(&err))
}

/// Delegates `duplicates/findSimilar` to [`LiveService::find_similar`].
fn dispatch_find_similar(
    params: &Value,
    service: &Arc<LiveService>,
    handle: &Handle,
) -> Result<Value, Value> {
    let request: deslop_core::live::FindSimilarRequest = serde_json::from_value(params.clone())
        .map_err(|error| rpc_error(INVALID_PARAMS_CODE, format!("invalid params: {error}")))?;
    let result = handle
        .block_on(service.find_similar(&request))
        .map_err(|error| rpc_error(INTERNAL_ERROR_CODE, error.to_string()))?;
    serde_json::to_value(&result).map_err(|err| rpc_serialise_error(&err))
}

/// Delegates `embedding/listModels` to [`LiveService::embedding_list_models`].
fn dispatch_list_models(service: &Arc<LiveService>, handle: &Handle) -> Result<Value, Value> {
    let models = handle.block_on(service.embedding_list_models());
    serde_json::to_value(&models).map_err(|err| rpc_serialise_error(&err))
}

/// Delegates `embedding/setModel` to [`LiveService::embedding_set_model`].
///
/// The VSIX reaches this swap through the LSP custom method; MCP clients
/// reach it here. Without this arm the MCP backend can only ever be told
/// "method not found".
fn dispatch_set_model(
    params: &Value,
    service: &Arc<LiveService>,
    handle: &Handle,
) -> Result<Value, Value> {
    let provider_id = required_str_param(params, "provider_id")?;
    let model_id = required_str_param(params, "model_id")?;
    let endpoint = params.get("endpoint").and_then(Value::as_str);
    let provenance = handle
        .block_on(service.embedding_set_model(provider_id, model_id, endpoint))
        .map_err(|error| rpc_error(INTERNAL_ERROR_CODE, error.to_string()))?;
    serde_json::to_value(&provenance).map_err(|err| rpc_serialise_error(&err))
}

/// Forces the same full refresh as `workspace/executeCommand`
/// `deslop.lsp.refreshReport`, but over the MCP-facing IPC endpoint.
fn dispatch_refresh_report(service: &Arc<LiveService>, handle: &Handle) -> Result<Value, Value> {
    handle.block_on(async {
        let session = service.session();
        let (previous_generation, previous_report, delta) = {
            let mut guard = session.lock().await;
            let previous_generation = guard.generation();
            let previous_report = guard.report();
            let delta = guard
                .refresh_full()
                .map_err(|error| rpc_error(INTERNAL_ERROR_CODE, error.to_string()))?;
            (previous_generation, previous_report, delta)
        };
        service
            .remember_snapshot(previous_generation, previous_report)
            .await;
        Ok(json!({
            "command": crate::commands::REFRESH_REPORT,
            "generation": delta.to_generation,
            "clustersAdded": delta.clusters_added.len(),
            "clustersRemoved": delta.clusters_removed.len(),
            "clustersUpdated": delta.clusters_updated.len(),
        }))
    })
}

/// Builds the JSON-RPC `-32602` (invalid params) error for a missing field.
fn missing_param_error(key: &str) -> Value {
    rpc_error(INVALID_PARAMS_CODE, format!("missing {key}"))
}

/// Builds a JSON-RPC error envelope from its named protocol fields.
fn rpc_error(code: i32, message: impl Into<String>) -> Value {
    json!({
        (RPC_CODE_KEY): code,
        (RPC_MESSAGE_KEY): message.into(),
    })
}

/// Reads a required string parameter `key`, or the standard `-32602` error.
fn required_str_param<'a>(params: &'a Value, key: &str) -> Result<&'a str, Value> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| missing_param_error(key))
}

/// Reads a required `u64` parameter `key`, or the standard `-32602` error.
fn required_u64_param(params: &Value, key: &str) -> Result<u64, Value> {
    params
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| missing_param_error(key))
}

/// Maps a serialisation failure to a JSON-RPC internal-error envelope.
fn rpc_serialise_error(err: &serde_json::Error) -> Value {
    rpc_error(INTERNAL_ERROR_CODE, err.to_string())
}
