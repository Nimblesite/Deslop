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
        .stdout(contains("Alpha.cs"))
        .stdout(contains("Beta.cs"))
        .stdout(contains("\"signals\""))
        .stdout(contains("\"structural\": 1.0"));
    Ok(())
}

// Implements [PIPELINE-LANG-TRAIT] for Rust: two `.rs` files with the same
// function structure but renamed identifiers must produce a structural
// clone cluster — proves the multi-language pipeline routes files by
// extension to the Rust parser.
#[test]
fn detects_type2_clone_in_rust_fixture() -> Result<()> {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("rust-small");
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(&fixture)
        .arg("--format")
        .arg("json")
        .arg("--min-nodes")
        .arg("10")
        .assert()
        .success()
        .stdout(contains("\"files_analysed\": 2"))
        .stdout(contains("alpha.rs"))
        .stdout(contains("beta.rs"))
        .stdout(contains("\"structural\": 1.0"));
    Ok(())
}

// Implements [PIPELINE-LANG-TRAIT] for Python: two `.py` files with the
// same function structure but renamed identifiers must cluster via the
// Python parser.
#[test]
fn detects_type2_clone_in_python_fixture() -> Result<()> {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("python-small");
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(&fixture)
        .arg("--format")
        .arg("json")
        .arg("--min-nodes")
        .arg("10")
        .assert()
        .success()
        .stdout(contains("\"files_analysed\": 2"))
        .stdout(contains("alpha.py"))
        .stdout(contains("beta.py"))
        .stdout(contains("\"structural\": 1.0"));
    Ok(())
}

// Implements multi-language dispatch in [`crate::pipeline`]: a directory
// mixing `.cs`, `.rs`, and `.py` files must be handled in a single run,
// with each file routed to its language parser by extension
// ([PIPELINE-LANG-TRAIT] + [PIPELINE-DISCOVER-FILES]).
#[test]
fn handles_mixed_language_fixture() -> Result<()> {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("mixed-small");
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(&fixture)
        .arg("--format")
        .arg("json")
        .arg("--min-nodes")
        .arg("10")
        .assert()
        .success()
        .stdout(contains("\"files_analysed\": 3"))
        .stdout(contains("Lib.cs"))
        .stdout(contains("lib.rs"))
        .stdout(contains("lib.py"));
    Ok(())
}

// Implements [DECISION-TYPE3-TWO-PASS] + [FUSION-STRATEGY-MAX-SUM]: two C#
// files with the same method structure and one extra statement inserted in
// one of them are Type-3 near-miss clones. The exact-Merkle pass cannot
// match them (subtree hashes differ), so the token LSH pass must produce a
// cross-file cluster whose `structural` signal is 0 and whose
// `token_jaccard` is high.
#[test]
fn detects_type3_clone_in_csharp_fixture() -> Result<()> {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("csharp-type3");
    let mut cmd = Command::cargo_bin("codededup")?;
    let _assertion = cmd
        .arg(&fixture)
        .arg("--format")
        .arg("json")
        .arg("--min-nodes")
        .arg("15")
        .assert()
        .success()
        .stdout(contains("Delta.cs"))
        .stdout(contains("Epsilon.cs"))
        .stdout(contains("\"structural\": 0.0"))
        .stdout(contains("\"token_jaccard\":"));
    Ok(())
}
