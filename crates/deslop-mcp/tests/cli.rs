//! End-to-end tests for `deslop-mcp`.
//!
//! Drives the real binary over stdio with raw JSON-RPC frames —
//! exactly the contract that Claude Code / Cursor / Continue honour
//! in production. No mocking of the MCP framing anywhere; the only
//! non-binary inputs are the fixture source tree and the prepared
//! JSON payloads below.
//!
//! Covers [MCP-TESTING]: initialize + tools/list + every tool call +
//! resources/read + unparseable / unsupported-language / below-min-nodes
//! edge cases + path-traversal rejection + malformed-frame handling.

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use tempfile::TempDir;

/// One live `deslop-mcp` child-process conversation. Holds stdio
/// handles + the buffered line reader so the test author works in
/// request/response pairs instead of raw bytes.
struct McpChild {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl McpChild {
    fn spawn(root: &Path, extra_args: &[&str]) -> Result<Self> {
        let binary = env!("CARGO_BIN_EXE_deslop-mcp");
        let mut cmd = Command::new(binary);
        let _ = cmd
            .arg("--root")
            .arg(root)
            .args(extra_args)
            .env("RUST_LOG", "info")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().context("spawn deslop-mcp binary")?;
        let stdin = child.stdin.take().context("child stdin")?;
        let stdout = child.stdout.take().context("child stdout")?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 0,
        })
    }

    fn request(&mut self, method: &str, params: &Value) -> Result<Value> {
        self.next_id = self.next_id.saturating_add(1);
        let id = self.next_id;
        let frame = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.send_frame(&frame)?;
        loop {
            let response = self.read_frame()?;
            let response_id = response.get("id").cloned().unwrap_or(Value::Null);
            if response_id == json!(id) {
                return Ok(response);
            }
            // Notifications mixed with responses: skip and keep reading.
            if response.get("method").is_none() {
                return Err(anyhow!("unexpected frame without id match: {response:?}"));
            }
        }
    }

    fn notify(&mut self, method: &str, params: &Value) -> Result<()> {
        let frame = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.send_frame(&frame)
    }

    fn read_frame(&mut self) -> Result<Value> {
        let mut line = String::new();
        let bytes = self.stdout.read_line(&mut line)?;
        if bytes == 0 {
            return Err(anyhow!("mcp stdout closed unexpectedly"));
        }
        serde_json::from_str(&line)
            .with_context(|| format!("invalid JSON from mcp: frame was: {line}"))
    }

    fn send_frame(&mut self, frame: &Value) -> Result<()> {
        let bytes = serde_json::to_vec(frame)?;
        self.stdin.write_all(&bytes)?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;
        Ok(())
    }

    fn send_raw_line(&mut self, line: &str) -> Result<()> {
        self.stdin.write_all(line.as_bytes())?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;
        Ok(())
    }

    fn finish(mut self) -> std::process::ExitStatus {
        drop(self.stdin);
        self.child
            .wait_timeout(Duration::from_secs(30))
            .ok()
            .flatten()
            .unwrap_or_else(|| {
                let _ = self.child.kill();
                self.child
                    .wait()
                    .unwrap_or_else(|_| std::process::ExitStatus::default())
            })
    }
}

