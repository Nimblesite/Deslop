//! [FUSED-CONTENT-GATE-LITERAL-BOUND] When one string is most of an
//! endpoint's text, the pair is a copy only if the strings are near-copies.
//!
//! Three cases, each kept verbatim:
//!
//! * The `DataProvider` repository's `PostgresSchemaInspector.cs` assigns a
//!   different multi-line SQL query to `CommandText` and binds the same two
//!   parameters. The three statements of setup reported as Nearly identical
//!   and charged every line of both queries as duplicated (gh #581). C# raw
//!   strings were not literals at all, so the queries' text was never read
//!   ([LANG-CAND-CSHARP-RAW-STRING]). The queries share 13% of their text.
//! * `BenchmarkDotNet`'s analyzer tests each hold the program under test in
//!   a raw string and differ from their neighbours by one attribute name.
//!   The corpus register judges them CLEARLY IN. Demanding byte-equal
//!   strings dropped three such judged pairs; a near-copy of a string is a
//!   copy.
//! * Osprey's workbench tests hold two Lean kernel exports, each a function
//!   returning one 5 KB string (gh #625). The second is the first with one
//!   universe name changed: 99.96% the same text, and Osprey has since
//!   derived it from the first. It is a real duplicate and stays reported.
//!
//! The control copies one SQL query into a second class under new names: the
//! same text twice must still be reported, alone.

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
const EXPORTS_FILE: &str = "imported.ospml";
/// The two export functions, one line each.
const EXPORT_LINES: [u64; 2] = [5, 7];
const EMBEDDED_SOURCES_FIXTURE: &str = "csharp-embedded-source-tests";
/// The four analyzer tests: `[Fact]` through the closing brace.
const EMBEDDED_SOURCE_TESTS: [(u64, u64); 4] = [(12, 38), (40, 66), (68, 94), (96, 122)];
/// The pair the corpus register judges CLEARLY IN, as method bodies.
const JUDGED_PAIR: [(u64, u64); 2] = [(41, 66), (97, 122)];
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
    assert_eq!(evidence.explanation, LITERAL_BOUND_REFUSAL);
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

/// Every cluster's occurrences as `(start line, end line)`, sorted.
fn line_spans(report: &Value) -> Vec<Vec<(u64, u64)>> {
    clusters(report)
        .iter()
        .map(|cluster| {
            let mut spans: Vec<(u64, u64)> = occurrences(cluster)
                .iter()
                .filter_map(|occurrence| {
                    field(occurrence, "start_line")
                        .as_u64()
                        .zip(field(occurrence, "end_line").as_u64())
                })
                .collect();
            spans.sort_unstable();
            spans
        })
        .collect()
}

/// Tests whose embedded programs differ by one attribute name are copies.
#[test]
fn near_copied_embedded_sources_are_still_duplicates() -> Result<()> {
    let report = run_report(&fixture(EMBEDDED_SOURCES_FIXTURE), MIN_NODES)?;
    assert_eq!(field(&report, "files_analysed").as_u64(), Some(ONE_FILE));
    let spans = line_spans(&report);
    assert!(
        spans.contains(&EMBEDDED_SOURCE_TESTS.to_vec()),
        "the four analyzer tests are one family of near-copies: {spans:?}"
    );
    assert!(
        spans.contains(&JUDGED_PAIR.to_vec()),
        "the judged CLEARLY IN pair is reported at its own extent: {spans:?}"
    );
    for cluster in clusters(&report) {
        assert_eq!(cluster_kind(cluster), NEARLY_IDENTICAL_KIND, "{cluster:#}");
    }
    Ok(())
}

/// Two functions returning strings that differ by one name are copies.
#[test]
fn near_identical_string_exports_are_duplicates() -> Result<()> {
    let root = fixture(EXPORTS_FIXTURE);
    let report = run_report(&root, MIN_NODES)?;
    assert_eq!(field(&report, "files_analysed").as_u64(), Some(ONE_FILE));
    let [exports] = clusters(&report) else {
        anyhow::bail!(
            "exactly the two exports are reported: {:?}",
            reported(&report)
        );
    };
    assert_eq!(cluster_kind(exports), NEARLY_IDENTICAL_KIND);
    assert_eq!(occurrence_files(exports), [EXPORTS_FILE, EXPORTS_FILE]);
    assert_eq!(
        line_spans(&report),
        [EXPORT_LINES.map(|line| (line, line)).to_vec()]
    );
    Ok(())
}
