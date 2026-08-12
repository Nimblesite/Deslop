//! Black-box controls for [PIPELINE-CLUSTER-EXACT] cross-cluster
//! subsumption: which of two views of one physical duplication survives.
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

mod common;
use crate::common::*;

/// One rendered occurrence reduced to the fields enclosure depends on.
#[derive(Clone, Debug)]
struct Span {
    path: String,
    start: u64,
    end: u64,
    start_line: u64,
    end_line: u64,
}

fn spans(cluster: &serde_json::Value) -> Vec<Span> {
    occurrences(cluster)
        .iter()
        .filter_map(|occurrence| {
            Some(Span {
                path: occurrence.get("path")?.as_str()?.to_owned(),
                start: occurrence.get("start_byte")?.as_u64()?,
                end: occurrence.get("end_byte")?.as_u64()?,
                start_line: occurrence.get("start_line")?.as_u64()?,
                end_line: occurrence.get("end_line")?.as_u64()?,
            })
        })
        .collect()
}

/// True when `inner` lies wholly inside `outer` in the same file.
fn contains(outer: &Span, inner: &Span) -> bool {
    outer.path == inner.path && outer.start <= inner.start && inner.end <= outer.end
}

/// True when every span of `nested` sits inside some span of `enclosing`
/// and the two sets are not identical — the redundant-view relation.
fn strictly_encloses(enclosing: &[Span], nested: &[Span]) -> bool {
    if enclosing.is_empty() || nested.is_empty() {
        return false;
    }
    let every_nested_inside = nested
        .iter()
        .all(|candidate| enclosing.iter().any(|outer| contains(outer, candidate)));
    let some_enclosing_outside = enclosing
        .iter()
        .any(|outer| !nested.iter().any(|inner| contains(inner, outer)));
    every_nested_inside && some_enclosing_outside
}

/// Returns a description of the first pair of published clusters where
/// one is a nested re-description of the other.
fn first_nested_view(report: &serde_json::Value) -> Option<String> {
    let sets: Vec<(String, Vec<Span>)> = clusters(report)
        .iter()
        .map(|cluster| (cluster_id(cluster).to_owned(), spans(cluster)))
        .collect();
    sets.iter().enumerate().find_map(|(index, (left_id, left))| {
        sets.iter().skip(index.saturating_add(1)).find_map(
            |(right_id, right)| match (
                strictly_encloses(left, right),
                strictly_encloses(right, left),
            ) {
                (true, _) => Some(format!(
                    "cluster {right_id} is nested inside {left_id}; only the \
                     enclosing view may be published"
                )),
                (_, true) => Some(format!(
                    "cluster {left_id} is nested inside {right_id}; only the \
                     enclosing view may be published"
                )),
                _ => None,
            },
        )
    })
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
    for member in &members {
        let line_count = member.end_line.saturating_sub(member.start_line);
        assert!(
            line_count >= 55,
            "occurrence in {path} spans {line_count} lines; the duplicated \
             method body is 60 statements, so a handful of lines means the \
             nested view replaced it: {report:#}",
            path = member.path,
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
