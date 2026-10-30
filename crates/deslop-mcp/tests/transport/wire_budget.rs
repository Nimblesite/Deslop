//! [MCP-RESULT-SIZE-CAP] Full error frames disclose omitted data and preserve recovery.

use anyhow::{ensure, Result};
use serde_json::Value;

use super::COMPLETE_FRAME_CAP;

pub(super) fn assert_bounded_error(line: &str) -> Result<Value> {
    ensure!(
        line.len() <= COMPLETE_FRAME_CAP,
        "recovery error emitted {} wire bytes",
        line.len()
    );
    let response: Value = serde_json::from_str(line)?;
    ensure!(
        response.get("error").is_some(),
        "backend failure must remain an error"
    );
    assert_recovery_advice(&response)?;
    Ok(response)
}

pub(super) fn assert_protocol_error(line: &str, code: i64) -> Result<()> {
    ensure!(
        line.len() <= COMPLETE_FRAME_CAP,
        "protocol error emitted {} wire bytes",
        line.len()
    );
    let response = assert_bounded_error(line)?;
    ensure!(
        response.get("id") == Some(&Value::Null),
        "unreplyable identifier must be omitted"
    );
    ensure!(
        response.pointer("/error/code").and_then(Value::as_i64) == Some(code),
        "original error code must survive"
    );
    assert_truncation_and_advice(&response)
}

fn assert_recovery_advice(response: &Value) -> Result<()> {
    ensure!(
        response
            .pointer("/error/data/next_action")
            .and_then(Value::as_str)
            .is_some_and(|hint| !hint.is_empty()),
        "bounded error must explain how to obtain full recovery guidance"
    );
    Ok(())
}

fn assert_truncation_and_advice(response: &Value) -> Result<()> {
    ensure!(
        response
            .pointer("/error/data/truncated")
            .and_then(Value::as_bool)
            == Some(true),
        "omitted error details must be explicit"
    );
    ensure!(
        response
            .pointer("/error/data/next_action")
            .and_then(Value::as_str)
            .is_some_and(|hint| !hint.is_empty()),
        "recovery advice must survive"
    );
    Ok(())
}
