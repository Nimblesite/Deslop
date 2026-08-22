//! E2E pin for [PIPELINE-NORMALIZE-AST-OPERATOR]: the delimiters *inside*
//! a literal are framing, not behaviour, and must never become operator
//! leaves.
//!
//! `OPERATOR_FRAMING_PARENTS` names `LITERAL_KIND` as a framing parent and
//! documents the case it exists for — "a delimiter *inside* a literal,
//! such as a JavaScript regex's `/`". The row is compared against the raw
//! tree-sitter parent kind, which for a regex is `regex`; the normalised
//! kind `__literal__` is never a parent kind tree-sitter produces, so the
//! row can never match and the delimiters are emitted.
//!
//! The cost is the gh #147 mechanism verbatim. A regex that must normalise
//! to one collapsed literal leaf instead spans five nodes
//! (`__literal__` over `__op__/`, `__literal__`, `__op__/`,
//! `__literal__`), which lifts it over the `--min-nodes` floor and makes
//! two regexes that match entirely different text hash the same — the
//! delimiters and the collapsed pattern are all that is left to compare.
//! Two unrelated files then publish their regex constants as `identical`
//! duplication.

use std::fs;

use anyhow::Result;
use assert_cmd::Command;
use serde_json::Value;

use crate::common::*;

/// The fixture: two files whose only shared shape is "a module-level
/// constant holding a regex literal". The patterns match different text
/// and the identifiers differ, so nothing here is duplication.
const FIXTURE: &str = "js-regex-literal-delimiters";

/// Both fixture files, for the cross-file span assertion.
const FIXTURE_FILES: [&str; 2] = ["email_rules.js", "order_codes.js"];

/// Node floor the fixture is pinned at. A correctly collapsed regex is a
/// single leaf and cannot reach it; the inflated form clears it with the
/// delimiters alone.
const MIN_NODES: &str = "4";

/// The normalised kind every literal collapses to.
const LITERAL_KIND: &str = "__literal__";

/// The operator leaf a regex delimiter is wrongly emitted as.
const REGEX_DELIMITER_LEAF: &str = "__op__/";

/// Runs the detector over the fixture and returns the JSON report.
fn run_fixture_report() -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = deslop_cmd(&fixture(FIXTURE), &output)?
        .args(["--min-nodes", MIN_NODES, "--embeddings", "off"])
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

/// The `--debug-ast` dump for one fixture file.
fn debug_ast(file_name: &str) -> Result<String> {
    let output = Command::cargo_bin("deslop")?
        .arg("--debug-ast")
        .arg(fixture(FIXTURE).join(file_name))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    Ok(String::from_utf8(output)?)
}

/// Every path a cluster's occurrences name.
fn occurrence_paths(cluster: &Value) -> Vec<String> {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |values| {
            values
                .iter()
                .filter_map(|occurrence| {
                    occurrence
                        .get("path")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .collect()
        })
}

// [PIPELINE-NORMALIZE-AST-OPERATOR] The normalisation contract, stated
// directly against the dump: a regex literal is one collapsed leaf. A
// delimiter emitted here is a position two unrelated regexes are forced
// to agree on, and it is the whole of what makes them hash alike.
#[test]
fn a_regex_literal_collapses_to_a_single_leaf_with_no_delimiter_operators() -> Result<()> {
    for file_name in FIXTURE_FILES {
        let dump = debug_ast(file_name)?;
        assert!(
            !dump.contains(REGEX_DELIMITER_LEAF),
            "{file_name}: a regex's `/` delimits the literal, it does not \
             compute anything — emitting it as {REGEX_DELIMITER_LEAF} inflates \
             the literal from one node to five and gives two different \
             patterns the same hash:\n{dump}"
        );
        assert_eq!(
            dump.matches(LITERAL_KIND).count(),
            1,
            "{file_name}: the file holds exactly one literal — the regex — and \
             it must collapse to exactly one {LITERAL_KIND} leaf:\n{dump}"
        );
    }
    Ok(())
}

// [CLONE-NOISE] The user-visible half: two regex constants that match
// entirely different text are not duplication, and no cluster may span
// the two files.
#[test]
fn two_unrelated_regex_constants_do_not_publish_as_duplication() -> Result<()> {
    let report = run_fixture_report()?;
    let spanning: Vec<&Value> = clusters(&report)
        .iter()
        .filter(|cluster| {
            let paths = occurrence_paths(cluster);
            FIXTURE_FILES
                .iter()
                .all(|wanted| paths.iter().any(|path| path == wanted))
        })
        .collect();
    assert!(
        spanning.is_empty(),
        "`/[a-z]+@[a-z]+/i` and `/[0-9]{{3}}-[0-9]{{4}}/g` match different text \
         and share no logic — a cluster spanning {FIXTURE_FILES:?} is the gh \
         #147 inflation mechanism reproduced on regex delimiters: {report:#}"
    );
    assert_eq!(
        metric_field(&report, "duplicated_loc").as_u64(),
        Some(0),
        "no line of either file is duplicated: {lines:#?}",
        lines = visible_cluster_lines(&report),
    );
    Ok(())
}
