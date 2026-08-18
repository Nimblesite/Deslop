//! TEMPORARY diagnostic harness for GH #358. Delete before finishing.
#[path = "cli/mock_ollama.rs"]
mod mock_ollama;

use std::path::Path;

use anyhow::Result;
use mock_ollama::MockOllama;

mod common;
use crate::common::*;

fn dump(name: &str) -> Result<()> {
    let fixture_root = fixture(name);
    let server = MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let scan_root = tmp.path().join("src");
    seed(&fixture_root, &scan_root)?;
    let mut command = deslop_cmd(&scan_root, &output)?;
    let assertion = command
        .env("RUST_LOG", "deslop_core=debug")
        .args([
            "--min-nodes",
            "5",
            "--embeddings",
            "required",
            "--embedding-provider",
            "ollama",
            "--embedding-model",
            "nomic-embed-text",
            "--embedding-endpoint",
            server.endpoint(),
        ])
        .assert()
        .success();
    let out = assertion.get_output();
    println!("===== {name} STDERR =====");
    println!("{}", String::from_utf8_lossy(&out.stderr));
    println!("===== {name} LOG =====");
    let logs = scan_root.join("../logs");
    if let Ok(entries) = std::fs::read_dir(&logs) {
        for entry in entries.flatten() {
            let body = std::fs::read_to_string(entry.path()).unwrap_or_default();
            for line in body.lines() {
                if line.contains("pairs") || line.contains("clusters") || line.contains("hidden") || line.contains("embedding") {
                    println!("{line}");
                }
            }
        }
    }
    println!("===== {name} JSON =====");
    println!(
        "{}",
        std::fs::read_to_string(Path::new(&output.with_extension("json")))?
    );
    Ok(())
}

#[test]
fn diag_same_role() -> Result<()> {
    dump("python-issue-119-same-role")
}

#[test]
fn diag_role_mismatch() -> Result<()> {
    dump("python-issue-119-role-mismatch")
}
