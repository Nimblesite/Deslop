//! [CORPUS-REPORT-VERDICT] Whether a corpus run passed, and every reason it
//! did not.
//!
//! A run fails on an unaccepted incorrect judgment, a regression, or any failed
//! curated check, including tracked checks and crashed scans. Only a judged
//! pair explicitly on record as failing is tolerated. The verdict is the first thing a scorecard says,
//! because a failure that has to be found in a table is one a reader skims
//! past.

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
    /// curated check, [`OUTCOMES_PATH`] for an explicitly accepted judged pair.
    /// Tracking a curated check does not exempt it from failing the verdict.
    pub tracked_in: Option<String>,
    /// True when this failure is a regression: something that passed before
    /// fails now. Other unaccepted failures also fail the run.
    pub regression: bool,
}

impl FailureReason {
    /// [CORPUS-REPORT-VERDICT] Only an explicitly accepted judged failure is tolerated.
    #[must_use]
    pub fn fails_the_run(&self) -> bool {
        self.regression || self.tracked_in.as_deref() != Some(OUTCOMES_PATH)
    }
}

/// Whether a run passed, and why not when it did not.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Verdict {
    /// Every failure, new and known, in target order, check failures before
    /// judged pairs.
    pub failures: Vec<FailureReason>,
}

impl Verdict {
    /// True when every failure is an explicitly accepted judged defect.
    #[must_use]
    pub fn passed(&self) -> bool {
        !self.failures.iter().any(FailureReason::fails_the_run)
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
                regression: !failure.known,
            })
    })
}

/// [CORPUS-SCORE-OUTCOMES] One judged pair answered wrongly: a regression
/// when it is on record as passing, an unaccepted failure when unrecorded,
/// and a tolerated defect only when explicitly on record as failing.
fn pair_failure(pair: &PairOutcome) -> FailureReason {
    FailureReason {
        repo: pair.key.repo.clone(),
        what: pair.key.defect().to_owned(),
        detail: format!("`{}`: {}", pair.key.occurrences.join("` + `"), pair.note()),
        tracked_in: (pair.standing == Standing::StillFailing).then(|| OUTCOMES_PATH.to_owned()),
        regression: pair.standing == Standing::Regressed,
    }
}
