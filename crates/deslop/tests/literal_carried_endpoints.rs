//! [FUSED-CONTENT-GATE-LITERAL-BOUND] When one string is most of an
//! endpoint's text, the pair is a copy only if the strings are the same.
//!
//! Two reported false positives, kept verbatim:
//!
//! * The `DataProvider` repository's `PostgresSchemaInspector.cs` assigns a different
//!   multi-line SQL query to `CommandText` and binds the same two
//!   parameters. The three statements of setup reported as Nearly identical
//!   and charged every line of both queries as duplicated (gh #581). C# raw
//!   strings were not literals at all, so the queries' text was never read
//!   ([LANG-CAND-CSHARP-RAW-STRING]).
//! * Osprey's workbench tests hold two different Lean kernel exports, each a
//!   function returning one 5 KB string. They reported as Nearly identical
//!   with mass 1,783 (gh #625).
//!
//! The control copies one query into a second class under new names: the
//! same text twice is a real duplicate and must still be reported, alone.

use anyhow::{Context, Result};
use deslop_core::report::PairEndpoint;
use serde_json::Value;

use crate::common::{
    cluster_kind, clusters, field, fixture, occurrence_files, occurrences, run_report, seed,
    signals::compare_endpoints, temp_scan_dir, IDENTICAL_KIND, NEARLY_IDENTICAL_KIND,
};

const MIN_NODES: u32 = 30;
const SQL_FIXTURE: &str = "csharp-sql-command-setup";
const SQL_COPY_FIXTURE: &str = "csharp-sql-command-copy";
const SQL_FILE: &str = "PostgresSchemaInspector.cs";
const SQL_COPY_FILE: &str = "ColumnAudit.cs";
const EXPORTS_FIXTURE: &str = "osprey-literal-exports";
const ONE_FILE: u64 = 1;
const TWO_FILES: u64 = 2;
const PAIR: usize = 2;
/// The statement runs gh #581 compared: from the assignment to the end of
/// the second parameter binding.
const COLUMN_QUERY_START: &str = "colCmd.CommandText";
const COLUMN_QUERY_END: &str = "colCmd.Parameters.AddWithValue(\"@table\", tableName);";
const INDEX_QUERY_START: &str = "idxCmd.CommandText";
const INDEX_QUERY_END: &str = "idxCmd.Parameters.AddWithValue(\"@table\", tableName);";
/// The copied method's first and last lines in both files.
const COPIED_LINES: (u64, u64) = (7, 34);
const LITERAL_BOUND_REFUSAL: &str =
    "rejected: one string literal carries an endpoint and the texts differ";

/// Every cluster as its file names, for failure messages.
fn reported(report: &Value) -> Vec<Vec<String>> {
    clusters(report).iter().map(occurrence_files).collect()
}

/// Asserts the scan analysed `files` and reported nothing.
fn assert_nothing_reported(report: &Value, files: u64, what: &str) {
    assert_eq!(field(report, "files_analysed").as_u64(), Some(files));
    assert!(
        clusters(report).is_empty(),
        "{what} share no text and must not be reported: {:?}",
        reported(report)
    );
    assert_eq!(
        field(field(report, "metrics"), "duplicated_loc").as_u64(),
        Some(0),
        "{what}: no line of either string is duplicated"
    );
}

/// The byte span in `text` from `start` to the end of `end`.
fn span(text: &str, start: &str, end: &str) -> Result<(usize, usize)> {
    let from = text.find(start).with_context(|| format!("no {start}"))?;
    let to = text.find(end).with_context(|| format!("no {end}"))?;
    Ok((from, to.saturating_add(end.len())))
}

/// Two different SQL queries behind the same three lines of setup.
#[test]
fn different_sql_queries_are_not_duplicates_of_each_other() -> Result<()> {
    let root = fixture(SQL_FIXTURE);
    let report = run_report(&root, MIN_NODES)?;
    assert_nothing_reported(&report, ONE_FILE, "the column and index queries");

    let path = root.join(SQL_FILE);
    let text = std::fs::read_to_string(&path)?;
    let [left, right] = [
        span(&text, COLUMN_QUERY_START, COLUMN_QUERY_END)?,
        span(&text, INDEX_QUERY_START, INDEX_QUERY_END)?,
    ]
    .map(|(start_byte, end_byte)| PairEndpoint {
        path: path.clone(),
        start_byte,
        end_byte,
    });
    let evidence = compare_endpoints(&root, MIN_NODES, left, right)?.evidence;
    assert!(!evidence.admitted, "gh #581 pair: {evidence:#?}");
    assert_eq!(evidence.classification, None);
    assert!(
        evidence.agreement < 1.0,
        "the two queries are different text, and the measurement must see it: {evidence:#?}"
    );
    assert!(evidence.explanation.starts_with("rejected: "));
    Ok(())
}

/// The same query copied under new names is reported, without the other query.
#[test]
fn a_copied_sql_query_is_still_reported_alone() -> Result<()> {
    let (_tmp, root) = temp_scan_dir("src")?;
    seed(&fixture(SQL_FIXTURE), &root)?;
    seed(&fixture(SQL_COPY_FIXTURE), &root)?;
    let report = run_report(&root, MIN_NODES)?;
    assert_eq!(field(&report, "files_analysed").as_u64(), Some(TWO_FILES));
    let [copy] = clusters(&report) else {
        anyhow::bail!(
            "exactly the copied query is reported: {:?}",
            reported(&report)
        );
    };
    assert!([IDENTICAL_KIND, NEARLY_IDENTICAL_KIND].contains(&cluster_kind(copy)));
    assert_eq!(occurrences(copy).len(), PAIR);
    let mut files = occurrence_files(copy);
    files.sort();
    assert_eq!(files, [SQL_COPY_FILE, SQL_FILE]);
    for occurrence in occurrences(copy) {
        let lines = (
            field(occurrence, "start_line").as_u64(),
            field(occurrence, "end_line").as_u64(),
        );
        assert_eq!(lines, (Some(COPIED_LINES.0), Some(COPIED_LINES.1)));
    }
    Ok(())
}

/// Two functions that each return a different 5 KB string.
#[test]
fn different_string_exports_are_not_duplicates_of_each_other() -> Result<()> {
    let report = run_report(&fixture(EXPORTS_FIXTURE), MIN_NODES)?;
    assert_nothing_reported(&report, ONE_FILE, "the two Lean exports");
    Ok(())
}

/// The refusal names its reason when nothing else would have refused the pair.
#[test]
fn explicit_comparison_of_the_exports_names_the_literal_rule() -> Result<()> {
    let root = fixture(EXPORTS_FIXTURE);
    let path = root.join("imported.ospml");
    let text = std::fs::read_to_string(&path)?;
    let [left, right] = ["counterfeitEquality ()", "counterfeitEqualityRenamed ()"].map(|name| {
        let start_byte = text.find(name).unwrap_or(text.len());
        let end_byte = text
            .get(start_byte..)
            .and_then(|rest| rest.find('\n'))
            .map_or(text.len(), |offset| start_byte.saturating_add(offset));
        PairEndpoint {
            path: path.clone(),
            start_byte,
            end_byte,
        }
    });
    let evidence = compare_endpoints(&root, MIN_NODES, left, right)?.evidence;
    assert!(!evidence.admitted, "gh #625 pair: {evidence:#?}");
    assert_eq!(evidence.explanation, LITERAL_BOUND_REFUSAL);
    Ok(())
}
