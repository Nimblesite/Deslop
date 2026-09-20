//! [CLONE-BUCKETS-STRUCTURAL-ONLY] Separate displayed information from duplication.

use serde_json::Value;

use super::{
    clusters, field, IDENTICAL_KIND, LOOSELY_SIMILAR_KIND, NEARLY_IDENTICAL_KIND,
    SAME_BEHAVIOR_KIND, STRUCTURAL_ONLY_KIND,
};

const KIND_FIELD: &str = "kind";
const MASS_FIELD: &str = "mass";
const RANK_FIELD: &str = "rank";
/// [CLONE-KIND-LABELS] Every kind that claims duplication, in display order.
const CLONE_KINDS: [&str; 4] = [
    IDENTICAL_KIND,
    NEARLY_IDENTICAL_KIND,
    SAME_BEHAVIOR_KIND,
    LOOSELY_SIMILAR_KIND,
];
const NO_DUPLICATION_WEIGHT: u64 = 0;

/// Keep real clones while checking that informational matches claim no duplication.
pub(crate) fn clone_findings(report: &Value) -> Vec<Value> {
    clusters(report)
        .iter()
        .filter(|finding| is_clone_finding(finding))
        .cloned()
        .collect()
}

/// Validate the category before using a finding in a duplication assertion.
pub(crate) fn is_clone_finding(finding: &Value) -> bool {
    let kind = field(finding, KIND_FIELD).as_str().unwrap_or_default();
    if kind == STRUCTURAL_ONLY_KIND {
        assert_informational_weight(finding);
        return false;
    }
    assert!(
        CLONE_KINDS.contains(&kind),
        "unknown finding category: {kind}"
    );
    true
}

/// [CLONE-BUCKETS-DUAL-LABEL] Informational records claim no duplicate quantity.
fn assert_informational_weight(finding: &Value) {
    for name in [MASS_FIELD, RANK_FIELD] {
        assert_eq!(
            field(finding, name).as_u64(),
            Some(NO_DUPLICATION_WEIGHT),
            "[CLONE-BUCKETS-STRUCTURAL-ONLY] informational {name} must be zero: {finding:#}"
        );
    }
}
