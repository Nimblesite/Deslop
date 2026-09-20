//! [FUSED-CONTENT-GATE-CALL-TARGET] External operations are not local renames.
//! [CLONE-NOISE-LITERAL-VARIATION-CALLS] Chained receiver payloads remain scaffolding.

use anyhow::Result;

use crate::common::*;

const FIXTURE: &str = "js-member-call-targets";
const FILES: [&str; 2] = ["start.js", "cancel.js"];
const NODE_FLOORS: [u32; 2] = [12, 30];
const CONTROL_START: u64 = 8;
const CONTROL_END: u64 = 17;
const CONTROL_SPAN: (u64, u64) = (CONTROL_START, CONTROL_END);
const CLUSTER_COUNT: usize = 1;
const OCCURRENCE_COUNT: usize = 2;
const FIRST_RANK: u64 = 1;
const RANK_FIELD: &str = "rank";

#[test]
fn external_method_changes_are_rejected_but_receiver_renames_publish() -> Result<()> {
    for floor in NODE_FLOORS {
        let report = run_report(&fixture(FIXTURE), floor)?;
        assert_receiver_clone(&report)?;
    }
    Ok(())
}

/// The single control copy publishes completely while unrelated API sequences do not.
fn assert_receiver_clone(report: &serde_json::Value) -> Result<()> {
    assert_eq!(
        clone_findings(report).len(),
        CLUSTER_COUNT,
        "one real copy: {report:#}"
    );
    let clone = expect_cluster_spanning(report, &FILES)?;
    assert_cluster_identity(clone);
    for occurrence in occurrences(clone) {
        assert_eq!(occurrence_line_span(occurrence), CONTROL_SPAN);
        assert!(
            !occurrence_is_hidden(occurrence),
            "receiver rename stays visible"
        );
    }
    Ok(())
}

/// Kind, rank and membership remain meaningful when the noise is rejected.
fn assert_cluster_identity(clone: &serde_json::Value) {
    assert_eq!(cluster_kind(clone), NEARLY_IDENTICAL_KIND);
    assert_eq!(
        clone.get(RANK_FIELD).and_then(serde_json::Value::as_u64),
        Some(FIRST_RANK)
    );
    assert_eq!(occurrences(clone).len(), OCCURRENCE_COUNT);
}
