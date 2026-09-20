//! [CLONE-BUCKETS-THRESHOLDS] Routing settings through the real CLI and JSON report.

use super::{
    language_sections::{RUST_A, RUST_B},
    support::*,
};
use crate::common::{
    cluster_kind, clusters, occurrence_paths, occurrences, IDENTICAL_KIND, NEARLY_IDENTICAL_KIND,
};

const SCAN_DIRECTORY: &str = "src";
const CONFIG_NAME: &str = ".deslop.toml";
const SOURCE_PATHS: [&str; 2] = ["first.rs", "second.rs"];
const ROUTING_SECTION: &str = "[tuning.routing]\n";
const EMPTY_CONFIG: &str = "";
const ROUTING_FIELD: &str = "routing";
const MIN_SHAPE: &str = "nearly_identical_min_shape";
const MIN_NEAR_CONTENT: &str = "nearly_identical_min_content";
const MIN_SIMILAR_CONTENT: &str = "similar_min_content";
const MAX_SHAPE_CONTENT: &str = "shape_only_max_content";
const DEFAULTS: [(&str, f64); 4] = [
    (MIN_SHAPE, 0.90),
    (MIN_NEAR_CONTENT, 0.70),
    (MIN_SIMILAR_CONTENT, 0.50),
    (MAX_SHAPE_CONTENT, 0.05),
];
const STRICT_SETTINGS: [(&str, f64); 4] = [
    (MIN_SHAPE, 1.0),
    (MIN_NEAR_CONTENT, 1.0),
    (MIN_SIMILAR_CONTENT, 1.0),
    (MAX_SHAPE_CONTENT, 0.0),
];
const OPEN_SHAPE_SETTINGS: [(&str, f64); 4] = [
    (MIN_SHAPE, 0.0),
    (MIN_NEAR_CONTENT, 0.000_001),
    (MIN_SIMILAR_CONTENT, 0.000_001),
    (MAX_SHAPE_CONTENT, 0.0),
];
const HIGH_CEILING_SETTINGS: [(&str, f64); 4] = [
    (MIN_SHAPE, 1.0),
    (MIN_NEAR_CONTENT, 1.0),
    (MIN_SIMILAR_CONTENT, 1.0),
    (MAX_SHAPE_CONTENT, 0.999_999),
];
const PARTIAL_SHAPE: f64 = 0.95;
const INVALID_NUMBERS: [&str; 6] = ["nan", "+nan", "inf", "-inf", "-0.01", "1.01"];
const VALIDATION_MESSAGE: &str = "must be finite and within [0, 1]";
const ORDER_MESSAGE: &str =
    "shape_only_max_content < similar_min_content <= nearly_identical_min_content";
const UNKNOWN_FIELD: &str = "unknown field";
const UNKNOWN_ROUTING_KEY: &str = "unrecognised_content_floor";
const UNKNOWN_TUNING_SECTION: &str = "[tuning.unrecognised]\n";
const SINGLE_CLUSTER: usize = 1;
const FIRST_RANK: u64 = 1;
const ZERO_MASS: u64 = 0;
const RANK_FIELD: &str = "rank";
const MASS_FIELD: &str = "mass";
const METRICS_FIELD: &str = "metrics";
const CLONE_COUNT_FIELD: &str = "clusters_total";

fn routing_config(settings: &[(&str, f64)]) -> String {
    let values = settings
        .iter()
        .map(|(key, value)| format!("{key} = {value}\n"));
    format!("{ROUTING_SECTION}{}", values.collect::<String>())
}

fn routing_command(tmp: &Path, config: &str, second: &str) -> Result<Command> {
    let scan_root = tmp.join(SCAN_DIRECTORY);
    fs::create_dir_all(&scan_root)?;
    for (path, source) in SOURCE_PATHS.iter().zip([RUST_A, second]) {
        fs::write(scan_root.join(path), source)?;
    }
    fs::write(scan_root.join(CONFIG_NAME), config)?;
    let mut command = deslop_command(&scan_root, &tmp.join(REPORT_OUTPUT_STEM))?;
    let _command = command.args([MIN_NODES_FLAG, MIN_NODES_VALUE]);
    Ok(command)
}

fn routing_report(config: &str, second: &str) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let mut command = routing_command(tmp.path(), config, second)?;
    let _assertion = command.assert().success();
    read_json_report(&outputs_under(tmp.path()).json)
}

fn assert_settings(report: &Value, settings: &[(&str, f64)]) -> Result<()> {
    let expected = serde_json::to_value(
        settings
            .iter()
            .copied()
            .collect::<std::collections::BTreeMap<_, _>>(),
    )?;
    assert_eq!(
        field(report, ROUTING_FIELD),
        &expected,
        "effective routing settings"
    );
    Ok(())
}

