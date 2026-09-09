//! Wire-contract guard for IPC replies ([MCP-IPC-WIRE-MISMATCH]).
//!
//! The LSP serves the MCP its documents over IPC, and the two binaries
//! can come from two Deslop builds — an extension bundle installed
//! weeks ago beside an engine built today. When the wire moved between
//! those builds, `serde` surfaced the drift as a raw field error that
//! named neither binary, neither version, nor what to do. Every IPC
//! decode now passes through [`decode`]: a reply stamped with another
//! producer version is refused before it is decoded, and a reply this
//! binary cannot decode at the same version is refused the same way,
//! because a document this binary cannot read is a wire mismatch
//! whatever the version strings say.

use std::path::Path;

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::BackendError;

#[cfg(test)]
mod tests;

/// The field a whole report stamps with its producer's version.
const TOOL_VERSION_FIELD: &str = "tool_version";
/// The engine version reported when the reply carries no stamp — a
/// per-file or per-range page is not a whole report.
const UNSTAMPED_ENGINE_VERSION: &str = "unknown (reply carries no tool_version)";

/// Decodes one IPC `result` into `T`, refusing a reply produced by
/// another Deslop version and naming both versions, the endpoint and
/// the remedy whenever the reply cannot be read.
///
/// # Errors
///
/// [`BackendError::WireMismatch`] when the reply's `tool_version` is
/// not this binary's, or when the reply does not decode as `T`.
pub(super) fn decode<T: DeserializeOwned>(
    method: &str,
    endpoint: &Path,
    result: Value,
) -> Result<T, BackendError> {
    let engine_version = stamped_version(&result);
    refuse_foreign_version(method, endpoint, engine_version.as_deref())?;
    serde_json::from_value(result).map_err(|err| {
        mismatch(
            method,
            endpoint,
            engine_version,
            format!("the reply did not decode: {err}"),
        )
    })
}

/// Refuses a reply stamped with a producer version other than this
/// binary's before any field of it is read.
fn refuse_foreign_version(
    method: &str,
    endpoint: &Path,
    engine_version: Option<&str>,
) -> Result<(), BackendError> {
    match engine_version {
        Some(version) if version != crate::version() => Err(mismatch(
            method,
            endpoint,
            Some(version.to_owned()),
            format!("its tool_version is {version}, so it was not decoded"),
        )),
        _ => Ok(()),
    }
}

/// The producer version a whole report stamps on itself.
fn stamped_version(result: &Value) -> Option<String> {
    result
        .get(TOOL_VERSION_FIELD)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// The named condition, with the engine version spelled out even when
/// the reply carried none.
fn mismatch(
    method: &str,
    endpoint: &Path,
    engine_version: Option<String>,
    detail: String,
) -> BackendError {
    BackendError::WireMismatch {
        method: method.to_owned(),
        mcp_version: crate::version().to_owned(),
        engine_version: engine_version.unwrap_or_else(|| UNSTAMPED_ENGINE_VERSION.to_owned()),
        endpoint: endpoint.to_path_buf(),
        detail,
    }
}
