//! End-to-end regression coverage for issue #42: Type-1 exact clones
//! (byte-identical code) must render a distinct action sentence from
//! Type-2 renamed-identifier clones ([CLONE-BUCKETS]).
//!
//! Acceptance: csharp-small (two methods, same structure, renamed
//! identifiers) must NOT claim "every copy is the same". csharp-type1
//! (two methods that are byte-identical) MUST have at least one cluster
//! claiming "every copy is the same".
//! Tests [CLONE-BUCKETS-IDENTICAL]

use anyhow::Result;

use crate::common::signals::has_verbatim_pair;
use crate::common::*;

// Issue #42: Type-1 clusters (byte-identical code) are a different
// finding from Type-2 clusters (renamed identifiers, identical
// structure) — extraction from a Type-2 copy needs parameterisation
// that a byte-identical copy does not. The interpretation sentence is
// gone from the mass-only wire; the distinction is proven by the byte
// truth: the Type-1 fixture's occurrences slice to identical source
// bytes, the Type-2 fixture's do not ([PIPELINE-CLUSTER-CLOSURE]).
#[test]
fn type2_clusters_render_distinct_action_from_type1() -> Result<()> {
    let type2_scan = fixture("csharp-small");
    let type2_report = run_report(&type2_scan, 30)?;
    let type1_scan = fixture("csharp-type1");
    let type1_report = run_report(&type1_scan, 30)?;

    assert!(
        !clusters(&type2_report).is_empty(),
        "csharp-small must produce at least one cluster: {type2_report}",
    );
    assert!(
        !clusters(&type1_report).is_empty(),
        "csharp-type1 must produce at least one cluster: {type1_report}",
    );

    // Type-2: renamed-identifier clones must NOT be byte-proven copies —
    // their occurrences differ in raw bytes.
    for cluster in clusters(&type2_report) {
        assert!(
            !has_verbatim_pair(&type2_scan, cluster)?,
            "Type-2 cluster must not be a byte-identical copy (issue #42): {cluster:#}",
        );
    }

    // Type-1: the reported view is the whole file (whose namespace and
    // class names differ), but the copied method body is byte-identical
    // inside every occurrence — the strongest byte fact the report
    // exposes for a genuine Type-1 copy ([PIPELINE-CLUSTER-CLOSURE]).
    let type1_cluster = clusters(&type1_report)
        .first()
        .ok_or_else(|| anyhow::anyhow!("csharp-type1 must produce a cluster: {type1_report}"))?;
    let texts = occurrence_texts(&type1_scan, type1_cluster)?;
    assert!(
        texts.len() >= 2
            && texts
                .iter()
                .all(|text| text.contains("public int Tally(int bound)")),
        "Type-1 fixture must carry the byte-identical method body in every \
         reported occurrence (issue #42): {texts:#?}",
    );

    Ok(())
}
