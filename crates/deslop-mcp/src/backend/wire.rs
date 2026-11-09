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
/// The envelope field holding a reply's result.
const RESULT_FIELD: &str = "result";
/// The envelope field holding a reply's error.
const ERROR_FIELD: &str = "error";
/// The envelope field naming the `deslop-mcp` that ships beside the
/// replying `deslop-lsp`, so a stale registration can be pointed at it.
const BUNDLED_MCP_FIELD: &str = "deslop_mcp";
/// The remedy every refusal carries.
const REINSTALL: &str = "reinstall the Deslop VSIX so deslop-mcp and deslop-lsp ship from one bundle, then re-run the analysis";
/// How the remedy names this binary when its own path cannot be read.
const THIS_BINARY: &str = "this deslop-mcp";
/// Why a reply envelope with no producer stamp is refused.
const UNSTAMPED_ENVELOPE: &str = "the reply carries no tool_version stamp, which every reply from this build carries, so deslop-lsp comes from an older build";

/// Refuses an IPC reply envelope that this build did not produce, before
/// its result or error is read. Every reply envelope stamps its
/// producer's version, so a reply with no stamp comes from an engine
/// older than the stamp, and is refused as surely as a foreign one: a
/// page that happens to decode is still another build's answer.
///
/// # Errors
///
/// [`BackendError::WireMismatch`] when the envelope's `tool_version` is
/// absent or is not this binary's.
pub(super) fn refuse_foreign_envelope(
    method: &str,
    endpoint: &Path,
    envelope: &Value,
) -> Result<(), BackendError> {
    let refusal = match stamped_version(envelope) {
        Some(version) if version == crate::version() => return Ok(()),
        Some(version) => mismatch(
            method,
            endpoint,
            Some(version.clone()),
            format!("its tool_version is {version}, so it was not decoded"),
        ),
        None => mismatch(
            method,
            endpoint,
            envelope.get(RESULT_FIELD).and_then(stamped_version),
            UNSTAMPED_ENVELOPE.to_owned(),
        ),
    };
    Err(with_registration(
        with_error_answer(refusal, envelope),
        envelope,
    ))
}

/// Adds what another build answered when its reply was an error, so a
/// refused `-32601` still says which method the other build lacks.
fn with_error_answer(refusal: BackendError, envelope: &Value) -> BackendError {
    match (refusal, envelope.get(ERROR_FIELD)) {
        (
            BackendError::WireMismatch {
                method,
                mcp_version,
                engine_version,
                endpoint,
                detail,
                remedy,
            },
            Some(error),
        ) => BackendError::WireMismatch {
            method,
            mcp_version,
            engine_version,
            endpoint,
            detail: format!("{detail}; it answered with the error {error}").into_boxed_str(),
            remedy,
        },
        (refusal, _) => refusal,
    }
}

/// [MCP-IPC-WIRE-MISMATCH] Names the registration to change when the
/// reply says which `deslop-mcp` ships beside its `deslop-lsp`: after a
/// VSIX upgrade an agent still launches the old bundle's binary by
/// absolute path, and reinstalling alone never rewrites that path.
fn with_registration(refusal: BackendError, envelope: &Value) -> BackendError {
    let bundled = envelope.get(BUNDLED_MCP_FIELD).and_then(Value::as_str);
    match (refusal, bundled) {
        (
            BackendError::WireMismatch {
                method,
                mcp_version,
                engine_version,
                endpoint,
                detail,
                ..
            },
            Some(bundled),
        ) => BackendError::WireMismatch {
            method,
            mcp_version,
            engine_version,
            endpoint,
            detail,
            remedy: registration_remedy(bundled).into_boxed_str(),
        },
        (refusal, _) => refusal,
    }
}

/// The remedy naming both the bundled `deslop-mcp` and this stale one.
fn registration_remedy(bundled: &str) -> String {
    let this = std::env::current_exe().map_or_else(
        |_| THIS_BINARY.to_owned(),
        |path| format!("{:?}", path.display().to_string()),
    );
    format!("{REINSTALL}; or point your agent's MCP server at {bundled:?}, the deslop-mcp that ships with this deslop-lsp, instead of {this}")
}

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
        method: method.into(),
        mcp_version: crate::version(),
        engine_version: engine_version
            .unwrap_or_else(|| UNSTAMPED_ENGINE_VERSION.to_owned())
            .into_boxed_str(),
        endpoint: endpoint.into(),
        detail: detail.into_boxed_str(),
        remedy: REINSTALL.into(),
    }
}
