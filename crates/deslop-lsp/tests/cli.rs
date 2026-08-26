//! Contract tests for the `deslop-lsp` binary surface.
//!
//! Tests [LSP-TESTING] — spawns the real LSP binary and talks JSON-RPC
//! over stdio against fixture workspaces; no mocked live session.

use std::{
    path::Path,
    process::{Child, Command as StdCommand, Stdio},
    time::Duration,
};

use anyhow::{anyhow, Result};
use assert_cmd::Command;
use serde_json::Value;

use crate::common::{
    call, copy_fixture, handshake, notification, request, send_and_recv, spawn_lsp,
    spawn_lsp_on_fixture, take_io, wait_for_exit, write_frame,
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

// Issue #201: a rootless launch (the transport flag with no workspace root,
// e.g. `deslop-lsp --stdio` from a folderless VS Code window) must FAIL
// LOUDLY — exit non-zero AND write the usage error to stderr. The original
// crash was preceded by the parser taking `--stdio` as the root; the guard
// now rejects it, and the diagnostic must be visible (no silent no-op — the
// subscriber is installed at the process boundary, not only on the serve
// path). stdout stays clean so it never corrupts a JSON-RPC stream.
#[test]
fn rootless_launch_fails_loudly_with_usage_on_stderr() -> Result<()> {
    for args in [vec!["--stdio"], vec!["--debug", "--stdio"]] {
        let output = Command::cargo_bin("deslop-lsp")?
            .timeout(Duration::from_secs(10))
            .args(&args)
            .output()?;
        assert!(
            !output.status.success(),
            "rootless `deslop-lsp {args:?}` must exit non-zero, got {}",
            output.status
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("usage: deslop-lsp"),
            "rootless launch must surface the usage error on stderr (no silent no-op), got stderr={stderr:?}"
        );
        assert!(
            output.stdout.is_empty(),
            "rootless launch must not write to stdout, got {:?}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
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
    let _status = deslop_test_support::reap::reap_with_stdin(&mut child, stdin);
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
    let _status = deslop_test_support::reap::reap_with_stdin(&mut child, stdin);
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

fn pointer<'a>(value: &'a Value, path: &str) -> Result<&'a str> {
    value
        .pointer(path)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing string at {path}: {value}"))
}

fn assert_version_manifest(value: &Value, name: &str, kind: &str) {
    deslop_test_support::assert_version_manifest(value, name, kind, expected_version());
}

fn expected_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
