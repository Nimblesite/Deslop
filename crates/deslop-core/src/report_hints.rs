//! Agent action-hint playbook for reports.

use crate::{
    buckets::{bucket_labels, ClusterKind},
    clone_category::CloneCategory,
};

// `ActionHint` is generated from `docs/models/live-ipc.td` by
// `scripts/typediagram/generate.mjs`. The data shape lives in
// `crate::wire_generated`; the playbook constructor below stays here.
pub use crate::wire_generated::ActionHint;

/// Playbook shown to agents. One entry per bucket in [CLONE-BUCKETS], plus a
/// `category=data` entry pointing data-table clusters at the builder / asset
/// remedy instead of "extract the duplicate" ([RANK-CATEGORY]).
#[must_use]
pub fn default_action_hints() -> Vec<ActionHint> {
    let mut hints = Vec::with_capacity(ClusterKind::all().len().saturating_add(1));
    for kind in ClusterKind::all() {
        let labels = bucket_labels(kind);
        hints.push(ActionHint {
            pattern: format!("bucket={}", labels.css_suffix),
            recommendation: labels.agent_summary(),
        });
    }
    hints.push(ActionHint {
        pattern: format!("category={}", CloneCategory::DataTable.wire_label()),
        recommendation: CloneCategory::DataTable.action_sentence().to_owned(),
    });
    hints
}