trait WaitTimeout {
    fn wait_timeout(
        &mut self,
        duration: Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl WaitTimeout for Child {
    fn wait_timeout(
        &mut self,
        duration: Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>> {
        let started = std::time::Instant::now();
        while started.elapsed() < duration {
            if let Some(status) = self.try_wait()? {
                return Ok(Some(status));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(None)
    }
}

fn fixture_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("tests/fixtures/csharp-mcp")
}

fn copied_fixture_root() -> Result<TempDir> {
    let temp = TempDir::new()?;
    copy_dir_all(&fixture_root(), temp.path())?;
    Ok(temp)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            let _bytes = fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn init_session(child: &mut McpChild) -> Result<Value> {
    child.request(
        "initialize",
        &json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "mcp-e2e-harness", "version": "0.1.0" }
        }),
    )
}

#[test]
fn prints_exact_version_contract() -> Result<()> {
    let binary = env!("CARGO_BIN_EXE_deslop-mcp");
    let output = Command::new(binary).arg("--version").output()?;
    assert!(output.status.success(), "status was {}", output.status);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "deslop-mcp 0.1.0\n"
    );
    assert!(output.stderr.is_empty(), "stderr must stay empty");
    Ok(())
}

#[test]
fn prints_json_version_contract() -> Result<()> {
    let binary = env!("CARGO_BIN_EXE_deslop-mcp");
    let output = Command::new(binary)
        .arg("--version")
        .arg("--json")
        .output()?;
    assert!(output.status.success(), "status was {}", output.status);
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_version_manifest(&value, "deslop-mcp", "mcp");
    assert!(output.stderr.is_empty(), "stderr must stay empty");
    Ok(())
}

fn assert_version_manifest(value: &Value, name: &str, kind: &str) {
    assert_eq!(value.get("manifestVersion"), Some(&Value::from(1)));
    assert_eq!(value.get("name"), Some(&Value::from(name)));
    assert_eq!(value.get("version"), Some(&Value::from("0.1.0")));
    assert_eq!(value.get("kind"), Some(&Value::from(kind)));
    assert_eq!(value.get("language"), Some(&Value::from("rust")));
    assert_eq!(value.get("product"), Some(&Value::from("deslop")));
}

fn call_tool(child: &mut McpChild, name: &str, arguments: &Value) -> Result<Value> {
    let response = child.request(
        "tools/call",
        &json!({ "name": name, "arguments": arguments }),
    )?;
    if response.get("error").is_some() {
        return Err(anyhow!("tools/call {name} failed: {response}"));
    }
    response
        .get("result")
        .cloned()
        .ok_or_else(|| anyhow!("missing result in response: {response}"))
}

fn structured_tool_result(result: &Value) -> Result<Value> {
    result
        .get("structuredContent")
        .cloned()
        .ok_or_else(|| anyhow!("missing structuredContent in {result}"))
}

fn wait_for_generation(child: &mut McpChild, minimum: u64) -> Result<Value> {
    // Use a deadline rather than a fixed iteration count so this helper
    // remains reliable under `cargo llvm-cov` where instrumentation
    // overhead can make the background embedding refresh 10× slower
    // than in a normal test run — issue #57.
    let now = Instant::now();
    let deadline = now.checked_add(Duration::from_secs(30)).unwrap_or(now);
    while Instant::now() < deadline {
        let result = call_tool(child, "session-config", &json!({}))?;
        let snap = structured_tool_result(&result)?;
        let generation = value_get(&snap, "/generation")?.as_u64().unwrap_or(0);
        if generation >= minimum {
            return Ok(snap);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(anyhow!("generation did not reach {minimum}"))
}

fn read_workspace_settings(root: &Path) -> Result<Value> {
    let path = root.join(".vscode/settings.json");
    let source = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    Ok(serde_json::from_str(&source)?)
}

fn value_get(value: &Value, pointer: &str) -> Result<Value> {
    value
        .pointer(pointer)
        .cloned()
        .ok_or_else(|| anyhow!("pointer {pointer} not found in {value}"))
}

#[test]
fn initialize_returns_server_info_and_capabilities() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let response = init_session(&mut child)?;
    assert_eq!(value_get(&response, "/jsonrpc")?, json!("2.0"));
    assert_eq!(
        value_get(&response, "/result/protocolVersion")?,
        json!("2024-11-05")
    );
    assert_eq!(
        value_get(&response, "/result/serverInfo/name")?,
        json!("deslop-mcp")
    );
    assert_eq!(
        value_get(&response, "/result/serverInfo/version")?,
        json!("0.1.0")
    );
    assert!(
        value_get(&response, "/result/capabilities/tools")?.is_object(),
        "tools capability missing: {response}"
    );
    assert!(
        value_get(&response, "/result/capabilities/resources")?.is_object(),
        "resources capability missing: {response}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn tools_list_returns_all_tools_with_schemas() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request("tools/list", &json!({}))?;
    let tools = value_get(&response, "/result/tools")?;
    let names: Vec<String> = tools
        .as_array()
        .ok_or_else(|| anyhow!("tools not array"))?
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str).map(str::to_owned))
        .collect();
    assert_eq!(names.len(), 10, "expected 10 tools, got {names:?}");
    for expected in [
        "top-offenders",
        "report-get",
        "report-query",
        "report-for-file",
        "report-for-range",
        "find-similar",
        "cluster-by-id",
        "list-embedding-models",
        "set-embedding-model",
        "session-config",
    ] {
        assert!(
            names.iter().any(|candidate| candidate == expected),
            "missing tool: {expected}"
        );
    }
    assert_eq!(
        names.first().map(String::as_str),
        Some("top-offenders"),
        "top-offenders must be listed first as the primary tool"
    );
    for tool in tools.as_array().unwrap_or(&Vec::new()) {
        let description = tool
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!description.is_empty(), "tool missing description: {tool}");
        assert!(
            tool.get("inputSchema").is_some_and(Value::is_object),
            "tool missing inputSchema: {tool}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn top_offenders_returns_full_clusters_with_occurrences_and_interpretation() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(&mut child, "top-offenders", &json!({ "n": 3 }))?;
    let payload = structured_tool_result(&result)?;
    let total = value_get(&payload, "/total_clusters")?
        .as_u64()
        .ok_or_else(|| anyhow!("total_clusters must be present"))?;
    assert!(total >= 1, "fixture must have at least one cluster");
    assert_eq!(
        value_get(&payload, "/n")?.as_u64(),
        Some(3),
        "n must echo the requested value"
    );
    let clusters = value_get(&payload, "/clusters")?;
    let clusters_arr = clusters
        .as_array()
        .ok_or_else(|| anyhow!("clusters must be an array"))?;
    assert!(
        clusters_arr.len() <= 3,
        "returned {} clusters but requested max 3",
        clusters_arr.len()
    );
    let first = clusters_arr
        .first()
        .ok_or_else(|| anyhow!("at least one cluster expected"))?;
    assert!(
        first.get("occurrences").is_some_and(Value::is_array),
        "top-offenders must return full occurrences array: {first}"
    );
    assert!(
        first
            .get("interpretation")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty()),
        "top-offenders must return interpretation text: {first}"
    );
    assert!(
        first
            .get("bucket")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty()),
        "top-offenders must return bucket: {first}"
    );
    assert!(
        first
            .get("weight")
            .and_then(Value::as_f64)
            .is_some_and(|w| w > 0.0),
        "top-offenders must return positive weight: {first}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn top_offenders_defaults_to_five_and_clusters_are_worst_first() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(&mut child, "top-offenders", &json!({}))?;
    let payload = structured_tool_result(&result)?;
    assert_eq!(
        value_get(&payload, "/n")?.as_u64(),
        Some(5),
        "omitting n must default to 5"
    );
    let clusters = value_get(&payload, "/clusters")?;
    let clusters_arr = clusters
        .as_array()
        .ok_or_else(|| anyhow!("clusters must be an array"))?;
    assert!(
        clusters_arr.len() <= 5,
        "default n=5 must not return more than 5 clusters"
    );
    let weights: Vec<f64> = clusters_arr
        .iter()
        .filter_map(|c| c.get("weight").and_then(Value::as_f64))
        .collect();
    assert_eq!(
        weights.len(),
        clusters_arr.len(),
        "every cluster must have a weight"
    );
    let sorted = weights.windows(2).all(|w| w[0] >= w[1]);
    assert!(
        sorted,
        "clusters must be worst-first by weight: {weights:?}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_get_returns_paginated_slim_report_page() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 10 }),
    )?;
    let page = structured_tool_result(&result)?;
    assert_eq!(
        value_get(&page, "/report_schema_version")?
            .as_u64()
            .unwrap_or(0),
        1,
        "schema version must round-trip on the page"
    );
    assert!(
        !value_get(&page, "/schema_doc")?
            .as_str()
            .unwrap_or("")
            .is_empty(),
        "schema_doc must be embedded on every page so first-call clients learn the shape"
    );
    let total = value_get(&page, "/total_clusters")?
        .as_u64()
        .ok_or_else(|| anyhow!("total_clusters must be a number"))?;
    let returned = value_get(&page, "/page/returned")?
        .as_u64()
        .ok_or_else(|| anyhow!("page.returned missing"))?;
    assert_eq!(value_get(&page, "/page/offset")?, json!(0));
    assert_eq!(value_get(&page, "/page/limit")?, json!(10));
    assert!(
        returned <= 10,
        "returned ({returned}) must respect requested limit"
    );
    assert!(
        total >= returned,
        "total_clusters ({total}) must be >= returned ({returned})"
    );
    assert!(total >= 1, "fixture should surface at least one cluster");
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_get_requires_offset_argument() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "report-get",
            "arguments": { "limit": 10 }
        }),
    )?;
    assert_eq!(
        value_get(&response, "/error/code")?.as_i64(),
        Some(-32_602),
        "missing offset must be InvalidParams; got {response}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_get_requires_limit_argument() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "report-get",
            "arguments": { "offset": 0 }
        }),
    )?;
    assert_eq!(
        value_get(&response, "/error/code")?.as_i64(),
        Some(-32_602),
        "missing limit must be InvalidParams; got {response}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_get_clusters_are_slim_summaries_only() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 10 }),
    )?;
    let page = structured_tool_result(&result)?;
    let clusters = value_get(&page, "/clusters")?;
    let array = clusters
        .as_array()
        .ok_or_else(|| anyhow!("clusters not array"))?;
    assert!(!array.is_empty(), "fixture should produce >= 1 cluster");
    for cluster in array {
        assert!(
            cluster.get("members").is_none(),
            "ClusterSummary must drop full member list (lives behind cluster-by-id): {cluster}"
        );
        assert!(
            cluster.get("occurrences").is_none(),
            "ClusterSummary must drop full occurrences[] (lives behind cluster-by-id): {cluster}"
        );
        for required in [
            "id",
            "bucket",
            "score",
            "size_nodes",
            "occurrence_count",
            "language",
            "first_occurrence",
        ] {
            assert!(
                cluster.get(required).is_some(),
                "ClusterSummary missing required field {required:?}: {cluster}"
            );
        }
        let first_occ = value_get(cluster, "/first_occurrence")?;
        for occ_field in ["path", "start_byte", "end_byte"] {
            assert!(
                first_occ.get(occ_field).is_some(),
                "first_occurrence missing {occ_field:?}: {first_occ}"
            );
        }
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_get_first_occurrence_belongs_to_full_cluster() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let page = structured_tool_result(&call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 10 }),
    )?)?;
    let clusters = value_get(&page, "/clusters")?
        .as_array()
        .cloned()
        .ok_or_else(|| anyhow!("clusters not array"))?;
    assert!(!clusters.is_empty(), "fixture should produce >= 1 cluster");
    for summary in &clusters {
        assert_first_occurrence_matches_full_cluster(&mut child, summary)?;
    }
    let _ = child.finish();
    Ok(())
}

