//! [CORPUS-REPORT-ENTRY] How one judged pair is written to the JSON
//! scorecard: the pair, pass or fail, and — only on a fail — the one thing
//! that failed. A judged pair is either answered exactly or it fails; the
//! scorer's working (cluster, coverage, judge's reason) stays in memory for
//! the markdown scorecard and never pads the record.

use serde::Serialize;

use super::{outcomes::Outcome, ScoredEntry, CLEARLY_IN};

/// What failed when a CLEARLY IN pair was published nowhere.
const NOT_REPORTED: &str = "not reported";

/// One judged pair as the JSON scorecard writes it.
#[derive(Serialize)]
pub(super) struct WrittenEntry {
    /// `clearly_in` or `clearly_out`.
    verdict: String,
    /// The ranges as the register wrote them.
    occurrences: Vec<String>,
    /// Pass or fail.
    outcome: Outcome,
    /// What failed, absent on a pass.
    #[serde(skip_serializing_if = "Option::is_none")]
    failed: Option<String>,
}

impl From<ScoredEntry> for WrittenEntry {
    fn from(entry: ScoredEntry) -> Self {
        let failed = (!entry.correct).then(|| what_failed(&entry));
        Self {
            outcome: Outcome::of(&entry),
            failed,
            verdict: entry.verdict,
            occurrences: entry.occurrences,
        }
    }
}

/// The one thing a wrongly answered pair got wrong.
pub(super) fn what_failed(entry: &ScoredEntry) -> String {
    if entry.verdict != CLEARLY_IN {
        return format!(
            "reported in cluster {}",
            entry.cluster.as_deref().unwrap_or_default()
        );
    }
    match (&entry.reported_as, &entry.clone_type) {
        (Some(kind), Some(judged)) => format!("reported as {kind}, judged {judged}"),
        _ => NOT_REPORTED.to_owned(),
    }
}
