//! [Deslop#132] The CLI must not silently scan a non-existent directory.
//! Tests [CLI-SUBCOMMAND-LOOKALIKE]
//!
//! Repro: `deslop top-offenders` was previously parsed as a positional
//! `PATH=top-offenders`, which doesn't exist, and the pipeline emitted
//! "no duplication detected — your codebase is clean." against zero
//! files. An agent following an AGENTS.md recipe that recommended
//! `deslop top-offenders` therefore reported a confident false negative.
//!
//! New contract: the CLI exits non-zero with a clear message naming
//! `deslop .` or `deslop <path>` as the correct form, plus a dedicated
//! hint when the path matches a known MCP tool name / UI label.

use std::{fs, path::Path};

use anyhow::Result;
use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

/// The binary under test.
const DESLOP_BIN: &str = "deslop";
/// The VSIX panel label agent recipes mistake for a subcommand.
const UI_LABEL: &str = "top-offenders";
/// Every tool the MCP server registers ([MCP-TOOLS], [AUTOFIX-MERGE-MCP]).
/// None of them is a CLI subcommand.
const MCP_TOOL_NAMES: [&str; 8] = [
    "find-similar",
    "duplicates",
    "compare-pair",
    "cluster-by-id",
    "rescan",
    "session",
    "schema-doc",
    "merge-plan",
];
/// What the dedicated message calls the rejected word.
const MCP_NAME_HINT: &str = "MCP tool name";
/// The correct invocation the message must name.
const CORRECT_FORM: &str = "deslop .";
/// The server the message must point the MCP tool form at.
const MCP_SERVER: &str = "deslop-mcp";
/// Where a scan would have written its report ([OUTPUT-DIR]).
const REPORT_DIRECTORY: &str = ".deslop";

/// [CLI-SUBCOMMAND-LOOKALIKE] `deslop <word>` is refused with the dedicated
/// message — the word, what it really is, the correct form, and where the MCP
/// form lives — and scans nothing.
fn assert_refused_as_mcp_name(word: &str) -> Result<()> {
    let cwd = TempDir::new()?;
    let _output = Command::cargo_bin(DESLOP_BIN)?
        .current_dir(cwd.path())
        .arg(word)
        .assert()
        .failure()
        .stderr(contains(word))
        .stderr(contains(MCP_NAME_HINT))
        .stderr(contains(CORRECT_FORM))
        .stderr(contains(MCP_SERVER));
    assert!(
        !cwd.path().join(REPORT_DIRECTORY).exists(),
        "`deslop {word}` was refused, so it must not have written a report"
    );
    Ok(())
}

#[test]
fn deslop_top_offenders_argument_errors_with_actionable_hint() -> Result<()> {
    assert_refused_as_mcp_name(UI_LABEL)
}

#[test]
fn deslop_find_similar_argument_errors_with_actionable_hint() -> Result<()> {
    MCP_TOOL_NAMES
        .into_iter()
        .try_for_each(assert_refused_as_mcp_name)
}

#[test]
fn deslop_nonexistent_path_errors_instead_of_clean_scan() -> Result<()> {
    let cwd = TempDir::new()?;
    let missing = cwd.path().join("does-not-exist");
    let _output = Command::cargo_bin("deslop")?
        .arg(&missing)
        .assert()
        .failure()
        .stderr(contains("does not exist"));
    Ok(())
}

#[test]
fn deslop_existing_path_still_runs_to_completion() -> Result<()> {
    let cwd = TempDir::new()?;
    let scan = cwd.path().join("workspace");
    fs::create_dir_all(&scan)?;
    write_minimal_csharp_file(&scan)?;
    let report_base = cwd.path().join("report");
    let _output = Command::cargo_bin("deslop")?
        .arg(&scan)
        .args(["--min-nodes", "30", "--embeddings", "off"])
        .arg("--output")
        .arg(&report_base)
        .assert()
        .success();
    Ok(())
}

fn write_minimal_csharp_file(scan: &Path) -> Result<()> {
    let source = b"namespace N { public class A { public int X => 1; } }\n";
    fs::write(scan.join("A.cs"), source)?;
    Ok(())
}
