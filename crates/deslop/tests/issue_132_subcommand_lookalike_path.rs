//! [Deslop#132] The CLI must not silently scan a non-existent directory.
//!
//! Repro: `deslop top-offenders` was previously parsed as a positional
//! `PATH=top-offenders`, which doesn't exist, and the pipeline emitted
//! "no duplication detected — your codebase is clean." against zero
//! files. An agent following an AGENTS.md recipe that recommended
//! `deslop top-offenders` therefore reported a confident false negative.
//!
//! New contract: the CLI exits non-zero with a clear message naming
//! `--root .` or `deslop <path>` as the correct form, plus a dedicated
//! hint when the path matches a known MCP tool name / UI label.

use std::{fs, path::Path};

use anyhow::Result;
use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

#[test]
fn deslop_top_offenders_argument_errors_with_actionable_hint() -> Result<()> {
    let cwd = TempDir::new()?;
    let _output = Command::cargo_bin("deslop")?
        .current_dir(cwd.path())
        .arg("top-offenders")
        .assert()
        .failure()
        .stderr(contains("top-offenders"))
        .stderr(contains("MCP tool name"))
        .stderr(contains("deslop ."));
    Ok(())
}

#[test]
fn deslop_find_similar_argument_errors_with_actionable_hint() -> Result<()> {
    let cwd = TempDir::new()?;
    let _output = Command::cargo_bin("deslop")?
        .current_dir(cwd.path())
        .arg("find-similar")
        .assert()
        .failure()
        .stderr(contains("find-similar"))
        .stderr(contains("MCP tool name"));
    Ok(())
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
        .arg("--min-nodes")
        .arg("30")
        .arg("--embeddings")
        .arg("off")
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
