//! [MCP-IPC-WIRE-MISMATCH] Every IPC reply envelope says which build wrote
//! it and where that build's `deslop-mcp` is, results and errors alike, so a
//! client from another build is refused by name and told what to register.

use std::path::Path;

use serde_json::{json, Value};

use super::{json_rpc_response, BUNDLED_MCP_FIELD, BUNDLED_MCP_NAME, PRODUCER_FIELD};

/// A correlation id for the envelopes under test.
const ID: u64 = 7;

/// Both envelope shapes a handler can produce.
fn envelopes() -> [Value; 2] {
    [
        json_rpc_response(&json!(ID), Ok(json!({ "clusters": [] }))),
        json_rpc_response(
            &json!(ID),
            Err(json!({ "code": -32_602, "message": "bad params" })),
        ),
    ]
}

#[test]
fn every_reply_envelope_stamps_this_build_and_its_bundled_mcp() -> anyhow::Result<()> {
    let beside = std::env::current_exe()?
        .parent()
        .map(|dir| dir.join(BUNDLED_MCP_NAME))
        .ok_or_else(|| anyhow::anyhow!("the test binary has a directory"))?;
    for envelope in envelopes() {
        assert_eq!(
            envelope.get(PRODUCER_FIELD).and_then(Value::as_str),
            Some(deslop_core::version()),
            "the envelope names the build that wrote it: {envelope}"
        );
        let bundled = envelope
            .get(BUNDLED_MCP_FIELD)
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("the envelope names its deslop-mcp: {envelope}"))?;
        assert_eq!(
            Path::new(bundled),
            beside,
            "it is the deslop-mcp beside this binary"
        );
        assert_eq!(envelope.get("id"), Some(&json!(ID)), "the id is echoed");
    }
    let [result, error] = envelopes();
    assert!(
        result.get("result").is_some() && result.get("error").is_none(),
        "{result}"
    );
    assert!(
        error.get("error").is_some() && error.get("result").is_none(),
        "{error}"
    );
    Ok(())
}
