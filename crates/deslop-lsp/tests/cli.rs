//! Contract tests for the `deslop-lsp` binary surface.

mod common;

use std::{
    path::Path,
    process::{Child, Command as StdCommand, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use assert_cmd::Command;
use serde_json::Value;

use crate::common::{
    call, copy_fixture, handshake, notification, request, send_and_recv, spawn_lsp,
    spawn_lsp_on_fixture, take_io, write_frame,
};

// Implements the Shipwright binary contract: every IDE-launched
// executable must expose a stable plain text version line.
// Timeout is generous to accommodate `cargo llvm-cov` instrumentation
// overhead when the whole suite runs concurrently.
#[test]
fn prints_exact_version_contract() -> Result<()> {
    let mut cmd = Command::cargo_bin("deslop-lsp")?;
    let expected = format!("deslop-lsp {}\n", expected_version());
    let _assertion = cmd
        .timeout(Duration::from_secs(10))
        .arg("--version")
        .assert()
        .success()
        .stdout(expected)
        .stderr("");
    Ok(())
}

#[test]
fn prints_json_version_contract() -> Result<()> {
    let output = Command::cargo_bin("deslop-lsp")?
        .timeout(Duration::from_secs(10))
        .arg("--version")
        .arg("--json")
        .output()?;
    assert!(output.status.success(), "status was {}", output.status);
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_version_manifest(&value, "deslop-lsp", "lsp");
    assert!(output.stderr.is_empty(), "stderr must stay empty");
    Ok(())
}

#[test]
fn initialize_reports_server_info_version() -> Result<()> {
    let (_workspace, mut child, mut stdin, mut stdout, _stderr) =
        spawn_lsp_on_fixture("csharp-small")?;
    let init = handshake(&mut stdin, &mut stdout)?;
    assert_eq!(pointer(&init, "/result/serverInfo/name")?, "deslop-lsp");
    assert_eq!(
        pointer(&init, "/result/serverInfo/version")?,
        expected_version()
    );
    let _shutdown = call(&mut stdin, &mut stdout, "shutdown", &Value::Null)?;
    let _ = child.kill();
    Ok(())
}

#[test]
fn exits_when_initialized_parent_process_disappears() -> Result<()> {
    let parent_workspace = copy_fixture("csharp-small")?;
    let mut parent = spawn_fake_parent(parent_workspace.path())?;
    assert!(
        parent.try_wait()?.is_none(),
        "fake parent must stay alive until the LSP records its process id"
    );

    let (_workspace, mut child, mut stdin, mut stdout, _stderr) =
        spawn_lsp_on_fixture("csharp-small")?;
    let init = initialize_with_process_id(&mut stdin, &mut stdout, parent.id())?;
    assert_eq!(pointer(&init, "/result/serverInfo/name")?, "deslop-lsp");

    parent.kill()?;
    let parent_status = parent.wait()?;
    assert!(
        !parent_status.success(),
        "fake parent should be killed during orphan-exit test"
    );
    let exit = wait_for_exit(&mut child, Duration::from_secs(5))?;
    if exit.is_none() {
        let _ = child.kill();
        let _ = child.wait();
    }
    assert!(
        exit.is_some(),
        "deslop-lsp must exit within 5s after initialize.processId disappears"
    );
    Ok(())
}

#[test]
fn reload_uses_fingerprint_cache_for_unchanged_workspace() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let cold = report_after_start(workspace.path())?;
    let cold_hits = cache_stat(&cold, "hits")?;
    let cold_misses = cache_stat(&cold, "misses")?;
    assert_eq!(
        cold_hits, 0,
        "cold LSP start must have zero cache hits — nothing in cache yet: {cold}"
    );
    assert!(
        cold_misses >= 1,
        "cold LSP start must register at least one fingerprint-cache miss: {cold}"
    );
    let warm = report_after_start(workspace.path())?;
    let warm_hits = cache_stat(&warm, "hits")?;
    let warm_misses = cache_stat(&warm, "misses")?;
    assert_eq!(
        warm_misses, 0,
        "warm LSP restart must have zero fingerprint-cache misses — all files must be served from cache: {warm}"
    );
    assert_eq!(
        warm_hits, cold_misses,
        "warm LSP restart must hit the cache for every file that was a miss on the cold run: {warm}"
    );
    Ok(())
}

fn report_after_start(workspace: &Path) -> Result<Value> {
    let mut child = spawn_lsp(workspace)?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    let response = call(&mut stdin, &mut stdout, "deslop/reportGet", &Value::Null)?;
    let _shutdown = call(&mut stdin, &mut stdout, "shutdown", &Value::Null)?;
    let _ = child.kill();
    response
        .get("result")
        .cloned()
        .ok_or_else(|| anyhow!("missing report result: {response}"))
}

fn cache_stat(report: &Value, field: &str) -> Result<u64> {
    report
        .pointer(&format!("/cache_stats/{field}"))
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("missing cache_stats.{field}: {report}"))
}

fn initialize_with_process_id(
    stdin: &mut std::process::ChildStdin,
    stdout: &mut std::io::BufReader<std::process::ChildStdout>,
    process_id: u32,
) -> Result<Value> {
    let (init_id, init) = request(
        "initialize",
        &serde_json::json!({
            "processId": process_id,
            "rootUri": null,
            "capabilities": {}
        }),
    )?;
    let response = send_and_recv(stdin, stdout, init_id, &init)?;
    write_frame(stdin, &notification("initialized", &serde_json::json!({}))?)?;
    Ok(response)
}

fn spawn_fake_parent(workspace: &Path) -> Result<Child> {
    let bin = assert_cmd::cargo::cargo_bin("deslop-lsp");
    Ok(StdCommand::new(bin)
        .arg(workspace)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?)
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> Result<Option<ExitStatus>> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        thread::sleep(Duration::from_millis(50));
    }
    child.try_wait().map_err(Into::into)
}

fn pointer<'a>(value: &'a Value, path: &str) -> Result<&'a str> {
    value
        .pointer(path)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing string at {path}: {value}"))
}

fn assert_version_manifest(value: &Value, name: &str, kind: &str) {
    assert_eq!(value.get("manifestVersion"), Some(&Value::from(1)));
    assert_eq!(value.get("name"), Some(&Value::from(name)));
    assert_eq!(
        value.get("version").and_then(Value::as_str),
        Some(expected_version())
    );
    assert_eq!(value.get("kind"), Some(&Value::from(kind)));
    assert_eq!(value.get("language"), Some(&Value::from("rust")));
    assert_eq!(value.get("product"), Some(&Value::from("deslop")));
}

fn expected_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
