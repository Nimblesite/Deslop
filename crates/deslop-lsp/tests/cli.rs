//! Contract tests for the `deslop-lsp` binary surface.

use std::time::Duration;

use anyhow::Result;
use assert_cmd::Command;

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
