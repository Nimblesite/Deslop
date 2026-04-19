//! End-to-end CLI tests. Per `CLAUDE.md`, these are the only kind of test the
//! project ships — driving the binary as a black box against fixture input
//! and asserting on stdout/stderr/exit code.

use anyhow::Result;
use assert_cmd::Command;
use predicates::str::contains;

// Implements [CLI-INVOCATION-VERSION]: `codededup --version` prints the binary
// name and exits 0. Keeps a stable, machine-readable version line for agents
// that probe tool identity.
#[test]
fn prints_version_and_exits_zero() -> Result<()> {
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("codededup"));
    Ok(())
}

// Implements [CLI-INVOCATION-HELP]: `--help` advertises the configurable
// minimum-subtree-nodes flag so agents can discover the tuning surface.
#[test]
fn prints_help_and_mentions_min_nodes_flag() -> Result<()> {
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("--min-nodes"));
    Ok(())
}

// Implements [CLI-INVOCATION-PATH]: passing an empty directory must not panic
// and must exit 0. Locks in the "no clones found ≠ error" contract.
#[test]
fn accepts_path_argument_without_panicking() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd.arg(tmp.path()).assert().success();
    Ok(())
}

// Implements [PIPELINE-CLUSTER-EXACT] + [PIPELINE-NORMALIZE-AST]: two C# files
// with the same structure but renamed identifiers (Type-2 clone) must produce
// a cluster of size 2 in the rendered JSON report.
#[test]
fn detects_type2_clone_in_csharp_fixture() -> Result<()> {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("csharp-small");
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(&fixture)
        .arg("--format")
        .arg("json")
        .arg("--min-nodes")
        .arg("8")
        .assert()
        .success()
        .stdout(contains("\"report_schema_version\": 1"))
        .stdout(contains("\"files_analysed\": 2"))
        .stdout(contains("\"size\": 2"))
        .stdout(contains("Alpha.cs"))
        .stdout(contains("Beta.cs"));
    Ok(())
}