fn assert_pair(report: &Value, kind: &str) -> Result<()> {
    let groups = clusters(report);
    assert_eq!(groups.len(), SINGLE_CLUSTER, "one complete copied pair");
    let group = groups
        .first()
        .ok_or_else(|| anyhow::anyhow!("missing copied pair"))?;
    assert_eq!(cluster_kind(group), kind);
    assert_eq!(field(group, RANK_FIELD).as_u64(), Some(FIRST_RANK));
    assert!(field(group, MASS_FIELD)
        .as_u64()
        .is_some_and(|mass| mass > ZERO_MASS));
    assert_members(group);
    assert_clone_count(report);
    Ok(())
}

fn assert_members(group: &Value) {
    let paths: std::collections::BTreeSet<_> = occurrence_paths(group).into_iter().collect();
    assert_eq!(occurrences(group).len(), SOURCE_PATHS.len());
    assert_eq!(paths, SOURCE_PATHS.map(str::to_owned).into_iter().collect());
}

fn assert_clone_count(report: &Value) {
    assert_eq!(
        field(field(report, METRICS_FIELD), CLONE_COUNT_FIELD).as_u64(),
        Some(FIRST_RANK)
    );
}

fn assert_invalid(config: &str, message: &str) -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut command = routing_command(tmp.path(), config, RUST_B)?;
    let _assertion = command.assert().failure().stderr(contains(message));
    assert!(
        !outputs_under(tmp.path()).json.exists(),
        "invalid configuration must not write a report"
    );
    Ok(())
}

#[test]
fn defaults_are_recorded_and_near_copies_remain_ranked_clones() -> Result<()> {
    for config in [EMPTY_CONFIG, ROUTING_SECTION] {
        let report = routing_report(config, RUST_B)?;
        assert_settings(&report, &DEFAULTS)?;
        assert_pair(&report, NEARLY_IDENTICAL_KIND)?;
    }
    Ok(())
}

#[test]
fn a_partial_override_preserves_every_other_default() -> Result<()> {
    let config = routing_config(&[(MIN_SHAPE, PARTIAL_SHAPE)]);
    let report = routing_report(&config, RUST_B)?;
    let expected = DEFAULTS.map(|(key, value)| {
        (
            key,
            if key == MIN_SHAPE {
                PARTIAL_SHAPE
            } else {
                value
            },
        )
    });
    assert_settings(&report, &expected)?;
    assert_pair(&report, NEARLY_IDENTICAL_KIND)
}

#[test]
fn zero_one_and_equal_clone_floors_are_valid_and_do_not_overrule_copy_proof() -> Result<()> {
    for settings in [STRICT_SETTINGS, OPEN_SHAPE_SETTINGS, HIGH_CEILING_SETTINGS] {
        let config = routing_config(&settings);
        for (source, expected_kind) in [(RUST_A, IDENTICAL_KIND), (RUST_B, NEARLY_IDENTICAL_KIND)] {
            let report = routing_report(&config, source)?;
            assert_settings(&report, &settings)?;
            assert_pair(&report, expected_kind)?;
        }
    }
    Ok(())
}

#[test]
fn every_routing_setting_rejects_non_finite_and_out_of_range_values() -> Result<()> {
    for (key, _) in DEFAULTS {
        for invalid in INVALID_NUMBERS {
            let config = format!("{ROUTING_SECTION}{key} = {invalid}\n");
            assert_invalid(
                &config,
                &format!("tuning.routing.{key} {VALIDATION_MESSAGE}"),
            )?;
        }
    }
    Ok(())
}

#[test]
fn overlapping_content_boundaries_are_rejected() -> Result<()> {
    const INVALID_ORDER: [[(&str, f64); 2]; 3] = [
        [(MAX_SHAPE_CONTENT, 0.50), (MIN_SIMILAR_CONTENT, 0.50)],
        [(MAX_SHAPE_CONTENT, 0.51), (MIN_SIMILAR_CONTENT, 0.50)],
        [(MIN_SIMILAR_CONTENT, 0.71), (MIN_NEAR_CONTENT, 0.70)],
    ];
    for settings in INVALID_ORDER {
        assert_invalid(&routing_config(&settings), ORDER_MESSAGE)?;
    }
    Ok(())
}

#[test]
fn unknown_routing_keys_and_tuning_sections_are_rejected() -> Result<()> {
    let config = routing_config(&[(UNKNOWN_ROUTING_KEY, PARTIAL_SHAPE)]);
    assert_invalid(&config, UNKNOWN_FIELD)?;
    assert_invalid(UNKNOWN_TUNING_SECTION, UNKNOWN_FIELD)
}
