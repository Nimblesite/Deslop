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
// [FUSED-CONTENT-GATE-RENAME] The effect declarations keep every type name
// and rename two: a Type-2 copy.
const EFFECT_KIND: &str = "nearly_identical";
/// alpha's `effect Drafts` row; its `//!` line is a module comment, not part of it.
const ALPHA_EFFECT_ROWS: (u64, u64) = (2, 2);
/// beta's `effect Notes` row with the `///` doc comment that belongs to it.
const BETA_EFFECT_ROWS: (u64, u64) = (1, 2);
/// First row of each file's handler function; nothing from here on is a copy.
const HANDLER_FIRST_ROW: u64 = 3;
const NO_ADDED_CLUSTERS: &[Value] = &[];
const IDENTICAL_KIND: &str = "identical";
const STRUCTURAL_ONLY_KIND: &str = "structural_only";

#[test]
fn detects_identical_osprey_interpolated_expressions() -> Result<()> {
    assert_top_pair(IDENTICAL_FIXTURE, IDENTICAL_KIND)
}

// [FUSED-SHARED-SUBTREE-CORE] The two handlers share one shape, rename every
// local and preserve no literal (`0`/`1`, `"Saved"`/`"Stored"`, …): a
// different vocabulary over one shape, refused whole and at its core
// (`osprey_vocabulary`). What the files do share is their effect
// declaration, and that is the finding — never the handler functions.
#[test]
fn renamed_osprey_effect_handlers_report_only_their_shared_effect() -> Result<()> {
    let json = run_min_nodes(FIXTURE, MIN_NODES)?;
    let cluster = require_type2_cluster(&json, FIRST_FILE, SECOND_FILE)?;
    assert_eq!(field(&cluster, "rank").as_u64(), Some(EXPECTED_RANK));
    assert_eq!(field(&cluster, "kind").as_str(), Some(EFFECT_KIND));
    assert_eq!(rows_in(&cluster, FIRST_FILE), vec![ALPHA_EFFECT_ROWS]);
    assert_eq!(rows_in(&cluster, SECOND_FILE), vec![BETA_EFFECT_ROWS]);
    for clone in report_clusters(&json)?
        .iter()
        .filter(|found| is_clone_kind(found))
    {
        for file in [FIRST_FILE, SECOND_FILE] {
            let rows = rows_in(clone, file);
            assert!(
                rows.iter().all(|(_, last)| *last < HANDLER_FIRST_ROW),
                "no clone may reach {file}'s handler function: {clone:#}"
            );
        }
    }
    Ok(())
}

/// `(start_line, end_line)` of every occurrence of `cluster` in `file`.
fn rows_in(cluster: &Value, file: &str) -> Vec<(u64, u64)> {
    field(cluster, "occurrences")
        .as_array()
        .into_iter()
        .flatten()
        .filter(|occurrence| {
            field(occurrence, "path")
                .as_str()
                .is_some_and(|path| path.ends_with(file))
        })
        .map(|occurrence| {
            let row = |key| field(occurrence, key).as_u64().unwrap_or_default();
            (row("start_line"), row("end_line"))
        })
        .collect()
}

/// Shape-only findings are informational, never clones ([CLONE-BUCKETS-STRUCTURAL-ONLY]).
fn is_clone_kind(cluster: &Value) -> bool {
    field(cluster, "kind").as_str() != Some(STRUCTURAL_ONLY_KIND)
}

/// The fixture's two files form the top-ranked cluster of the expected kind.
fn assert_top_pair(fixture_name: &str, kind: &str) -> Result<()> {
    let json = run_min_nodes(fixture_name, MIN_NODES)?;
    let cluster = require_type2_cluster(&json, FIRST_FILE, SECOND_FILE)?;
    assert_eq!(field(&cluster, "rank").as_u64(), Some(EXPECTED_RANK));
    assert_eq!(field(&cluster, "kind").as_str(), Some(kind));
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
    let cluster = require_type2_cluster(&json, FIRST_FILE, SECOND_FILE)?;
    let removed = require_array(&delta, "/clusters_removed", "Osprey delta")?;
    assert!(removed.iter().any(|id| id == field(&cluster, "id")));
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