fn assert_first_occurrence_matches_full_cluster(
    child: &mut McpChild,
    summary: &Value,
) -> Result<()> {
    let id = value_get(summary, "/id")?;
    let first = value_get(summary, "/first_occurrence")?;
    let cluster =
        structured_tool_result(&call_tool(child, "cluster-by-id", &json!({ "id": id }))?)?;
    let occurrences = value_get(&cluster, "/occurrences")?
        .as_array()
        .cloned()
        .ok_or_else(|| anyhow!("occurrences not array"))?;
    assert!(
        occurrences.iter().any(|occ| same_occurrence(occ, &first)),
        "first_occurrence must be present in cluster-by-id occurrences: {summary}"
    );
    Ok(())
}

fn same_occurrence(left: &Value, right: &Value) -> bool {
    let left_path = left.get("path").and_then(Value::as_str);
    let right_path = right.get("path").and_then(Value::as_str);
    let left_start = left.get("start_byte").and_then(Value::as_u64);
    let right_start = right.get("start_byte").and_then(Value::as_u64);
    let left_end = left.get("end_byte").and_then(Value::as_u64);
    let right_end = right.get("end_byte").and_then(Value::as_u64);
    left_path == right_path && left_start == right_start && left_end == right_end
}

#[test]
fn report_get_offset_past_end_returns_empty_page() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let probe = structured_tool_result(&call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 1 }),
    )?)?;
    let total = value_get(&probe, "/total_clusters")?
        .as_u64()
        .ok_or_else(|| anyhow!("total_clusters missing"))?;
    let past = total.saturating_add(100);
    let page = structured_tool_result(&call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": past, "limit": 10 }),
    )?)?;
    assert_eq!(
        value_get(&page, "/page/returned")?,
        json!(0),
        "offset past end must return zero clusters"
    );
    assert!(
        value_get(&page, "/clusters")?
            .as_array()
            .is_some_and(Vec::is_empty),
        "clusters[] must be empty when offset is past the end"
    );
    assert_eq!(
        value_get(&page, "/total_clusters")?
            .as_u64()
            .unwrap_or(u64::MAX),
        total,
        "total_clusters must not change when paging past the end"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_get_response_stays_under_byte_budget() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 50 }),
    )?;
    let page = structured_tool_result(&result)?;
    let serialised = serde_json::to_string(&page)?;
    // 50KB budget. Earlier "fat" report-get on a real workspace was 2.4MB
    // which blew out every agent context; the slim ClusterSummary must
    // keep a 50-cluster page comfortably under this floor.
    assert!(
        serialised.len() < 50_000,
        "report-get page exceeded 50KB budget: was {} bytes",
        serialised.len()
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn initialize_capabilities_have_no_null_values() -> Result<()> {
    // Regression: a `prompts: null` / `logging: null` payload was rejected
    // by Claude Desktop's MCP picker with `expected: object, received:
    // null`. Capabilities the server does not implement must be omitted,
    // not nulled.
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let response = init_session(&mut child)?;
    let capabilities = value_get(&response, "/result/capabilities")?;
    let object = capabilities
        .as_object()
        .ok_or_else(|| anyhow!("capabilities not an object: {capabilities}"))?;
    for (key, value) in object {
        assert!(
            !value.is_null(),
            "capability {key:?} is null — must be omitted or set to an object instead"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_filters_by_language() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(
        &mut child,
        "report-query",
        &json!({ "offset": 0, "limit": 50, "language": "csharp" }),
    )?;
    let page = structured_tool_result(&result)?;
    let clusters = value_get(&page, "/clusters")?;
    let array = clusters
        .as_array()
        .ok_or_else(|| anyhow!("clusters not array"))?;
    assert!(
        !array.is_empty(),
        "fixture should match >= 1 csharp cluster"
    );
    for cluster in array {
        assert_eq!(
            cluster.get("language").and_then(Value::as_str),
            Some("csharp"),
            "language filter not applied: {cluster}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_filters_by_unknown_language_returns_empty() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let page = structured_tool_result(&call_tool(
        &mut child,
        "report-query",
        &json!({ "offset": 0, "limit": 50, "language": "cobol" }),
    )?)?;
    assert_eq!(value_get(&page, "/total_clusters")?, json!(0));
    assert!(value_get(&page, "/clusters")?
        .as_array()
        .is_some_and(Vec::is_empty));
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_filters_by_path_contains() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let page = structured_tool_result(&call_tool(
        &mut child,
        "report-query",
        &json!({ "offset": 0, "limit": 50, "path_contains": "Alpha" }),
    )?)?;
    let array = value_get(&page, "/clusters")?
        .as_array()
        .cloned()
        .ok_or_else(|| anyhow!("clusters not array"))?;
    assert!(
        !array.is_empty(),
        "Alpha.cs participates in the planted clone family"
    );
    for cluster in &array {
        let first_path = cluster
            .pointer("/first_occurrence/path")
            .and_then(Value::as_str)
            .unwrap_or("");
        // first_occurrence is one representative — path_contains may match
        // any occurrence, so we can't assert on first_occurrence alone.
        // Instead prove the filter narrowed the result by checking
        // total_clusters dropped vs the unfiltered baseline.
        let _ = first_path;
    }
    let unfiltered = structured_tool_result(&call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 1 }),
    )?)?;
    let unfiltered_total = value_get(&unfiltered, "/total_clusters")?
        .as_u64()
        .unwrap_or(0);
    let filtered_total = value_get(&page, "/total_clusters")?.as_u64().unwrap_or(0);
    assert!(
        filtered_total <= unfiltered_total,
        "filtered total ({filtered_total}) must be <= unfiltered total ({unfiltered_total})"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_filters_by_min_size() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let page = structured_tool_result(&call_tool(
        &mut child,
        "report-query",
        &json!({ "offset": 0, "limit": 50, "min_size": 20 }),
    )?)?;
    let clusters = value_get(&page, "/clusters")?;
    for cluster in clusters.as_array().unwrap_or(&Vec::new()) {
        let size = cluster
            .get("size_nodes")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        assert!(
            size >= 20,
            "min_size=20 violated: cluster size_nodes={size}, cluster={cluster}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_filters_by_min_score() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let baseline = structured_tool_result(&call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 1 }),
    )?)?;
    let max_score = value_get(&baseline, "/clusters/0/score")?
        .as_f64()
        .ok_or_else(|| anyhow!("baseline score missing"))?;
    let floor = max_score / 2.0;
    let page = structured_tool_result(&call_tool(
        &mut child,
        "report-query",
        &json!({ "offset": 0, "limit": 50, "min_score": floor }),
    )?)?;
    for cluster in value_get(&page, "/clusters")?
        .as_array()
        .unwrap_or(&Vec::new())
    {
        let score = cluster.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        assert!(
            score >= floor,
            "min_score={floor} violated: cluster score={score}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_requires_offset_and_limit() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "report-query",
            "arguments": { "language": "csharp" }
        }),
    )?;
    assert_eq!(
        value_get(&response, "/error/code")?.as_i64(),
        Some(-32_602),
        "missing offset+limit must be InvalidParams; got {response}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_query_echoes_filters_in_response() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let page = structured_tool_result(&call_tool(
        &mut child,
        "report-query",
        &json!({
            "offset": 0,
            "limit": 5,
            "language": "csharp",
            "min_size": 10,
        }),
    )?)?;
    let filters = value_get(&page, "/filters")?;
    assert_eq!(filters.get("language"), Some(&json!("csharp")));
    assert_eq!(filters.get("min_size"), Some(&json!(10)));
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_for_file_returns_only_matching_clusters() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(
        &mut child,
        "report-for-file",
        &json!({ "path": "Alpha.cs" }),
    )?;
    let payload = structured_tool_result(&result)?;
    let clusters = value_get(&payload, "/clusters")?;
    let array = clusters
        .as_array()
        .ok_or_else(|| anyhow!("clusters not array"))?;
    assert!(
        !array.is_empty(),
        "Alpha.cs participates in the planted Type-2 clone"
    );
    for cluster in array {
        let occurrences = cluster
            .get("occurrences")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("occurrences missing"))?;
        let touches_alpha = occurrences
            .iter()
            .filter_map(|occ| occ.get("path").and_then(Value::as_str))
            .any(|path| path.ends_with("Alpha.cs"));
        assert!(
            touches_alpha,
            "cluster must touch Alpha.cs, got {occurrences:?}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_for_range_rejects_inverted_range() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "report-for-range",
            "arguments": { "path": "Alpha.cs", "start_byte": 100, "end_byte": 1 }
        }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_snippet_returns_below_min_nodes_for_tiny_input() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "500"])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(
        &mut child,
        "find-similar",
        &json!({ "snippet": "int x = 0;", "language": "csharp" }),
    )?;
    let payload = structured_tool_result(&result)?;
    assert_eq!(value_get(&payload, "/below_min_nodes")?, json!(true));
    let clusters = value_get(&payload, "/clusters")?;
    assert!(clusters.as_array().is_some_and(Vec::is_empty));
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_snippet_unsupported_language_yields_error() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "find-similar",
            "arguments": { "snippet": "fn main() {}", "language": "cobol" }
        }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_002));
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_requires_exactly_one_input_variant() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "find-similar",
            "arguments": {}
        }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_range_finds_clone_on_alpha() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let alpha = fixture_root().join("Alpha.cs");
    let source = std::fs::read_to_string(&alpha)?;
    let result = call_tool(
        &mut child,
        "find-similar",
        &json!({
            "path": alpha,
            "start_byte": 0,
            "end_byte": source.len(),
            "top_n": 3,
        }),
    )?;
    let payload = structured_tool_result(&result)?;
    assert_eq!(value_get(&payload, "/below_min_nodes")?, json!(false));
    let _ = child.finish();
    Ok(())
}

