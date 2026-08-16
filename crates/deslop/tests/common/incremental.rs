//! Shared helpers for the incremental-analysis suites
//! ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]): exact `cache_stats`
//! assertions and the strip-and-compare view the equivalence contract
//! is judged by. One copy, used by `incremental_equivalence.rs` and
//! `signature_reuse.rs` — the two suites must agree on what
//! "identical modulo `cache_stats`" means.

use std::collections::BTreeSet;

use serde_json::Value;

use super::field;

/// Asserts the report's exact `cache_stats` — the one member allowed to
/// differ between an incremental and a cold pass.
pub(crate) fn assert_cache_stats(report: &Value, hits: u64, misses: u64, label: &str) {
    let stats = field(report, "cache_stats");
    let actual = (
        field(stats, "hits").as_u64(),
        field(stats, "misses").as_u64(),
    );
    assert_eq!(
        actual,
        (Some(hits), Some(misses)),
        "{label}: cache (hits, misses): {report}"
    );
}

/// The report minus its top-level `cache_stats` member — the exact view
/// the equivalence contract compares. Asserts the member existed, so a
/// schema drift can never make the strip (or its comparison) vacuous.
pub(crate) fn without_cache_stats(report: &Value) -> Value {
    let mut view = report.clone();
    let removed = view
        .as_object_mut()
        .and_then(|members| members.remove("cache_stats"));
    assert!(
        removed.is_some(),
        "report carries no top-level cache_stats member to strip: {report}"
    );
    view
}

/// Top-level members whose values differ between two stripped reports —
/// the first thing an equivalence failure message must name.
fn differing_members(left: &Value, right: &Value) -> Vec<String> {
    let member_names: BTreeSet<String> = [left, right]
        .iter()
        .filter_map(|value| value.as_object())
        .flat_map(|members| members.keys().cloned())
        .collect();
    member_names
        .into_iter()
        .filter(|name| left.get(name) != right.get(name))
        .collect()
}

/// Asserts the incremental report equals the cold report for the same
/// corpus state, field for field, after removing exactly the top-level
/// `cache_stats` member from both sides.
pub(crate) fn assert_reports_equal(incremental: &Value, cold: &Value, scenario: &str) {
    let incremental_view = without_cache_stats(incremental);
    let cold_view = without_cache_stats(cold);
    let diverging = differing_members(&incremental_view, &cold_view);
    assert_eq!(
        incremental_view, cold_view,
        "{scenario}: incremental report diverged from the cold report of the same corpus \
         state in top-level members {diverging:?}; cache_stats is the sole permitted \
         difference ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE])\n\
         incremental: {incremental:#}\ncold: {cold:#}"
    );
}
