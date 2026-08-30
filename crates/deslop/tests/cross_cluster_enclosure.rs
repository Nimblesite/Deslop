//! Black-box controls for [PIPELINE-CLUSTER-SUBSUME]: which of two
//! views of one physical duplication survives.
//!
//! Overlap alone cannot answer that. A whole-method clone and the run of
//! single-statement clones nested inside it cover the same bytes in both
//! directions, so an overlap-only rule keeps whichever ranked heavier —
//! and the fine-grained view always ranks heavier, because it carries one
//! occurrence per statement. The 60-statement clone below then rendered as
//! 120 one-line occurrences of `var valueN = N + N;` while the duplicated
//! method itself vanished: the only extractable duplicate in the corpus,
//! reported as unactionable line noise.
//!
//! The contract these tests pin is physical enclosure — when every
//! occurrence of one cluster sits inside an occurrence of another, the
//! enclosing view is the duplication and the nested view is a redundant
//! re-description of it.

use anyhow::Result;
use deslop_test_support::enclosure::{first_nested_view as first_nested, spans_of as spans, Span};

use crate::common::*;

/// The published clusters as `(id, spans)`, the shape the shared
/// enclosure predicate consumes.
fn cluster_spans(report: &serde_json::Value) -> Vec<(String, Vec<Span>)> {
    clusters(report)
        .iter()
        .map(|cluster| (cluster_id(cluster).to_owned(), spans(cluster)))
        .collect()
}

/// Returns a description of the first pair of published clusters where
/// one is a nested re-description of the other.
fn first_nested_view(report: &serde_json::Value) -> Option<String> {
    first_nested(&cluster_spans(report))
}

/// The line count an occurrence covers, from the rendered line numbers.
fn line_span(occurrence: &serde_json::Value) -> u64 {
    let start = occurrence
        .get("start_line")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default();
    occurrence
        .get("end_line")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default()
        .saturating_sub(start)
}

fn enclosing_clone_report() -> Result<serde_json::Value> {
    run_report(&fixture("csharp-enclosing-method-clone"), 8)
}

// The duplicated 60-statement `Run()` method must be reported as the
// duplicate it is: one cluster, one occurrence per file, each spanning
// the whole method. The nested per-statement view covers the same bytes
// but is not extractable — it must not replace the method-level clone.
#[test]
fn a_whole_method_clone_outranks_its_nested_statement_view() -> Result<()> {
    let report = enclosing_clone_report()?;
    let cluster = expect_cluster_spanning(&report, &["Alpha.cs", "Beta.cs"])?;
    let members = spans(cluster);
    assert_eq!(
        members.len(),
        2,
        "the duplicated method must render as one occurrence per file, not one \
         per nested statement: {report:#}"
    );
    for member in occurrences(cluster) {
        let line_count = line_span(member);
        assert!(
            line_count >= 55,
            "occurrence spans {line_count} lines; the duplicated method body is \
             60 statements, so a handful of lines means the nested view \
             replaced it: {report:#}"
        );
    }
    assert_eq!(
        cluster_bucket(cluster),
        "identical",
        "byte-identical method bodies must bucket as identical: {report:#}"
    );
    Ok(())
}

// Subsumption must not publish a cluster and a nested re-description of
// it. Asserted over both the enclosure corpus and the nested-fingerprint
// corpus that motivated the pass, so neither shape can regress alone.
#[test]
fn no_published_cluster_is_a_nested_view_of_another() -> Result<()> {
    for fixture_name in ["csharp-enclosing-method-clone", "csharp-fact-cross-cluster"] {
        let report = run_report(&fixture(fixture_name), 8)?;
        assert!(
            first_nested_view(&report).is_none(),
            "{fixture_name}: {detail}",
            detail = first_nested_view(&report).unwrap_or_default(),
        );
    }
    Ok(())
}

// The false-negative guard on the enclosure rule. A view may only be
// subsumed by one that names every file it names. Otherwise the files
// only it mentioned lose their duplication outright: no other cluster
// reports them, so the finding does not move — it disappears. Enclosure
// makes this easy to get wrong, because the enclosing view can be the
// narrower one. `ledger_e` is the witness: it belongs to the wide
// four-file family and to no other published cluster, so a subsumption
// pass that keeps only the three-file view erases it.
#[test]
fn subsumption_never_drops_a_view_naming_a_file_the_survivor_omits() -> Result<()> {
    let report = run_report(&fixture("ts-mixed-band"), 12)?;
    let named: std::collections::BTreeSet<String> = clusters(&report)
        .iter()
        .flat_map(occurrences)
        .filter_map(|occurrence| Some(occurrence.get("path")?.as_str()?.to_owned()))
        .filter_map(|path| path.rsplit('/').next().map(str::to_owned))
        .collect();
    for witness in ["ledger_a.ts", "ledger_b.ts", "ledger_d.ts", "ledger_e.ts"] {
        assert!(
            named.contains(witness),
            "{witness} is duplicated and named by no other published cluster, \
             so subsumption erased its finding; published files: {named:?}: \
             {report:#}"
        );
    }
    Ok(())
}

// A corpus staging several degrees of duplication cannot render one
// verdict for all of them. `ts-mixed-band` holds a near copy
// (`ledger_d`/`ledger_e`, one parenthesis apart from `ledger_a`), a
// renamed copy (`ledger_c`), and a same-shape family whose identifiers
// *and* literals all differ (`ledger_b`). The report must separate a
// one-token edit from a wholesale rewrite by routing them to distinct
// buckets with distinct evidence verdicts — never one label for all.
#[test]
fn ts_mixed_band_renders_a_distinct_confidence_per_band() -> Result<()> {
    let report = run_report(&fixture("ts-mixed-band"), 12)?;
    let published = clusters(&report);
    assert!(
        published.len() >= 2,
        "one visible cluster cannot express three degrees of duplication: \
         {report:#}"
    );
    let buckets: std::collections::BTreeSet<String> = published
        .iter()
        .map(|cluster| cluster_bucket(cluster).to_owned())
        .collect();
    assert!(
        buckets.len() >= 2,
        "three degrees of duplication cannot all render one bucket, got \
         {buckets:?}: {report:#}"
    );
    let verdicts: std::collections::BTreeSet<String> = published
        .iter()
        .map(|cluster| {
            field(cluster, "evidence_verdict")
                .as_str()
                .unwrap_or_default()
                .to_owned()
        })
        .collect();
    assert!(
        verdicts.len() >= 2,
        "three degrees of duplication cannot all render one evidence verdict, \
         got {verdicts:?}: {report:#}"
    );
    assert!(
        published
            .iter()
            .any(|cluster| approx(signal(cluster, "structural"), 1.0)),
        "the byte-proven family must reach the report: {report:#}"
    );
    Ok(())
}

// Control against over-collapsing: the surviving cluster must still name
// both files. A subsumption pass that erased one side of the pair would
// satisfy the enclosure invariant above while destroying the finding.
#[test]
fn enclosure_collapse_preserves_every_duplicated_file() -> Result<()> {
    let report = enclosing_clone_report()?;
    assert_eq!(
        cluster_count(&report),
        1,
        "the corpus holds exactly one duplicated region: {report:#}"
    );
    let cluster = expect_cluster_spanning(&report, &["Alpha.cs", "Beta.cs"])?;
    assert!(
        approx(signal(cluster, "structural"), 1.0),
        "byte-identical methods must measure structural 1.0: {report:#}"
    );
    assert_eq!(
        clusters_hidden(&report),
        0,
        "no part of a byte-identical whole-method clone may be routed to the \
         hidden bucket: {report:#}"
    );
    Ok(())
}
