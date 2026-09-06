//! E2E regression for GH #100 [CLONE-NOISE-PY-KWARGS-CTOR].
//!
//! ORM / dataclass / Pydantic model constructors with kwargs-only field
//! lists are bounded by the model's required columns. Two constructors
//! sharing the same arity but distinct keyword names cannot share a
//! refactor — extraction would collapse the per-model field contract.
//! The cluster filter must drop those clusters from the rendered report.

mod common;

use crate::common::*;

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
    Ok(())
}
