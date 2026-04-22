//! Agent action-hint playbook for reports.

use serde::{Deserialize, Serialize};

use crate::buckets::{bucket_labels, ClusterKind};

/// Short playbook entry surfaced at the top of every report so agents
/// can decide how to act before walking the cluster list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionHint {
    /// Matches one of the taxonomy rows in the report context
    /// (`type-1-2`, `type-3`, `fused-family`, `lsh-only-weak`).
    pub pattern: String,
    /// One-line recommendation written for an agent reader.
    pub recommendation: String,
}

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