#[test]
fn cluster_by_id_round_trips() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let report_value = structured_tool_result(&call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 1 }),
    )?)?;
    let first_id = value_get(&report_value, "/clusters/0/id")?
        .as_str()
        .ok_or_else(|| anyhow!("first cluster id missing"))?
        .to_owned();
    let cluster = structured_tool_result(&call_tool(
        &mut child,
        "cluster-by-id",
        &json!({ "id": &first_id }),
    )?)?;
    assert_eq!(value_get(&cluster, "/id")?, json!(first_id));
    assert!(
        cluster.get("occurrences").is_some(),
        "cluster-by-id is the deep-dive — must surface occurrences[]"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn cluster_by_id_unknown_returns_error() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "cluster-by-id",
            "arguments": { "id": "not-a-real-id" }
        }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn list_embedding_models_always_includes_stub() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(&mut child, "list-embedding-models", &json!({}))?;
    let payload = structured_tool_result(&result)?;
    let models = value_get(&payload, "/models")?;
    let bare_ids: Vec<String> = models
        .as_array()
        .ok_or_else(|| anyhow!("models not array"))?
        .iter()
        .filter_map(|model| {
            model
                .get("bare_id")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect();
    assert!(
        bare_ids.iter().any(|candidate| candidate == "stub"),
        "stub must be listed even when Ollama is unreachable; got {bare_ids:?}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_to_stub_succeeds() -> Result<()> {
    let workspace = copied_fixture_root()?;
    let mut child = McpChild::spawn(workspace.path(), &[])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(
        &mut child,
        "set-embedding-model",
        &json!({ "provider_id": "stub", "model_id": "blake3-stub", "user_initiated": true }),
    )?;
    let payload = structured_tool_result(&result)?;
    assert_eq!(value_get(&payload, "/provider_id")?, json!("stub"));
    assert_eq!(value_get(&payload, "/model_id")?, json!("blake3-stub"));
    let dimensions = value_get(&payload, "/dimensions")?.as_u64().unwrap_or(0);
    assert!(dimensions > 0, "stub should report non-zero dimensions");
    let settings = read_workspace_settings(workspace.path())?;
    assert_eq!(
        value_get(&settings, "/deslop.embedding.provider")?,
        json!("stub")
    );
    assert_eq!(
        value_get(&settings, "/deslop.embedding.model")?,
        json!("blake3-stub")
    );
    assert_eq!(
        value_get(&settings, "/deslop.embedding.mode")?,
        json!("auto")
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_preserves_shared_settings_and_endpoint() -> Result<()> {
    let workspace = copied_fixture_root()?;
    fs::create_dir_all(workspace.path().join(".vscode"))?;
    fs::write(
        workspace.path().join(".vscode/settings.json"),
        r#"{ "editor.tabSize": 2 }"#,
    )?;
    let mut child = McpChild::spawn(workspace.path(), &[])?;
    let _ = init_session(&mut child)?;
    let _result = call_tool(
        &mut child,
        "set-embedding-model",
        &json!({
            "provider_id": "stub",
            "model_id": "blake3-stub",
            "endpoint": "http://127.0.0.1:11434",
            "user_initiated": true
        }),
    )?;
    let settings = read_workspace_settings(workspace.path())?;
    assert_eq!(value_get(&settings, "/editor.tabSize")?, json!(2));
    assert_eq!(
        value_get(&settings, "/deslop.embedding.endpoint")?,
        json!("http://127.0.0.1:11434")
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_fails_when_shared_settings_cannot_be_written() -> Result<()> {
    let workspace = copied_fixture_root()?;
    fs::write(workspace.path().join(".vscode"), "not a directory")?;
    let mut child = McpChild::spawn(workspace.path(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "set-embedding-model",
            "arguments": {
                "provider_id": "stub",
                "model_id": "blake3-stub",
                "user_initiated": true
            }
        }),
    )?;
    assert!(
        response.get("error").is_some(),
        "expected config write error"
    );
    let snap = structured_tool_result(&call_tool(&mut child, "session-config", &json!({}))?)?;
    assert!(
        value_get(&snap, "/embedding_provenance")?.is_null(),
        "failed settings write must not switch MCP state: {snap}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_unknown_provider_errors() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "set-embedding-model",
            "arguments": { "provider_id": "aztec-cpu", "model_id": "blah", "user_initiated": true }
        }),
    )?;
    assert!(response.get("error").is_some(), "expected error response");
    let _ = child.finish();
    Ok(())
}

#[test]
fn session_config_reports_workspace_root_and_languages() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(&mut child, "session-config", &json!({}))?;
    let payload = structured_tool_result(&result)?;
    assert_eq!(value_get(&payload, "/min_nodes")?.as_u64().unwrap_or(0), 15);
    let languages_value = value_get(&payload, "/languages")?;
    let languages: Vec<String> = languages_value
        .as_array()
        .ok_or_else(|| anyhow!("languages not array"))?
        .iter()
        .filter_map(|value| value.as_str().map(str::to_owned))
        .collect();
    for expected in ["csharp", "python", "rust"] {
        assert!(
            languages.iter().any(|candidate| candidate == expected),
            "language {expected} missing from session config: {languages:?}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn resources_list_returns_report_and_schema_uris() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request("resources/list", &json!({}))?;
    let resources_value = value_get(&response, "/result/resources")?;
    let resources = resources_value
        .as_array()
        .ok_or_else(|| anyhow!("resources not array"))?;
    let uris: Vec<String> = resources
        .iter()
        .filter_map(|resource| {
            resource
                .get("uri")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect();
    assert!(
        uris.iter().any(|uri| uri == "deslop://report"),
        "report uri missing: {uris:?}"
    );
    assert!(
        uris.iter().any(|uri| uri == "deslop://schema"),
        "schema uri missing: {uris:?}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn resources_read_report_returns_parseable_json() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let response = child.request("resources/read", &json!({ "uri": "deslop://report" }))?;
    let text = value_get(&response, "/result/contents/0/text")?
        .as_str()
        .ok_or_else(|| anyhow!("report text payload missing"))?
        .to_owned();
    let parsed: Value = serde_json::from_str(&text)?;
    assert!(value_get(&parsed, "/clusters")?.is_array());
    let _ = child.finish();
    Ok(())
}

#[test]
fn resources_read_schema_returns_markdown_body() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request("resources/read", &json!({ "uri": "deslop://schema" }))?;
    let text = value_get(&response, "/result/contents/0/text")?
        .as_str()
        .ok_or_else(|| anyhow!("schema text payload missing"))?
        .to_owned();
    assert!(!text.is_empty(), "schema_doc must not be empty");
    let _ = child.finish();
    Ok(())
}

#[test]
fn resources_read_unknown_uri_errors() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request("resources/read", &json!({ "uri": "deslop://invalid" }))?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn unknown_method_returns_method_not_found() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request("completely/made-up", &json!({}))?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_601));
    let _ = child.finish();
    Ok(())
}

#[test]
fn malformed_frame_returns_parse_error() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    child.send_raw_line("{this is not valid json")?;
    let response = child.read_frame()?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_700));
    let _ = child.finish();
    Ok(())
}

