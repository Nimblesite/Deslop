//! Replay preserves the measured include set and resolves current gates.

use super::super::{
    support::*,
    total_include::{seed_three_relation_corpus, TOTAL_INCLUDE_MIN_NODES},
};

const ALL_KINDS: [&str; 4] = [
    "identical",
    "nearly_identical",
    "loosely_similar",
    "same_behavior",
];
const ALL_CONFIG: &str = "[total]\ninclude = [\"identical\", \"nearly_identical\", \"loosely_similar\", \"same_behavior\"]\n";
const CONFIG_FILE: &str = ".deslop.toml";
const REPLAY_STEM: &str = "replay";
const SOURCE_FILE: &str = "source.json";
const REPLAY_FLAG: &str = "--from-report";
const BREACH_CODE: i32 = 3;
const PASS_CODE: i32 = 0;
const ANALYSED_LOC: u64 = 100;
const KIND_LOC: u64 = 16;
const CONFIG_GATE: &str = "[threshold.kinds]\nidentical = 15\n";
const PASSING_GATE: &str = "[threshold.kinds]\nidentical = 17\n";
const EXCLUDED_MARKER: &str = "clone-group--loosely-similar clone-group--excluded";
const USAGE_CODE: i32 = 2;
const MISSING_CONFIG: &str = "missing.toml";
const NO_GATES_FLAG: &str = "--no-fail-over";
const CONFIG_FLAG: &str = "--config";
const SPLIT_BY_LANGUAGE_FLAG: &str = "--split-by-language";

fn measured_report(scan: &Path, output: &Path, config: &str) -> Result<Value> {
    seed_three_relation_corpus(scan)?;
    fs::write(scan.join(CONFIG_FILE), config)?;
    run_scan(scan, output, &[MIN_NODES_FLAG, TOTAL_INCLUDE_MIN_NODES])?;
    read_json_report(&with_ext(output, "json"))
}

fn replay(scan: &Path, output: &Path, source: &Value, expected_code: i32) -> Result<Value> {
    let source_path = output.with_file_name(SOURCE_FILE);
    fs::write(&source_path, serde_json::to_vec(source)?)?;
    let mut command = deslop_command(scan, output)?;
    let _assertion = command
        .arg(REPLAY_FLAG)
        .arg(source_path)
        .arg(NO_COLOR_FLAG)
        .assert()
        .code(expected_code);
    read_json_report(&with_ext(output, "json"))
}

// [METRICS-TOTAL-INCLUDE] A pre-include report measured every clone kind.
#[test]
fn replay_preserves_all_clone_kinds_when_the_original_include_field_is_absent() -> Result<()> {
    let (tmp, scan) = temp_scan_dir("src")?;
    let original = measured_report(&scan, &tmp.path().join(REPORT_OUTPUT_STEM), ALL_CONFIG)?;
    let mut legacy = original.clone();
    let metrics = legacy
        .get_mut("metrics")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| anyhow::anyhow!("metrics object missing"))?;
    let _removed = metrics.remove("total_includes");
    let _removed = metrics.remove("per_kind");
    let output = tmp.path().join(REPLAY_STEM);
    let actual = replay(&scan, &output, &legacy, PASS_CODE)?;
    assert_eq!(
        metric_field(&actual, "total_includes"),
        &serde_json::json!(ALL_KINDS)
    );
    assert_eq!(field(&actual, "clusters"), field(&original, "clusters"));
    assert_eq!(
        metric_field(&actual, "duplicated_loc").as_u64(),
        Some(ANALYSED_LOC)
    );
    assert_eq!(
        metric_field(&actual, "duplication_percent").as_f64(),
        Some(100.0)
    );
    assert_not_contains(
        &fs::read_to_string(with_ext(&output, "html"))?,
        EXCLUDED_MARKER,
        "legacy measured kinds stay included",
    );
    Ok(())
}

