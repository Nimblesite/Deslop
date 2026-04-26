//! Contract tests for the `deslop-lsp` binary surface.

mod common;

use std::time::Duration;

use anyhow::{anyhow, Result};
use assert_cmd::Command;
use serde_json::Value;

use crate::common::{call, copy_fixture, handshake, spawn_lsp, take_io};

// Implements the deployment-toolkit binary contract: every IDE-launched
// executable must expose a stable plain text version line.
#[test]
fn prints_exact_version_contract() -> Result<()> {
    let mut cmd = Command::cargo_bin("deslop-lsp")?;
    let _assertion = cmd
        .timeout(Duration::from_secs(2))
        .arg("--version")
        .assert()
        .success()
        .stdout("deslop-lsp 0.1.0\n")
        .stderr("");
    Ok(())
}

#[test]
fn prints_json_version_contract() -> Result<()> {
    let output = Command::cargo_bin("deslop-lsp")?
        .timeout(Duration::from_secs(2))
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
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut stdout, _stderr) = take_io(&mut child)?;
    let init = handshake(&mut stdin, &mut stdout)?;
    assert_eq!(pointer(&init, "/result/serverInfo/name")?, "deslop-lsp");
    assert_eq!(pointer(&init, "/result/serverInfo/version")?, "0.1.0");
    let _shutdown = call(&mut stdin, &mut stdout, "shutdown", &Value::Null)?;
    let _ = child.kill();
    Ok(())
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
    assert_eq!(value.get("version"), Some(&Value::from("0.1.0")));
    assert_eq!(value.get("kind"), Some(&Value::from(kind)));
    assert_eq!(value.get("language"), Some(&Value::from("rust")));
    assert_eq!(value.get("product"), Some(&Value::from("deslop")));
}