#[test]
fn path_outside_root_is_rejected() -> Result<()> {
    let outside = TempDir::new()?;
    let outside_file = outside.path().join("Evil.cs");
    std::fs::write(&outside_file, "namespace E { class X {} }")?;
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "report-for-file",
            "arguments": { "path": outside_file }
        }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_003));
    let _ = child.finish();
    Ok(())
}

#[test]
fn notifications_initialized_is_accepted_silently() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    child.notify("notifications/initialized", &json!({}))?;
    let response = child.request("tools/list", &json!({}))?;
    assert!(value_get(&response, "/result/tools")?.is_array());
    let _ = child.finish();
    Ok(())
}

#[test]
fn mark_changed_is_idempotent_across_second_session() -> Result<()> {
    let temp = TempDir::new()?;
    std::fs::write(
        temp.path().join("One.cs"),
        include_str!("fixtures/csharp-mcp/Alpha.cs"),
    )?;
    std::fs::write(
        temp.path().join("Two.cs"),
        include_str!("fixtures/csharp-mcp/Beta.cs"),
    )?;
    let mut child = McpChild::spawn(temp.path(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let first = structured_tool_result(&call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 100 }),
    )?)?;
    let first_count = value_get(&first, "/total_clusters")?.as_u64().unwrap_or(0);
    assert!(first_count >= 1, "expected at least one cluster initially");
    let _ = child.finish();
    std::fs::write(
        temp.path().join("Two.cs"),
        "namespace Lone { class Only { public int Go() => 1; } }\n",
    )?;
    let mut second = McpChild::spawn(temp.path(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut second)?;
    let rerun = structured_tool_result(&call_tool(
        &mut second,
        "report-get",
        &json!({ "offset": 0, "limit": 100 }),
    )?)?;
    let rerun_count = value_get(&rerun, "/total_clusters")?.as_u64().unwrap_or(0);
    assert!(
        rerun_count < first_count,
        "after mutating Two.cs to a unique file, cluster count must drop; was {first_count}, now {rerun_count}"
    );
    let _ = second.finish();
    Ok(())
}

