//! [CORPUS-REPORT-VERDICT] Whether a corpus run passed, and every reason it
//! did not.
//!
//! A run fails when any curated check failed — a crashed scan included — or
//! the last engine answered any judged pair wrongly. The verdict is the first
//! thing a scorecard says, because a failure that has to be found in a table is
//! one a reader skims past.

use serde::Serialize;

use super::{
    outcomes::{Outcome, PairOutcome, Standing, OUTCOMES_PATH},
    render::{Engine, TargetScore},
};
use crate::corpus::KNOWN_FAILURES_PATH;

/// One reason a run failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FailureReason {
    /// The repository row it failed on.
    pub repo: String,
    /// The check or gate threshold that failed.
    pub what: String,
    /// What was observed.
    pub detail: String,
    /// The record that already tracks it: [`KNOWN_FAILURES_PATH`] for a
    /// curated check, [`OUTCOMES_PATH`] for a judged pair. A tracked failure
    /// is still a failure; the record only says it is not new.
    pub tracked_in: Option<String>,
}

/// Whether a run passed, and why not when it did not.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Verdict {
    /// Every failure, in target order, check failures before judged pairs.
    pub failures: Vec<FailureReason>,
}

impl Verdict {
    /// True when nothing failed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

/// The verdict on the last engine: every curated check it failed on every
/// repository, then every judged pair it answered wrongly.
#[must_use]
pub fn verdict(engines: &[Engine], targets: &[TargetScore], pairs: &[PairOutcome]) -> Verdict {
    let Some(last) = engines.last() else {
        return Verdict::default();
    };
    let judged = pairs
        .iter()
        .filter(|pair| pair.outcome == Outcome::Fail)
        .map(pair_failure);
    Verdict {
        failures: check_failures(targets, &last.id).chain(judged).collect(),
    }
}

/// Every curated check `engine_id` failed, repository by repository.
fn check_failures<'a>(
    targets: &'a [TargetScore],
    engine_id: &'a str,
) -> impl Iterator<Item = FailureReason> + 'a {
    targets.iter().flat_map(move |target| {
        target
            .checks
            .get(engine_id)
            .into_iter()
            .flat_map(|outcome| &outcome.failures)
            .map(|failure| FailureReason {
                repo: target.name.clone(),
                what: failure.check.clone(),
                detail: failure.detail.clone(),
                tracked_in: failure.known.then(|| KNOWN_FAILURES_PATH.to_owned()),
            })
    })
}

/// [CORPUS-SCORE-OUTCOMES] One judged pair answered wrongly: tracked when it
/// is on record as failing, new when it regressed or was never recorded.
fn pair_failure(pair: &PairOutcome) -> FailureReason {
    FailureReason {
        repo: pair.key.repo.clone(),
        what: pair.key.defect().to_owned(),
        detail: format!("`{}`: {}", pair.key.occurrences.join("` + `"), pair.note()),
        tracked_in: (pair.standing == Standing::StillFailing).then(|| OUTCOMES_PATH.to_owned()),
    }
}
