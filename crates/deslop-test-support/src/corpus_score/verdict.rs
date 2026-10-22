//! [CORPUS-REPORT-VERDICT] Whether a corpus run passed, and every reason it
//! did not.
//!
//! A run fails on a regression, and on nothing else: a judged pair on record
//! as passing that the last engine now answers wrongly, or a curated check
//! failing that `corpus/known-failures.json` does not track — a crashed scan
//! included. Every other failure — a judged pair that has never passed — is
//! listed as a known defect and does not fail the run. The verdict is the first thing a scorecard says,
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
    /// curated check, [`OUTCOMES_PATH`] for a judged pair. A tracked failure
    /// is a known defect — still a bug, but not a regression, so it does not
    /// fail the run.
    pub tracked_in: Option<String>,
    /// True when this failure is a regression: something that passed before
    /// fails now. Only a regression fails the run.
    pub regression: bool,
}

/// Whether a run passed, and why not when it did not.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Verdict {
    /// Every failure, new and known, in target order, check failures before
    /// judged pairs.
    pub failures: Vec<FailureReason>,
}

impl Verdict {
    /// True when nothing regressed: every failure, if any, is a known defect.
    #[must_use]
    pub fn passed(&self) -> bool {
        !self.failures.iter().any(|failure| failure.regression)
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
/// when it is on record as passing, a known defect otherwise — tracked when it
/// is on record as failing.
fn pair_failure(pair: &PairOutcome) -> FailureReason {
    FailureReason {
        repo: pair.key.repo.clone(),
        what: pair.key.defect().to_owned(),
        detail: format!("`{}`: {}", pair.key.occurrences.join("` + `"), pair.note()),
        tracked_in: (pair.standing == Standing::StillFailing).then(|| OUTCOMES_PATH.to_owned()),
        regression: pair.standing.fails_the_run(),
    }
}
