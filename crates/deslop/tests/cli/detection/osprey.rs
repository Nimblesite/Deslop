//! [LANG-CAND-OSPREY] Osprey goes through the same CLI as every language.

use super::*;

const FIXTURE: &str = "osprey-small";
const IDENTICAL_FIXTURE: &str = "osprey-identical";
const MIN_NODES: &str = "10";
const FIRST_FILE: &str = "alpha.osp";
const SECOND_FILE: &str = "beta.osp";
const INTERPOLATION_FIXTURE: &str = "osprey-interpolation";
const INTERPOLATION_FILE: &str = "calculate.osp";
const DEBUG_AST_FLAG: &str = "--debug-ast";
const ADDITION_OPERATOR: &str = "__op__+";
const SUBTRACTION_OPERATOR: &str = "__op__-";
const REMOVE_FLAG: &str = "--rerun-remove";
const DELTA_EXTENSION: &str = "delta.json";
const EXPECTED_RANK: u64 = 1;
// [CLONE-BUCKETS-ROUTING] Shape alone does not select the near-identical bucket.
const RENAMED_KIND: &str = "loosely_similar";
const NO_ADDED_CLUSTERS: &[Value] = &[];
const IDENTICAL_KIND: &str = "identical";

#[test]
fn detects_identical_osprey_interpolated_expressions() -> Result<()> {
    assert_top_pair(IDENTICAL_FIXTURE, IDENTICAL_KIND)
}

#[test]
fn detects_renamed_osprey_effect_handlers() -> Result<()> {
    assert_top_pair(FIXTURE, RENAMED_KIND)
}

/// The fixture's two files form the top-ranked cluster of the expected kind.
fn assert_top_pair(fixture_name: &str, kind: &str) -> Result<()> {
    let json = run_min_nodes(fixture_name, MIN_NODES)?;
    assert_type2_report(&json, FIRST_FILE, SECOND_FILE)?;
    let clusters = report_clusters(&json)?;
    let cluster = require_cluster_spanning(&clusters, FIRST_FILE, SECOND_FILE)?;
    assert_eq!(field(cluster, "rank").as_u64(), Some(EXPECTED_RANK));
    assert_eq!(field(cluster, "kind").as_str(), Some(kind));
    Ok(())
}

#[test]
fn preserves_osprey_interpolation_operators() -> Result<()> {
    let ast = interpolation_ast()?;
    for operator in [ADDITION_OPERATOR, SUBTRACTION_OPERATOR] {
        assert_contains(&ast, operator, "interpolation retains executable operators");
    }
    Ok(())
}

/// Drive the rendered CLI AST rather than calling parser internals.
fn interpolation_ast() -> Result<String> {
    let (_tmp, mut command) = fixture_run_command(INTERPOLATION_FIXTURE)?;
    let output = command
        .arg(DEBUG_AST_FLAG)
        .arg(fixture(INTERPOLATION_FIXTURE).join(INTERPOLATION_FILE))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    Ok(String::from_utf8(output)?)
}

#[test]
fn osprey_file_removal_updates_the_live_report() -> Result<()> {
    let (json, delta) = removal_report()?;
    assert_type2_report(&json, FIRST_FILE, SECOND_FILE)?;
    let clusters = report_clusters(&json)?;
    let cluster = require_cluster_spanning(&clusters, FIRST_FILE, SECOND_FILE)?;
    let removed = require_array(&delta, "/clusters_removed", "Osprey delta")?;
    assert!(removed.iter().any(|id| id == field(cluster, "id")));
    assert_eq!(
        require_array(&delta, "/clusters_added", "Osprey delta")?.as_slice(),
        NO_ADDED_CLUSTERS
    );
    assert!(
        require_u64(&delta, "/to_generation", "Osprey delta")?
            > require_u64(&delta, "/from_generation", "Osprey delta")?
    );
    Ok(())
}

/// Exercise the same session update that filesystem watchers trigger.
fn removal_report() -> Result<(String, Value)> {
    let (tmp, root) = seeded_fixture_root(FIXTURE)?;
    let prefix = tmp.path().join(REPORT_OUTPUT_STEM);
    run_scan(&root, &prefix, &[MIN_NODES_FLAG, MIN_NODES])?;
    let json = fs::read_to_string(with_ext(&prefix, "json"))?;
    let mut command = deslop_command(&root, &prefix)?;
    let _assertion = command
        .args([MIN_NODES_FLAG, MIN_NODES, REMOVE_FLAG])
        .arg(root.join(SECOND_FILE))
        .assert()
        .success();
    let delta = read_json_report(&with_ext(&prefix, DELTA_EXTENSION))?;
    Ok((json, delta))
}
