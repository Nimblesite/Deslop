//! End-to-end regression coverage for issue #6's deterministic
//! embedding-pass waste: duplicate subtree snippets must not all enter
//! the ANN index.
//!
//! [REMOVE-STUB] The original test used the deterministic stub
//! provider. Production no longer ships the stub, so we drive the
//! same code path through an inline mock Ollama HTTP server.

#[path = "cli/mock_ollama.rs"]
mod mock_ollama;

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use assert_cmd::Command;
use serde_json::Value;

use crate::mock_ollama::MockOllama;

#[test]
fn duplicate_subtree_embeddings_are_collapsed_before_ann() -> Result<()> {
    let server = MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    write_duplicate_fixture(&scan_root, 8)?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("4")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-provider")
        .arg("ollama")
        .arg("--embedding-model")
        .arg("nomic-embed-text")
        .arg("--embedding-endpoint")
        .arg(server.endpoint())
        .assert()
        .success();
    let provenance = embedding_provenance(tmp.path())?;
    let attempted = metric(&provenance, "attempted_subtrees");
    let indexed = metric(&provenance, "indexed_subtrees");
    assert!(
        indexed > 0,
        "ANN input count must be surfaced: {provenance}"
    );
    assert!(
        indexed < attempted,
        "duplicate subtrees must collapse before ANN indexing: {provenance}"
    );
    Ok(())
}

fn write_duplicate_fixture(dir: &Path, files: usize) -> Result<()> {
    fs::create_dir_all(dir)?;
    for index in 0..files {
        fs::write(dir.join(format!("Clone{index}.cs")), clone_source(index))?;
    }
    Ok(())
}

fn clone_source(index: usize) -> String {
    format!(
        "namespace Perf{index}\n\
         {{\n\
         public class Clone\n\
         {{\n\
         public int Sum(int limit)\n\
         {{\n\
         int total = 0;\n\
         for (int i = 0; i < limit; i = i + 1) {{ total = total + i; }}\n\
         return total;\n\
         }}\n\
         }}\n\
         }}\n"
    )
}

fn embedding_provenance(tmp: &Path) -> Result<Value> {
    let mut path: PathBuf = tmp.join("report");
    let _replaced = path.set_extension("json");
    let report: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    report
        .get("embedding_provenance")
        .cloned()
        .ok_or_else(|| anyhow!("embedding_provenance missing: {report}"))
}

fn metric(provenance: &Value, field: &str) -> u64 {
    provenance
        .get(field)
        .and_then(Value::as_u64)
        .unwrap_or_default()
}