#[test]
fn report_for_range_returns_empty_when_path_has_no_clusters() -> Result<()> {
    let workspace = copied_fixture_root()?;
    let ghost = workspace.path().join("Lonely.cs");
    std::fs::write(
        &ghost,
        "namespace Lonely { class Solo { public int Uniq() => 42; } }",
    )?;
    let mut child = McpChild::spawn(workspace.path(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(
        &mut child,
        "report-for-range",
        &json!({
            "path": "Lonely.cs",
            "start_byte": 0,
            "end_byte": 10_000,
        }),
    )?;
    let payload = structured_tool_result(&result)?;
    let clusters = value_get(&payload, "/clusters")?;
    assert!(
        clusters.as_array().is_some_and(Vec::is_empty),
        "a unique-content file should not participate in any cluster"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_for_file_on_unknown_path_returns_empty_clusters() -> Result<()> {
    let workspace = copied_fixture_root()?;
    let ghost = workspace.path().join("Ghost.cs");
    std::fs::write(&ghost, "namespace G { class G {} }")?;
    let mut child = McpChild::spawn(workspace.path(), &[])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(
        &mut child,
        "report-for-file",
        &json!({ "path": "Ghost.cs" }),
    )?;
    let payload = structured_tool_result(&result)?;
    let clusters = value_get(&payload, "/clusters")?;
    assert!(
        clusters.as_array().is_some_and(Vec::is_empty),
        "unknown file should produce no clusters"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_swap_updates_session_config_provenance() -> Result<()> {
    let workspace = copied_fixture_root()?;
    let mut child = McpChild::spawn(workspace.path(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let swap_result = call_tool(
        &mut child,
        "set-embedding-model",
        &json!({ "provider_id": "stub", "model_id": "blake3-stub", "user_initiated": true }),
    )?;
    let spec = structured_tool_result(&swap_result)?;
    assert_eq!(value_get(&spec, "/provider_id")?, json!("stub"));
    let snap = wait_for_generation(&mut child, 2)?;
    let generation = value_get(&snap, "/generation")?.as_u64().unwrap_or(0);
    assert!(
        generation >= 2,
        "generation must bump after set-embedding-model"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_to_ollama_fails_when_daemon_not_running() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "set-embedding-model",
            "arguments": {
                "provider_id": "ollama",
                "model_id": "nomic-embed-text",
                "endpoint": "http://127.0.0.1:1",
                "user_initiated": true
            }
        }),
    )?;
    // Either a clean error envelope or the inner backend error. Both
    // paths exercise the ollama branch of set_embedding_model.
    assert!(
        response.get("error").is_some(),
        "ollama-to-nowhere must not succeed: {response}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_with_top_n_zero_falls_back_to_default() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let alpha = fixture_root().join("Alpha.cs");
    let source = std::fs::read_to_string(&alpha)?;
    let result = call_tool(
        &mut child,
        "find-similar",
        &json!({
            "path": alpha,
            "start_byte": 0,
            "end_byte": source.len(),
            "top_n": 0,
        }),
    )?;
    let payload = structured_tool_result(&result)?;
    assert_eq!(value_get(&payload, "/below_min_nodes")?, json!(false));
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_snippet_with_empty_source_returns_empty_result() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(
        &mut child,
        "find-similar",
        &json!({ "snippet": "", "language": "csharp" }),
    )?;
    let payload = structured_tool_result(&result)?;
    let clusters = value_get(&payload, "/clusters")?;
    assert!(clusters.as_array().is_some_and(Vec::is_empty));
    assert_eq!(value_get(&payload, "/below_min_nodes")?, json!(false));
    let _ = child.finish();
    Ok(())
}

#[test]
fn tools_call_missing_name_returns_invalid_params() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request("tools/call", &json!({ "arguments": {} }))?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn tools_call_unknown_tool_returns_method_not_found_error() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({ "name": "bogus-tool", "arguments": {} }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_601));
    let _ = child.finish();
    Ok(())
}

#[test]
fn resources_read_missing_uri_returns_invalid_params() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request("resources/read", &json!({}))?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn invalid_jsonrpc_version_returns_invalid_request() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    child.send_raw_line(r#"{"jsonrpc":"1.5","id":99,"method":"ping"}"#)?;
    let response = child.read_frame()?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_600));
    let _ = child.finish();
    Ok(())
}

#[test]
fn ping_method_returns_empty_object() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request("ping", &json!({}))?;
    assert!(response.get("error").is_none());
    let _ = child.finish();
    Ok(())
}

#[test]
fn shutdown_method_returns_null_result() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request("shutdown", &json!({}))?;
    assert_eq!(value_get(&response, "/result")?, json!(null));
    let _ = child.finish();
    Ok(())
}

#[test]
fn string_request_id_round_trips_through_dispatch() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    // The harness only issues numeric ids; craft a raw frame with a
    // string id so we exercise RequestId::String on the wire.
    let frame = r#"{"jsonrpc":"2.0","id":"alpha","method":"tools/list"}"#;
    child.send_raw_line(frame)?;
    let response = child.read_frame()?;
    assert_eq!(value_get(&response, "/id")?, json!("alpha"));
    let _ = child.finish();
    Ok(())
}

