//! [CORPUS-REPORT-CHECKS] The curated corpus checks, as the scorecard records
//! them.
//!
//! A clone register is one kind of accuracy evidence; the curated checks in
//! `corpus/<name>.json` — hand-verified duplicates that must be found, ranking
//! rules, scope floors, the resource ceilings — are the other, and they are
//! what `make test-corpus` asserts. They are recorded beside the register
//! score so every corpus run reports its accuracy in the same document, and a
//! repository with no register still states what was checked and what failed.

use serde::{Deserialize, Serialize};

/// Every curated check one corpus test evaluated, and the ones that failed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckOutcome {
    /// Every check id the test ran, in the order it ran them.
    pub evaluated: Vec<String>,
    /// Whether the manifest curates any accuracy claim at all. False means the
    /// run checked scope and resource ceilings only, so a pass there says
    /// nothing about whether detection on the repository is correct.
    pub accuracy_curated: bool,
    /// Every failure observed, whether or not it is already tracked.
    pub failures: Vec<CheckFailure>,
}

/// One failed curated check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckFailure {
    /// The check id, e.g. `recall` or `memory`.
    pub check: String,
    /// What failed, in the test's own words.
    pub detail: String,
    /// Whether `corpus/known-failures.json` already tracks this check for the
    /// repository. A known failure is still a failure; the flag separates a
    /// regression from a defect that was already there.
    pub known: bool,
}

impl CheckOutcome {
    /// Evaluated checks that produced no failure.
    #[must_use]
    pub fn passed(&self) -> usize {
        self.evaluated
            .iter()
            .filter(|check| !self.failures.iter().any(|failure| &failure.check == *check))
            .count()
    }

    /// Failures `corpus/known-failures.json` does not track.
    #[must_use]
    pub fn new_failures(&self) -> usize {
        self.failures
            .iter()
            .filter(|failure| !failure.known)
            .count()
    }
}
