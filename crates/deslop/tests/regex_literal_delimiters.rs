//! E2E pin for [PIPELINE-NORMALIZE-AST-OPERATOR]: the delimiters *inside*
//! a literal are framing, not behaviour, and must never become operator
//! leaves.
//!
//! A regex's `/` delimits the literal; it computes nothing. Emitting it
//! reproduced the gh #147 mechanism verbatim: the literal grew from the
//! parts it is made of to those parts *plus* two `__op__/` leaves, which
//! lifted `const name = /.../;` from four nodes to eight and over the
//! `--min-nodes` floor. Two unrelated files then published their regex
//! constants as duplication at `duplication_percent: 40.0`.
//!
//! The delimiters are what must go, not the literal's parts. Its named
//! parts stay: [FUSED-CONTENT-GATE] reads literal leaves as content
//! evidence, so collapsing `regex_pattern` into its parent would erase the
//! only thing that tells `/[a-z]+@[a-z]+/i` from `/[0-9]{3}-[0-9]{4}/g`.

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

/// Node floor the fixture is pinned at, chosen so the two outcomes fall on
/// opposite sides of it and nothing else does.
///
/// A correctly collapsed regex is one leaf, so `const name = /.../;` is
/// four nodes — `lexical_declaration`, `variable_declarator`, `__ident__`,
/// `__literal__` — and cannot reach this floor. The inflated form spans
/// five (`__literal__` over `__op__/`, `__literal__`, `__op__/`,
/// `__literal__`), which makes the declaration eight and clears it. A
/// floor of four would put *both* forms above it and the assertion would
/// be measuring the floor rather than the defect: at four nodes "a
/// constant bound to a literal" is a real shared shape, and suppressing it
/// is the min-nodes question, not this one.
const MIN_NODES: &str = "8";

/// The namespace every operator leaf is emitted behind. Neither fixture
/// file computes anything — a declaration, a member access and a call —
/// so a single leaf under this prefix is a framing token that escaped.
const OPERATOR_KIND_PREFIX: &str = "__op__";

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
        assert!(
            !dump.contains(OPERATOR_KIND_PREFIX),
            "{file_name}: nothing in this file computes anything — it declares a \
             constant, reads a member and calls it — so no token may reach the \
             digest as an operator:\n{dump}"
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