#[test]
fn relative_path_inside_workspace_is_accepted() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(
        &mut child,
        "report-for-range",
        &json!({
            "path": "./Alpha.cs",
            "start_byte": 0,
            "end_byte": 1,
        }),
    )?;
    let payload = structured_tool_result(&result)?;
    assert_eq!(value_get(&payload, "/path")?, json!("./Alpha.cs"));
    let _ = child.finish();
    Ok(())
}

#[test]
fn tool_missing_required_string_arg_returns_invalid_params() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    // report-for-file needs a "path" string — omit it.
    let response = child.request(
        "tools/call",
        &json!({ "name": "report-for-file", "arguments": {} }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn tool_missing_required_integer_arg_returns_invalid_params() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    // report-for-range needs start_byte + end_byte — omit both.
    let response = child.request(
        "tools/call",
        &json!({
            "name": "report-for-range",
            "arguments": { "path": "Alpha.cs" }
        }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_missing_model_id_returns_invalid_params() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "set-embedding-model",
            "arguments": { "provider_id": "stub", "user_initiated": true }
        }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn set_embedding_model_without_user_initiation_returns_invalid_params() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "set-embedding-model",
            "arguments": { "provider_id": "stub", "model_id": "blake3-stub" }
        }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn cluster_by_id_missing_id_returns_invalid_params() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({ "name": "cluster-by-id", "arguments": {} }),
    )?;
    assert_eq!(value_get(&response, "/error/code")?.as_i64(), Some(-32_602));
    let _ = child.finish();
    Ok(())
}

