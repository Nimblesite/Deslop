//! End-to-end CLI tests. Per `CLAUDE.md`, these are the only kind of test the
//! project ships — driving the binary as a black box against fixture input
//! and asserting on stdout/stderr/exit code.

use assert_cmd::Command;
use predicates::str::contains;

// Implements [CLI-INVOCATION-VERSION]: `codededup --version` prints the binary
// name and exits 0. Keeps a stable, machine-readable version line for agents
// that probe tool identity.
#[test]
fn prints_version_and_exits_zero() {
    let mut cmd = Command::cargo_bin("codededup").expect("binary builds");
    let _assertion = cmd
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("codededup"));
}

// Implements [CLI-INVOCATION-HELP]: `--help` advertises the configurable
// minimum-subtree-nodes flag so agents can discover the tuning surface.
#[test]
fn prints_help_and_mentions_min_nodes_flag() {
    let mut cmd = Command::cargo_bin("codededup").expect("binary builds");
    let _assertion = cmd
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("--min-nodes"));
}

// Implements [CLI-INVOCATION-PATH]: passing an empty directory must not panic
// and must exit 0. Locks in the "no clones found ≠ error" contract.
#[test]
fn accepts_path_argument_without_panicking() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut cmd = Command::cargo_bin("codededup").expect("binary builds");
    let _assertion = cmd.arg(tmp.path()).assert().success();
}