// [EXIT-CODES-KINDS] Replay resolves current configuration like the total gate.
#[test]
fn replay_resolves_current_kind_limits_without_changing_measured_clusters() -> Result<()> {
    let (tmp, scan) = temp_scan_dir("src")?;
    let original = measured_report(&scan, &tmp.path().join(REPORT_OUTPUT_STEM), ALL_CONFIG)?;
    let output = tmp.path().join(REPLAY_STEM);
    for (config, exit, breached) in [
        (CONFIG_GATE, BREACH_CODE, true),
        (PASSING_GATE, PASS_CODE, false),
    ] {
        fs::write(scan.join(CONFIG_FILE), config)?;
        let actual = replay(&scan, &output, &original, exit)?;
        assert_eq!(field(&actual, "clusters"), field(&original, "clusters"));
        let kind = per_kind_row(&actual, "identical")
            .ok_or_else(|| anyhow::anyhow!("identical metric missing"))?;
        assert_eq!(field(&kind, "duplicated_loc").as_u64(), Some(KIND_LOC));
        assert_eq!(
            field(field(&kind, "threshold"), "source").as_str(),
            Some("config")
        );
        assert_eq!(
            field(field(&kind, "threshold"), "breached").as_bool(),
            Some(breached)
        );
    }
    Ok(())
}

// [EXIT-CODES-KINDS] A legacy report cannot silently bypass a configured kind gate.
#[test]
fn replay_requires_kind_measurements_when_kind_limits_are_configured() -> Result<()> {
    let (tmp, scan) = temp_scan_dir("src")?;
    let mut legacy = measured_report(&scan, &tmp.path().join(REPORT_OUTPUT_STEM), ALL_CONFIG)?;
    let _removed = legacy
        .get_mut("metrics")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| anyhow::anyhow!("metrics object missing"))?
        .remove("per_kind");
    fs::write(scan.join(CONFIG_FILE), CONFIG_GATE)?;
    let source = tmp.path().join(SOURCE_FILE);
    fs::write(&source, serde_json::to_vec(&legacy)?)?;
    let mut command = deslop_command(&scan, &tmp.path().join(REPLAY_STEM))?;
    let _assertion = command
        .arg(REPLAY_FLAG)
        .arg(source)
        .arg(NO_COLOR_FLAG)
        .assert()
        .code(USAGE_CODE)
        .stderr(contains("per-kind measurements"));
    Ok(())
}

// [EXIT-CODES-KINDS] Opting out bypasses the config for report replay, like the original total gate.
#[test]
fn replay_without_gates_does_not_require_a_configuration_file() -> Result<()> {
    let (tmp, scan) = temp_scan_dir("src")?;
    let original = measured_report(&scan, &tmp.path().join(REPORT_OUTPUT_STEM), ALL_CONFIG)?;
    let source = tmp.path().join(SOURCE_FILE);
    fs::write(&source, serde_json::to_vec(&original)?)?;
    let output = tmp.path().join(REPLAY_STEM);
    let mut command = deslop_command(&scan, &output)?;
    let _assertion = command
        .arg(REPLAY_FLAG)
        .arg(source)
        .arg(NO_GATES_FLAG)
        .arg(SPLIT_BY_LANGUAGE_FLAG)
        .arg(CONFIG_FLAG)
        .arg(tmp.path().join(MISSING_CONFIG))
        .arg(NO_COLOR_FLAG)
        .assert()
        .success();
    let actual = read_json_report(&with_ext(&output, "json"))?;
    assert_eq!(field(&actual, "clusters"), field(&original, "clusters"));
    let rows = metric_field(&actual, "per_kind")
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("per-kind measurements missing"))?;
    assert_eq!(rows.len(), 3);
    for row in rows {
        assert_eq!(
            field(field(row, "threshold"), "source").as_str(),
            Some("none")
        );
        assert_eq!(
            field(field(row, "threshold"), "breached").as_bool(),
            Some(false)
        );
    }
    assert_eq!(threshold_field(&actual, "source").as_str(), Some("none"));
    Ok(())
}