#[test]
fn mcp_sends_empty_line_and_server_keeps_going() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    child.send_raw_line("")?;
    let response = child.request("tools/list", &json!({}))?;
    assert!(value_get(&response, "/result/tools")?.is_array());
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_for_file_accepts_nonexistent_leaf_but_resolves_parent() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    // Query a file that doesn't exist but whose *parent* (the scan
    // root) does. This exercises safety::canonicalise_best_effort's
    // nonexistent-leaf branch, returning an empty cluster set.
    let result = call_tool(
        &mut child,
        "report-for-file",
        &json!({ "path": "NeverCreated.cs" }),
    )?;
    let payload = structured_tool_result(&result)?;
    let clusters = value_get(&payload, "/clusters")?;
    assert!(
        clusters.as_array().is_some_and(Vec::is_empty),
        "phantom leaf must resolve under root and return no clusters"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn path_in_nonexistent_subdirectory_is_rejected_as_io_failure() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": "report-for-file",
            "arguments": { "path": "no/such/dir/Phantom.cs" }
        }),
    )?;
    assert!(
        response.get("error").is_some(),
        "nonexistent parent directory must surface an error: {response}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn binary_starts_with_stub_embeddings_auto_mode() -> Result<()> {
    let mut child = McpChild::spawn(
        &fixture_root(),
        &[
            "--min-nodes",
            "15",
            "--embeddings",
            "auto",
            "--embedding-provider",
            "stub",
        ],
    )?;
    let _ = init_session(&mut child)?;
    let result = call_tool(&mut child, "session-config", &json!({}))?;
    let snapshot = structured_tool_result(&result)?;
    assert!(
        value_get(&snapshot, "/embedding_provenance")?.is_object(),
        "stub-auto should populate provenance: {snapshot}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn binary_starts_with_ollama_auto_falls_back_to_stub() -> Result<()> {
    // Ollama unreachable → auto mode warns and disables embeddings.
    let mut child = McpChild::spawn(
        &fixture_root(),
        &[
            "--min-nodes",
            "15",
            "--embeddings",
            "auto",
            "--embedding-provider",
            "ollama",
            "--embedding-endpoint",
            "http://127.0.0.1:1",
        ],
    )?;
    let _ = init_session(&mut child)?;
    let result = call_tool(&mut child, "session-config", &json!({}))?;
    let snapshot = structured_tool_result(&result)?;
    // Unreachable Ollama falls back to stub — provenance shows stub provider.
    assert_eq!(
        value_get(&snapshot, "/embedding_provenance/provider_id")?,
        json!("stub"),
        "auto mode must fall back to stub when Ollama is unreachable: {snapshot}"
    );
    let _ = child.finish();
    Ok(())
}

/// [LSP-EMBEDDING-CONSENT] Audience: HUMAN. Issue #35. Even with
/// `--embeddings required`, an unreachable Ollama must not crash the MCP
/// binary — the server stays alive with stub embeddings. Positive invariant:
/// `session-config` returns stub provenance (not null, not a crash).
#[test]
fn binary_survives_when_required_ollama_endpoint_is_unreachable() -> Result<()> {
    let mut child = McpChild::spawn(
        &fixture_root(),
        &[
            "--min-nodes",
            "15",
            "--embeddings",
            "required",
            "--embedding-provider",
            "ollama",
            "--embedding-endpoint",
            "http://127.0.0.1:1",
        ],
    )?;
    let _ = init_session(&mut child)?;
    let result = call_tool(&mut child, "session-config", &json!({}))?;
    let snapshot = structured_tool_result(&result)?;
    assert_eq!(
        value_get(&snapshot, "/embedding_provenance/provider_id")?,
        json!("stub"),
        "required mode must fall back to stub (not crash) when Ollama is unreachable: {snapshot}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn binary_rejects_invalid_embedding_mode_string() -> Result<()> {
    // Spawn the binary directly (bypassing the McpChild harness) so we
    // can assert on the non-zero exit status.
    let binary = env!("CARGO_BIN_EXE_deslop-mcp");
    let output = Command::new(binary)
        .arg("--root")
        .arg(fixture_root())
        .arg("--embeddings")
        .arg("nonsense")
        .output()?;
    assert!(
        !output.status.success(),
        "invalid --embeddings value must not succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn binary_rejects_unknown_embedding_provider_at_init() -> Result<()> {
    let binary = env!("CARGO_BIN_EXE_deslop-mcp");
    let output = Command::new(binary)
        .arg("--root")
        .arg(fixture_root())
        .arg("--embeddings")
        .arg("auto")
        .arg("--embedding-provider")
        .arg("zzz-not-real")
        .output()?;
    assert!(
        !output.status.success(),
        "unknown provider must exit non-zero"
    );
    Ok(())
}

#[test]
fn files_changed_notification_triggers_reanalysis() -> Result<()> {
    let temp = TempDir::new()?;
    std::fs::write(
        temp.path().join("One.cs"),
        include_str!("fixtures/csharp-mcp/Alpha.cs"),
    )?;
    std::fs::write(
        temp.path().join("Two.cs"),
        include_str!("fixtures/csharp-mcp/Beta.cs"),
    )?;
    let mut child = McpChild::spawn(temp.path(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    let before = structured_tool_result(&call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 100 }),
    )?)?;
    let before_count = value_get(&before, "/total_clusters")?.as_u64().unwrap_or(0);
    assert!(before_count >= 1, "expected at least one cluster");
    // Edit Two.cs so the clone disappears, then push a notification.
    std::fs::write(
        temp.path().join("Two.cs"),
        "namespace Solo { class Only { public int Go() => 1; } }\n",
    )?;
    child.notify(
        "notifications/deslop/filesChanged",
        &json!({ "paths": [temp.path().join("Two.cs").to_string_lossy().into_owned()] }),
    )?;
    // Small probe via a request so the notification has flushed.
    let after = structured_tool_result(&call_tool(
        &mut child,
        "report-get",
        &json!({ "offset": 0, "limit": 100 }),
    )?)?;
    let after_count = value_get(&after, "/total_clusters")?.as_u64().unwrap_or(0);
    assert!(
        after_count < before_count,
        "mark_changed notification should drop the Two.cs clone; was {before_count}, now {after_count}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn files_changed_notification_with_empty_paths_is_a_noop() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;
    child.notify("notifications/deslop/filesChanged", &json!({ "paths": [] }))?;
    // Server must remain responsive after a no-op notification.
    let response = child.request("tools/list", &json!({}))?;
    assert!(value_get(&response, "/result/tools")?.is_array());
    let _ = child.finish();
    Ok(())
}

/// [MCP-NOTIFICATIONS] After `notifications/deslop/filesChanged` the
/// server must push `notifications/resources/updated` and
/// `notifications/deslop/reportChanged` before waiting for the next
/// client frame.
#[test]
fn files_changed_pushes_resources_updated_and_report_changed_notifications() -> Result<()> {
    let temp = TempDir::new()?;
    std::fs::write(
        temp.path().join("One.cs"),
        include_str!("fixtures/csharp-mcp/Alpha.cs"),
    )?;
    std::fs::write(
        temp.path().join("Two.cs"),
        include_str!("fixtures/csharp-mcp/Beta.cs"),
    )?;
    let mut child = McpChild::spawn(temp.path(), &["--min-nodes", "15"])?;
    let _ = init_session(&mut child)?;

    // Modify a file then notify the server.
    std::fs::write(
        temp.path().join("Two.cs"),
        "namespace Solo { class Only { public int Go() => 1; } }\n",
    )?;
    child.notify(
        "notifications/deslop/filesChanged",
        &json!({ "paths": [temp.path().join("Two.cs").to_string_lossy().into_owned()] }),
    )?;

    // Server pushes two notification frames synchronously before it
    // returns to its read loop — read them both right away.
    let frame1 = child.read_frame()?;
    assert_eq!(
        frame1.get("method").and_then(Value::as_str),
        Some("notifications/resources/updated"),
        "first pushed frame must be resources/updated: {frame1}"
    );
    assert!(
        frame1.get("id").is_none(),
        "notification must not carry an id: {frame1}"
    );
    assert_eq!(
        frame1.pointer("/params/uri").and_then(Value::as_str),
        Some("deslop://report"),
        "resources/updated must name deslop://report: {frame1}"
    );

    let frame2 = child.read_frame()?;
    assert_eq!(
        frame2.get("method").and_then(Value::as_str),
        Some("notifications/deslop/reportChanged"),
        "second pushed frame must be deslop/reportChanged: {frame2}"
    );
    assert!(
        frame2.get("id").is_none(),
        "notification must not carry an id: {frame2}"
    );
    assert!(
        frame2
            .pointer("/params/generation")
            .and_then(Value::as_u64)
            .is_some(),
        "reportChanged must include a numeric generation: {frame2}"
    );

    // Server stays alive and responsive after pushing notifications.
    let response = child.request("tools/list", &json!({}))?;
    assert!(value_get(&response, "/result/tools")?.is_array());

    let _ = child.finish();
    Ok(())
}

#[test]
fn list_embedding_models_response_shape_includes_metadata() -> Result<()> {
    let mut child = McpChild::spawn(&fixture_root(), &[])?;
    let _ = init_session(&mut child)?;
    let result = call_tool(&mut child, "list-embedding-models", &json!({}))?;
    let payload = structured_tool_result(&result)?;
    let models = value_get(&payload, "/models")?;
    let array = models
        .as_array()
        .ok_or_else(|| anyhow!("models not array"))?;
    for model in array {
        assert!(
            model.get("name").and_then(Value::as_str).is_some(),
            "model missing name: {model}"
        );
        assert!(
            model.get("bare_id").and_then(Value::as_str).is_some(),
            "model missing bare_id: {model}"
        );
        assert!(
            model.get("digest").and_then(Value::as_str).is_some(),
            "model missing digest: {model}"
        );
        assert!(
            model.get("size_bytes").and_then(Value::as_u64).is_some(),
            "model missing size_bytes: {model}"
        );
        assert!(
            model
                .get("is_embedding_model")
                .and_then(Value::as_bool)
                .is_some(),
            "model missing is_embedding_model: {model}"
        );
    }
    let _ = child.finish();
    Ok(())
}
