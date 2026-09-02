//! `tools/list` language enum tracks the live engine ([MCP-TOOL-DUPLICATES], gh #255).
//!
//! The `find-similar` prevention gate and the `duplicates` filter
//! advertise a closed `language` enum. Before #255 that enum was baked
//! from the MCP binary's compile-time parser registry while the runtime
//! validator delegated to the *live* engine over IPC — two sources of
//! truth that silently drift apart under any MCP/engine version skew
//! (e.g. an agent pointed at a released MCP binary while its engine
//! already detects a newly added language). The advertised enum then
//! rejects languages the engine actually supports, disabling the
//! Rule-zero gate for those languages.
//!
//! [`tools_list_payload`] now takes the live language set so the enum
//! it advertises is exactly what the validator accepts. Exercised in
//! process against a language (`zig`) the compile-time registry does not
//! contain, so a regression back to the static enum fails here.

use anyhow::{anyhow, Result};
use serde_json::Value;

use deslop_mcp::tools::tools_list_payload;

/// Extracts the `language` enum advertised for `tool_name` in a
/// `tools/list` payload.
fn language_enum_of(tools: &[Value], tool_name: &str) -> Result<Vec<String>> {
    let tool = tools
        .iter()
        .find(|tool| tool.get("name").and_then(Value::as_str) == Some(tool_name))
        .ok_or_else(|| anyhow!("tools/list must expose {tool_name}"))?;
    // `find-similar` takes one `language`; the `duplicates` filter block
    // takes a `languages` array ([MCP-TOOL-FILTERS]). Both advertise the
    // same closed enum.
    let values = [
        "/inputSchema/properties/language/enum",
        "/inputSchema/properties/languages/items/enum",
    ]
    .iter()
    .find_map(|pointer| tool.pointer(pointer).and_then(Value::as_array))
    .ok_or_else(|| anyhow!("{tool_name} must advertise a closed language enum: {tool}"))?;
    Ok(values
        .iter()
        .filter_map(|value| value.as_str().map(str::to_owned))
        .collect())
}

#[test]
fn issue_255_tools_list_language_enum_tracks_live_engine() -> Result<()> {
    // Tests [MCP-TOOL-DUPLICATES], gh #255. `zig` is deliberately not
    // in the compile-time parser registry: if the enum were still baked
    // from `language_ids()` it could never contain it, so this asserts
    // the advertised enum is the passed-in live set.
    let languages = vec!["rust".to_owned(), "zig".to_owned()];
    let payload = tools_list_payload(&languages);
    let tools = payload
        .get("tools")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("tools/list payload must expose a tools array"))?;
    for tool_name in ["find-similar", "duplicates"] {
        let advertised = language_enum_of(tools, tool_name)?;
        assert_eq!(
            advertised, languages,
            "issue #255: {tool_name} language enum must equal the live engine's languages"
        );
    }
    Ok(())
}
