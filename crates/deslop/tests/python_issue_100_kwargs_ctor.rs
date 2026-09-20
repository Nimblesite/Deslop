//! E2E regression for GH #100 [CLONE-NOISE-PY-KWARGS-CTOR].
//!
//! ORM / dataclass / Pydantic model constructors with kwargs-only field
//! lists are bounded by the model's required columns. Two constructors
//! sharing the same arity but distinct keyword names cannot share a
//! refactor — extraction would collapse the per-model field contract.
//! The cluster filter must drop those clusters from the rendered report.

use crate::common::*;

const NO_CLONES: usize = 0;
const NO_DUPLICATION: u64 = 0;
const INFORMATIONAL_FINDINGS: usize = 3;
const SUPPRESSED_CONSTRUCTOR_PAIR: u64 = 1;

#[test]
fn message_vs_agentlog_kwargs_constructors_do_not_cluster() -> Result<()> {
    let scan_root = fixture("python-issue-100-kwargs-ctor");
    let report = run_report(&scan_root, 4)?;
    let message_hits = summaries_where(&report, &scan_root, |text| text.contains("Message("))?;
    let agent_log_hits = summaries_where(&report, &scan_root, |text| text.contains("AgentLog("))?;
    assert!(
        message_hits.is_empty(),
        "Message(...) constructor calls must not surface as duplicates: {message_hits:#?}"
    );
    assert!(
        agent_log_hits.is_empty(),
        "AgentLog(...) constructor calls must not surface as duplicates: {agent_log_hits:#?}"
    );
    let visible = clone_findings(&report);
    assert_eq!(
        visible.len(),
        NO_CLONES,
        "the two models share only the mandatory kwargs-constructor \
         scaffolding, so no visible cluster may exist over this fixture at \
         all — nested windows below the constructor call are the same \
         non-duplicate seen narrower, not new clones: {visible:#?}"
    );
    // [CLONE-BUCKETS-STRUCTURAL-ONLY] Keep the measured shapes visible with zero weight.
    assert_eq!(clusters(&report).len(), INFORMATIONAL_FINDINGS);
    assert_eq!(
        clusters_hidden(&report),
        SUPPRESSED_CONSTRUCTOR_PAIR,
        "the constructor noise rule must still identify and suppress the whole pair"
    );
    assert_eq!(
        metric_field(&report, "duplicated_loc").as_u64(),
        Some(NO_DUPLICATION)
    );
    Ok(())
}
