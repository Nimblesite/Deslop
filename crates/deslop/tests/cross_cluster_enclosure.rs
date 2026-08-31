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

use crate::common::signals::{assert_no_pair_surface_on_cluster, has_verbatim_pair};
use crate::common::verdict::duplicated_loc_for_path;
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
    assert_no_pair_surface_on_cluster(cluster, "enclosure");
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

// [PIPELINE-CLUSTER-SUBSUME] may not erase an admitted member by keeping
// a narrower view. `ledger_b` is deliberately excluded: it has matching
// normalized shape but no raw-content support, so [FUSED-CONTENT-GATE]
// rejects its pairs before closure. The other four files must remain in
// the wider published view.
#[test]
fn subsumption_keeps_every_admitted_file_without_reviving_a_shape_only_rewrite() -> Result<()> {
    let report = run_report(&fixture("ts-mixed-band"), 12)?;
    let expected = std::collections::BTreeSet::from([
        "ledger_a.ts".to_owned(),
        "ledger_c.ts".to_owned(),
        "ledger_d.ts".to_owned(),
        "ledger_e.ts".to_owned(),
    ]);
    let admitted = expect_cluster_spanning(
        &report,
        &["ledger_a.ts", "ledger_c.ts", "ledger_d.ts", "ledger_e.ts"],
    )?;
    assert_eq!(
        cluster_file_set(admitted),
        expected,
        "the wider admitted view must retain every admitted file: {report:#}"
    );
    let named: std::collections::BTreeSet<String> = clusters(&report)
        .iter()
        .flat_map(occurrences)
        .filter_map(|occurrence| Some(occurrence.get("path")?.as_str()?.to_owned()))
        .filter_map(|path| path.rsplit('/').next().map(str::to_owned))
        .collect();
    assert_eq!(
        named,
        std::collections::BTreeSet::from([
            "ledger_a.ts".to_owned(),
            "ledger_c.ts".to_owned(),
            "ledger_d.ts".to_owned(),
            "ledger_e.ts".to_owned(),
        ]),
        "the report must publish precisely the admitted files: {report:#}"
    );
    assert_eq!(
        duplicated_loc_for_path(&report, "ledger_b.ts")?,
        0,
        "the shape-only rewrite must be rejected before closure: {report:#}"
    );
    assert_no_pair_surface_on_cluster(admitted, "ts-mixed-band admitted view");
    Ok(())
}

// A corpus staging several degrees of duplication keeps every admitted
// non-redundant view. `ledger_d`/`ledger_e` are near copies of
// `ledger_a`, `ledger_c` is a renamed copy, and `ledger_b` is a shape-only
// rewrite rejected before closure.
#[test]
fn ts_mixed_band_keeps_nonredundant_admitted_views() -> Result<()> {
    let report = run_report(&fixture("ts-mixed-band"), 12)?;
    let published = clusters(&report);
    assert!(
        published.len() >= 2,
        "one visible cluster cannot express three degrees of duplication: \
         {report:#}"
    );
    // [PIPELINE-CLUSTER-CLOSURE] The byte-proven family must reach the
    // report, and every cluster carries a clean cluster-only surface.
    for cluster in published {
        assert_no_pair_surface_on_cluster(cluster, "ts-mixed-band");
    }
    assert!(
        published
            .iter()
            .any(|cluster| has_verbatim_pair(&fixture("ts-mixed-band"), cluster).unwrap_or(false)),
        "the byte-proven family must reach the report: {report:#}"
    );
    Ok(())
}

// [PIPELINE-CLUSTER-SUBSUME] chooses the enclosing Type-2 class view over
// its nested byte-identical method. The survivor must still name both files,
// carry no pair evidence, and leave no hidden residual view.
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
        !has_verbatim_pair(&fixture("csharp-enclosing-method-clone"), cluster)?,
        "the enclosing class view differs by class name, so a nested method must not elect itself: {report:#}"
    );
    let names: std::collections::BTreeSet<String> = occurrences(cluster)
        .iter()
        .filter_map(|occurrence| occurrence.get("path")?.as_str().map(str::to_owned))
        .collect();
    assert_eq!(
        names,
        std::collections::BTreeSet::from(["Alpha.cs".to_owned(), "Beta.cs".to_owned()]),
        "the enclosing survivor must preserve both duplicated files"
    );
    assert_no_pair_surface_on_cluster(cluster, "enclosure");
    assert_eq!(
        clusters_hidden(&report),
        0,
        "no part of a byte-identical whole-method clone may be routed to the \
         hidden bucket: {report:#}"
    );
    Ok(())
}
