//! [CORPUS-REPORT-VERDICT] Whether a corpus run passed, and every reason it
//! did not.
//!
//! A run fails when any curated check failed — a crashed scan included — or
//! the last engine breached its register gate. The verdict is the first thing
//! a scorecard says, because a failure that has to be found in a table is one
//! a reader skims past.

use serde::Serialize;

use super::{
    gate::Breach,
    render::{Engine, TargetScore},
};

/// One reason a run failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FailureReason {
    /// The repository row it failed on.
    pub repo: String,
    /// The check or gate threshold that failed.
    pub what: String,
    /// What was observed.
    pub detail: String,
    /// Whether `corpus/known-failures.json` already tracks it. A tracked
    /// failure is still a failure; the flag only says it is not new.
    pub tracked: bool,
}

/// Whether a run passed, and why not when it did not.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Verdict {
    /// Every failure, in target order, check failures before gate breaches.
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
/// repository, then every register threshold it breached.
#[must_use]
pub fn verdict(engines: &[Engine], targets: &[TargetScore], breaches: &[Breach]) -> Verdict {
    let Some(last) = engines.last() else {
        return Verdict::default();
    };
    let checks = targets.iter().flat_map(|target| {
        target
            .checks
            .get(&last.id)
            .into_iter()
            .flat_map(|outcome| &outcome.failures)
            .map(|failure| FailureReason {
                repo: target.name.clone(),
                what: failure.check.clone(),
                detail: failure.detail.clone(),
                tracked: failure.known,
            })
    });
    let gate = breaches.iter().map(|breach| FailureReason {
        repo: breach.repo.clone(),
        what: breach.measure.clone(),
        detail: format!("allows {}, recorded {}", breach.allowed, breach.actual),
        tracked: false,
    });
    Verdict {
        failures: checks.chain(gate).collect(),
    }
}
