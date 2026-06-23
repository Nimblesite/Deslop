//! E2E regression for GH #126: generated-output headers and the
//! generator template strings that produce them are intentionally
//! related, but they are not actionable duplicate logic.
//! Tests [CLONE-NOISE-PY-GENERATED-OUTPUT]

mod common;

use std::{fs, path::Path};

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::common::*;

fn run_report(scan_root: &Path) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = deslop_cmd(scan_root, &output)?
        .args(["--min-nodes", "1", "--embeddings", "off"])
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

fn occurrence_path(occurrence: &Value) -> Result<&str> {
    occurrence
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("reported occurrence is missing path"))
}

fn occurrence_text(scan_root: &Path, occurrence: &Value) -> Result<String> {
    let path = occurrence_path(occurrence)?;
    let source = fs::read_to_string(scan_root.join(path))?;
    let start = occurrence_byte(occurrence, "start_byte")?;
    let end = occurrence_byte(occurrence, "end_byte")?;
    source
        .get(start..end)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("reported occurrence range is invalid for {path}"))
}

fn generated_template_clusters(report: &Value, scan_root: &Path) -> Result<Vec<String>> {
    let mut offenders = Vec::new();
    for cluster in clusters(report) {
        let mut saw_generator_template = false;
        let mut saw_generated_output = false;
        let mut texts = Vec::new();
        for occurrence in occurrences(cluster) {
            let path = occurrence_path(occurrence)?;
            let text = occurrence_text(scan_root, occurrence)?;
            saw_generator_template |=
                path.ends_with("scripts/gen_contracts.py") && text.contains("DO NOT HAND-EDIT");
            saw_generated_output |= path.ends_with("schemas_generated.py");
            texts.push(format!(
                "{path}: {}",
                text.lines().next().unwrap_or_default()
            ));
        }
        if saw_generator_template && saw_generated_output {
            offenders.push(format!(
                "bucket={:?}, size={:?}, snippets={texts:?}",
                cluster.get("bucket").and_then(Value::as_str),
                cluster.get("size").and_then(Value::as_u64),
            ));
        }
    }
    Ok(offenders)
}

#[test]
fn generated_header_template_does_not_surface_as_duplicate_logic() -> Result<()> {
    let scan_root = fixture("python-generated-template-false-positive");
    let generator = scan_root.join("scripts/gen_contracts.py");
    let generated = scan_root.join("src/agent_backend/api/schemas_generated.py");
    assert!(generator.is_file(), "generator fixture must exist");
    assert!(generated.is_file(), "generated output fixture must exist");
    assert!(
        fs::read_to_string(&generator)?.contains("PY_HEADER"),
        "generator fixture must contain the template literal"
    );
    assert!(
        fs::read_to_string(&generated)?.contains("DO NOT HAND-EDIT"),
        "generated fixture must carry the hand-edit warning"
    );

    let report = run_report(&scan_root)?;
    let offenders = generated_template_clusters(&report, &scan_root)?;
    assert!(
        offenders.is_empty(),
        "generator template strings and generated output must not rank as duplicate logic: {offenders:#?}"
    );
    Ok(())
}
