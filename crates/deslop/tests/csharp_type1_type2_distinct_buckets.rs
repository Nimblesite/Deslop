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

mod common;
use crate::common::*;

fn cluster_interpretations(report: &serde_json::Value) -> Vec<String> {
    report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .map(|clusters| {
            clusters
                .iter()
                .filter_map(|cluster| {
                    cluster
                        .get("interpretation")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

// Issue #42: Type-2 clusters (renamed identifiers, identical structure)
// must NOT claim "every copy is the same" since extraction requires
// parameterisation. Type-1 clusters (byte-identical code) MUST claim
// "every copy is the same". Currently both get the same Identical label.
#[test]
fn type2_clusters_render_distinct_action_from_type1() -> Result<()> {
    let type2_report = run_report(&fixture("csharp-small"), 30)?;
    let type1_report = run_report(&fixture("csharp-type1"), 30)?;

    let type2_interpretations = cluster_interpretations(&type2_report);
    let type1_interpretations = cluster_interpretations(&type1_report);

    assert!(
        !type2_interpretations.is_empty(),
        "csharp-small must produce at least one cluster: {type2_report}",
    );

    // Type-2: renamed-identifier clones must not claim the copies are byte-identical
    for interp in &type2_interpretations {
        assert!(
            !interp.contains("every copy is the same"),
            "Type-2 cluster must not claim copies are identical (issue #42): {interp}",
        );
    }

    // Type-1: at least one cluster is genuinely byte-identical (method-body level)
    assert!(
        type1_interpretations
            .iter()
            .any(|i| i.contains("every copy is the same")),
        "Type-1 fixture must produce at least one genuinely-identical cluster \
         (issue #42): {type1_interpretations:#?}",
    );

    Ok(())
}
