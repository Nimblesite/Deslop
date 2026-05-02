//! Agent action-hint playbook for reports.

use crate::buckets::{bucket_labels, ClusterKind};

// `ActionHint` is generated from `docs/models/live-ipc.td` by
// `scripts/typediagram-gen.mjs`. The data shape lives in
// `crate::wire_generated`; the playbook constructor below stays here.
pub use crate::wire_generated::ActionHint;

/// Playbook shown to agents. One entry per bucket in [CLONE-BUCKETS].
#[must_use]
pub fn default_action_hints() -> Vec<ActionHint> {
    let mut hints = Vec::with_capacity(ClusterKind::all().len());
    for kind in ClusterKind::all() {
        let labels = bucket_labels(kind);
        hints.push(ActionHint {
            pattern: format!("bucket={}", labels.css_suffix),
            recommendation: labels.agent_summary(),
        });
    }
    hints
}
