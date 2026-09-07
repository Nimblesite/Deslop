//! Unit pins for [MCP-IPC-WIRE-MISMATCH] at the decode seam: a foreign
//! version is refused before decoding, a same-version drift is named by
//! the field the wire moved, an unstamped reply is named honestly, and a
//! report from this version decodes.

use std::path::PathBuf;

use deslop_core::{report::ReportCluster, report_fixtures::fixture_report, Report};
use serde_json::{json, Value};

use super::decode;

/// A report written by the wire before `weight` became `mass`, stamped
/// by the release that wrote it.
const LEGACY_REPORT: &str = include_str!("../../../tests/fixtures/legacy-weight-report.json");
/// The producer version the legacy fixture is stamped with.
const LEGACY_TOOL_VERSION: &str = "0.31.0";
/// The field a whole report stamps its producer version in.
const TOOL_VERSION_FIELD: &str = "tool_version";
/// The whole-report IPC method.
const REPORT_GET: &str = "report/get";
/// A per-range page method, whose reply carries no version stamp.
const REPORT_FOR_RANGE: &str = "report/forRange";
/// The endpoint the message must name.
const ENDPOINT: &str = "/workspace/.deslop/cache/deslop.sock";
/// The remedy the message must name.
const REMEDY: &str = "reinstall the Deslop VSIX";
/// The field the current wire carries where the legacy wire carried `weight`.
const RENAMED_FIELD: &str = "missing field `mass`";
/// The prefix of every raw serde field error.
const FIELD_ERROR: &str = "missing field";
/// What the guard reports as the engine version of an unstamped reply.
const UNSTAMPED: &str = "unknown (reply carries no tool_version)";

/// The legacy fixture as a JSON value.

fn legacy_report() -> Value {
    serde_json::from_str(LEGACY_REPORT).expect("the legacy fixture is JSON")
}

/// The endpoint every pin decodes against.

fn endpoint() -> PathBuf {
    PathBuf::from(ENDPOINT)
}

/// The message the guard refuses a report reply with.

fn refused_report_message(method: &str, reply: Value) -> String {
    decode::<Report>(method, &endpoint(), reply)
        .expect_err("the guard must refuse this reply")
        .to_string()
}

/// Every element the named condition must carry.

fn assert_names_the_condition(message: &str, method: &str, engine_version: &str) {
    let mcp = format!("deslop-mcp {}", crate::version());
    let engine = format!("deslop-lsp {engine_version}");
    assert!(
        message.contains(&mcp),
        "names this binary's version: {message}"
    );
    assert!(
        message.contains(&engine),
        "names the reply's producer: {message}"
    );
    assert!(message.contains(method), "names the IPC method: {message}");
    assert!(message.contains(ENDPOINT), "names the endpoint: {message}");
    assert!(message.contains(REMEDY), "names the remedy: {message}");
}

#[test]
fn a_reply_from_another_release_is_refused_before_it_is_decoded() {
    let message = refused_report_message(REPORT_GET, legacy_report());
    assert_names_the_condition(&message, REPORT_GET, LEGACY_TOOL_VERSION);
    assert!(
        !message.contains(FIELD_ERROR),
        "refused before decoding, so no field error can appear: {message}"
    );
}

#[test]
fn a_same_version_reply_that_does_not_decode_names_the_field_the_wire_moved() {
    let mut reply = legacy_report();
    reply[TOOL_VERSION_FIELD] = Value::String(crate::version().to_owned());
    let message = refused_report_message(REPORT_GET, reply);
    assert_names_the_condition(&message, REPORT_GET, crate::version());
    assert!(
        message.contains(RENAMED_FIELD),
        "names the field this binary expects and the reply lacks: {message}"
    );
}

#[test]
fn an_unstamped_reply_that_does_not_decode_says_it_carried_no_version() {
    let message = decode::<Vec<ReportCluster>>(
        REPORT_FOR_RANGE,
        &endpoint(),
        json!({ "clusters": "not a list" }),
    )
    .expect_err("a page that is not a cluster list must be refused")
    .to_string();
    assert_names_the_condition(&message, REPORT_FOR_RANGE, UNSTAMPED);
}

#[test]
fn a_report_from_this_version_decodes() {
    let reply = serde_json::to_value(fixture_report(Vec::new())).expect("a report serialises");
    let report =
        decode::<Report>(REPORT_GET, &endpoint(), reply).expect("a current report decodes");
    assert_eq!(report.tool_version, crate::version());
}
