//! [OUTPUT-REPLAY-REQUIRED-FIELDS] Replay refuses a report missing any field
//! the engine writes: one usage error that names the field and the remedy.

use super::super::{
    support::*,
    total_include::{seed_three_relation_corpus, TOTAL_INCLUDE_MIN_NODES},
};

const SOURCE_FILE: &str = "stripped.json";
const REPLAY_STEM: &str = "replay";
const REPLAY_FLAG: &str = "--from-report";
const USAGE_CODE: i32 = 2;
const RESCAN_REMEDY: &str = "Re-scan to write a fresh report";
/// Pointer to the object holding each required field, and the field.
const REQUIRED_FIELDS: [(&str, &str); 7] = [
    ("/metrics", "total_includes"),
    ("/metrics", "per_kind"),
    ("/metrics", "per_file"),
    ("/metrics", "folders"),
    ("", "files_skipped"),
    ("", "routing"),
    ("/clusters/0/occurrences/0", "language"),
];
const PROVENANCE_FIELD: &str = "succeeded_subtrees";

fn refused_without(scan: &Path, tmp: &Path, report: &Value, expected: &str) -> Result<()> {
    let source = tmp.join(SOURCE_FILE);
    fs::write(&source, serde_json::to_vec(report)?)?;
    let output = tmp.join(REPLAY_STEM);
    let mut command = deslop_command(scan, &output)?;
    let _assertion = command
        .arg(REPLAY_FLAG)
        .arg(&source)
        .arg(NO_COLOR_FLAG)
        .assert()
        .code(USAGE_CODE)
        .stderr(contains(format!("missing field `{expected}`")))
        .stderr(contains(source.display().to_string()))
        .stderr(contains(RESCAN_REMEDY));
    assert!(
        !with_ext(&output, "json").exists(),
        "a replay refused for `{expected}` writes no report"
    );
    Ok(())
}

#[test]
fn replay_refuses_a_report_missing_any_field_the_engine_writes() -> Result<()> {
    let (tmp, scan) = temp_scan_dir("src")?;
    seed_three_relation_corpus(&scan)?;
    let measured = tmp.path().join(REPORT_OUTPUT_STEM);
    run_scan(&scan, &measured, &[MIN_NODES_FLAG, TOTAL_INCLUDE_MIN_NODES])?;
    let original = read_json_report(&with_ext(&measured, "json"))?;
    for (holder, name) in REQUIRED_FIELDS {
        let mut stripped = original.clone();
        let removed = stripped
            .pointer_mut(holder)
            .and_then(Value::as_object_mut)
            .and_then(|object| object.remove(name));
        assert!(removed.is_some(), "the engine writes {holder}/{name}");
        refused_without(&scan, tmp.path(), &stripped, name)?;
    }
    let mut provenance = original;
    let holder = provenance
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("report is not an object"))?;
    let _previous = holder.insert(
        "embedding_provenance".to_owned(),
        serde_json::json!({
            "provider_id": "ollama",
            "model_id": "model",
            "model_version": "1",
            "dimensions": 8,
            "attempted_subtrees": 2,
            "indexed_subtrees": 2,
            "failed_subtrees": 0,
        }),
    );
    refused_without(&scan, tmp.path(), &provenance, PROVENANCE_FIELD)
}
