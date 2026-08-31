//! Regression coverage for #168 [PIPELINE-NORMALIZE-AST]: a single
//! pathologically deep Dart file must never crash the whole run.
//!
//! A 5000-deep nested collection literal overflowed the post-parse
//! pipeline's recursive tree walks (SIGABRT, exit 134), taking the entire
//! batch run — and the long-lived LSP/MCP server — down with it. The bad
//! file must be skipped gracefully while the rest of the corpus is still
//! analysed.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde_json::Value;

use crate::common::scan_dir::temp_scan_dir;
use crate::common::*;

fn report_path(tmp: &Path) -> PathBuf {
    let mut path = tmp.join("report");
    let _replaced = path.set_extension("json");
    path
}

#[test]
fn deeply_nested_dart_file_is_skipped_not_crashed() -> Result<()> {
    let (tmp, src) = temp_scan_dir("src")?;

    // Pathological: a 5000-deep nested collection literal. Before the fix
    // this overflowed the stack and aborted the process with SIGABRT.
    let deep = format!("var x = {}{};\n", "[".repeat(5000), "]".repeat(5000));
    fs::write(src.join("deep.dart"), deep)?;

    // Two byte-identical helpers so a genuine clone cluster proves the rest
    // of the corpus is still analysed after the bad file is skipped.
    let helper = "int combine(int first, int second) {\n  \
        final total = first + second;\n  return total * total;\n}\n";
    fs::write(src.join("alpha.dart"), helper)?;
    fs::write(src.join("beta.dart"), helper)?;

    let report = report_path(tmp.path());
    let mut cmd = deslop_cmd(&src, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args([
            "--min-nodes",
            "5",
            "--embeddings",
            "off",
            "--notext",
            "--nohtml",
        ])
        .assert()
        .success();

    let body = fs::read_to_string(&report)?;
    let json: Value = serde_json::from_str(&body)?;
    assert!(
        !clusters(&json).is_empty(),
        "the duplicated alpha/beta helper must still cluster after the \
         deep file is skipped: {body}"
    );
    Ok(())
}
