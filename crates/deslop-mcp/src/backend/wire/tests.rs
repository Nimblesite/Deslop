//! Unit pins for [MCP-IPC-WIRE-MISMATCH] at the decode seam: a foreign
//! version is refused before decoding, a same-version drift is named by
//! the field the wire moved, an unstamped reply is named honestly, and a
//! report from this version decodes.

use std::path::PathBuf;

use anyhow::{ensure, Result};
use deslop_core::{
    live::wire::FindSimilarResult, report::ReportCluster, report_fixtures::fixture_report, Report,
};
use serde_json::{json, Value};

use super::decode;

/// A report another Deslop release wrote: stamped with that release's
/// version, carrying `weight` where this wire carries `mass`. It is the
/// input the guard must refuse — nothing in the product decodes it.
const FOREIGN_RELEASE_REPORT: &str =
    include_str!("../../../tests/fixtures/report-from-another-release.json");
/// The version the foreign report is stamped with.
const FOREIGN_RELEASE_VERSION: &str = "0.31.0";
/// The field a whole report stamps its producer version in.
const TOOL_VERSION_FIELD: &str = "tool_version";
/// The whole-report IPC method.
const REPORT_GET: &str = "report/get";
/// A per-range page method, whose reply carries no version stamp.
const REPORT_FOR_RANGE: &str = "report/forRange";
/// The complete generated response used by snippet and range queries.
const FIND_SIMILAR: &str = "duplicates/findSimilar";
/// Required fields in a generated find-similar reply, including an empty query's metadata.
const FIND_SIMILAR_FIELDS: &[&str] = &["clusters", "below_min_nodes", "total_occurrences"];
/// A valid empty query carries zero occurrences and an explicit size-floor verdict.
const EMPTY_OCCURRENCES: usize = 0;
const BELOW_MIN_NODES: bool = false;
/// The endpoint the message must name.
const ENDPOINT: &str = "/workspace/.deslop/cache/deslop.sock";
/// The remedy the message must name.
const REMEDY: &str = "reinstall the Deslop VSIX";
/// The field this wire carries where the foreign report carries `weight`.
const RENAMED_FIELD: &str = "missing field `mass`";
/// The prefix of every raw serde field error.
const FIELD_ERROR: &str = "missing field";
/// What the guard reports as the engine version of an unstamped reply.
const UNSTAMPED: &str = "unknown (reply carries no tool_version)";

/// The foreign report as a JSON value.
fn foreign_release_report() -> Result<Value> {
    Ok(serde_json::from_str(FOREIGN_RELEASE_REPORT)?)
}

/// The endpoint every pin decodes against.
fn endpoint() -> PathBuf {
    PathBuf::from(ENDPOINT)
}

/// The message the guard refuses a report reply with.
fn refused_report_message(method: &str, reply: Value) -> Result<String> {
    let refusal = decode::<Report>(method, &endpoint(), reply).err();
    ensure!(refusal.is_some(), "the guard must refuse this reply");
    Ok(refusal.map(|err| err.to_string()).unwrap_or_default())
}

/// Every element the named condition must carry.
fn ensure_names_the_condition(message: &str, method: &str, engine_version: &str) -> Result<()> {
    let mcp = format!("deslop-mcp {}", crate::version());
    let engine = format!("deslop-lsp {engine_version}");
    ensure!(
        message.contains(&mcp),
        "names this binary's version: {message}"
    );
    ensure!(
        message.contains(&engine),
        "names the reply's producer: {message}"
    );
    ensure!(message.contains(method), "names the IPC method: {message}");
    ensure!(message.contains(ENDPOINT), "names the endpoint: {message}");
    ensure!(message.contains(REMEDY), "names the remedy: {message}");
    Ok(())
}

#[test]
fn a_reply_from_another_release_is_refused_before_it_is_decoded() -> Result<()> {
    let message = refused_report_message(REPORT_GET, foreign_release_report()?)?;
    ensure_names_the_condition(&message, REPORT_GET, FOREIGN_RELEASE_VERSION)?;
    ensure!(
        !message.contains(FIELD_ERROR),
        "refused before decoding, so no field error can appear: {message}"
    );
    Ok(())
}

#[test]
fn a_same_version_reply_that_does_not_decode_names_the_field_the_wire_moved() -> Result<()> {
    let mut reply = foreign_release_report()?;
    let stamped = reply
        .as_object_mut()
        .map(|fields| fields.insert(TOOL_VERSION_FIELD.to_owned(), json!(crate::version())));
    ensure!(stamped.is_some(), "the foreign report is a JSON object");
    let message = refused_report_message(REPORT_GET, reply)?;
    ensure_names_the_condition(&message, REPORT_GET, crate::version())?;
    ensure!(
        message.contains(RENAMED_FIELD),
        "names the field this binary expects and the reply lacks: {message}"
    );
    Ok(())
}

#[test]
fn an_unstamped_reply_that_does_not_decode_says_it_carried_no_version() -> Result<()> {
    let refusal = decode::<Vec<ReportCluster>>(
        REPORT_FOR_RANGE,
        &endpoint(),
        json!({ "clusters": "not a list" }),
    )
    .err();
    ensure!(
        refusal.is_some(),
        "a page that is not a cluster list must be refused"
    );
    let message = refusal.map(|err| err.to_string()).unwrap_or_default();
    ensure_names_the_condition(&message, REPORT_FOR_RANGE, UNSTAMPED)
}

#[test]
fn a_report_from_this_version_decodes() -> Result<()> {
    let reply = serde_json::to_value(fixture_report(Vec::new()))?;
    let report = decode::<Report>(REPORT_GET, &endpoint(), reply)?;
    ensure!(
        report.tool_version == crate::version(),
        "a current report decodes with its own version stamp: {}",
        report.tool_version
    );
    Ok(())
}

/// [MCP-IPC-WIRE-MISMATCH] Remove one required field from an otherwise valid generated reply.
fn incomplete_find_similar_reply(field: &str) -> Result<Value> {
    let mut reply = serde_json::to_value(FindSimilarResult {
        clusters: Vec::new(),
        below_min_nodes: BELOW_MIN_NODES,
        total_occurrences: EMPTY_OCCURRENCES,
    })?;
    let removed = reply
        .as_object_mut()
        .and_then(|fields| fields.remove(field));
    ensure!(
        removed.is_some(),
        "the required field must exist before removal: {field}"
    );
    Ok(reply)
}

#[test]
fn an_incomplete_find_similar_reply_is_refused_without_empty_matches() -> Result<()> {
    for field in FIND_SIMILAR_FIELDS {
        let reply = incomplete_find_similar_reply(field)?;
        let message = decode::<FindSimilarResult>(FIND_SIMILAR, &endpoint(), reply)
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
        ensure_names_the_condition(&message, FIND_SIMILAR, UNSTAMPED)?;
        ensure!(
            message.contains(field),
            "names the missing field: {message}"
        );
    }
    Ok(())
}
