//! [CORPUS-BASELINE] The known-failures ratchet: which corpus checks already
//! fail, and how a run's failures are classified against them.
//! Spec: `docs/specs/corpus.md` [CORPUS-BASELINE].

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use serde_json::Value;

use super::{repo_root, Failure};

/// Environment variable that switches the suite from strict mode (any failure
/// fails the test) to baseline mode (only *new* failures fail).
pub const BASELINE_ENV: &str = "DESLOP_CORPUS_BASELINE";

/// [CORPUS-BASELINE] The set of checks already known to fail, per repository.
///
/// This is a ratchet, not an excuse: entries record defects that already have
/// a tracked issue, so CI reports them without blocking. Anything not listed
/// is a regression and fails even in baseline mode.
#[derive(Debug, Default)]
pub struct Baseline {
    /// Check ids already known to fail, keyed by repository name.
    known: BTreeMap<String, BTreeSet<String>>,
}

impl Baseline {
    /// Loads `corpus/known-failures.json`.
    ///
    /// # Errors
    ///
    /// Returns an error when the file exists but is not valid JSON.
    pub fn load() -> Result<Self> {
        let path = repo_root().join("corpus").join("known-failures.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        Ok(Self::parse(&crate::read_json(&path)?))
    }

    /// Reads a `known-failures.json` document already in memory. Entries that
    /// are not lists of check ids contribute nothing.
    #[must_use]
    pub fn parse(document: &Value) -> Self {
        let known = document
            .get("known_failures")
            .and_then(Value::as_object)
            .map(|entries| {
                entries
                    .iter()
                    .map(|(repo, checks)| (repo.clone(), check_ids(checks)))
                    .collect()
            })
            .unwrap_or_default();
        Self { known }
    }

    /// Checks recorded as already failing for `repo`.
    #[must_use]
    pub fn known_for(&self, repo: &str) -> BTreeSet<String> {
        self.known.get(repo).cloned().unwrap_or_default()
    }
}

/// The check ids one `known_failures` entry lists.
fn check_ids(checks: &Value) -> BTreeSet<String> {
    checks
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// True when the suite should report known failures instead of failing on them.
#[must_use]
pub fn baseline_mode() -> bool {
    std::env::var(BASELINE_ENV).is_ok_and(|value| value != "0" && !value.is_empty())
}

/// Prints every observed failure, classified against the baseline, and returns
/// the failures that should fail the test.
///
/// In strict mode (the default, and what `make test-corpus` runs locally) that
/// is all of them. In baseline mode it is only the ones not already recorded.
/// Checks in the baseline that did *not* fire are reported as possibly fixed
/// but never fail a run — with #301 outstanding, a lucky pass is not proof.
/// `evaluated` names the checks this caller actually ran. It is required
/// because a repository's checks are split across more than one test: the
/// determinism gate cannot observe `memory`, and the main gate cannot observe
/// `determinism`. Without it, each test would report the other's baseline
/// entries as possibly fixed while they were still failing elsewhere.
#[must_use]
pub fn classify(
    repo: &str,
    evaluated: &[&str],
    observed: &[Failure],
    baseline: &Baseline,
) -> Vec<Failure> {
    let known = baseline.known_for(repo);
    let (fresh, carried): (Vec<Failure>, Vec<Failure>) = observed
        .iter()
        .cloned()
        .partition(|failure| !known.contains(&failure.check));

    print_failures("[KNOWN] ", repo, &carried);
    print_failures("[NEW]   ", repo, &fresh);
    print_possibly_fixed(repo, evaluated, observed, &known);

    if baseline_mode() {
        fresh
    } else {
        observed.to_vec()
    }
}

/// Prints each failure under a classification label.
fn print_failures(label: &str, repo: &str, failures: &[Failure]) {
    for failure in failures {
        println!("  {label} {repo}/{}: {}", failure.check, failure.detail);
    }
}

/// Prints the baseline entries that were evaluated this run and did not fire.
///
/// Scoped to `evaluated` on purpose: a repository's checks are split across
/// more than one test, so an unscoped reconciliation would announce the
/// determinism gate's live defect as fixed from inside the resource gate.
fn print_possibly_fixed(
    repo: &str,
    evaluated: &[&str],
    observed: &[Failure],
    known: &BTreeSet<String>,
) {
    let observed_checks: BTreeSet<&str> = observed
        .iter()
        .map(|failure| failure.check.as_str())
        .collect();
    let evaluated: BTreeSet<&str> = evaluated.iter().copied().collect();
    for check in known
        .iter()
        .filter(|check| evaluated.contains(check.as_str()))
        .filter(|check| !observed_checks.contains(check.as_str()))
    {
        println!(
            "  [FIXED?] {repo}/{check}: baseline expects this to fail but it passed. \
             Confirm, then remove it from corpus/known-failures.json."
        );
    }
}
