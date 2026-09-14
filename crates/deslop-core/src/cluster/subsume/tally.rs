//! [PIPELINE-OBSERVABILITY-STAGES] Aggregate records for cross-cluster
//! subsumption: a completion record with the stage's counts, and a
//! fixed-interval progress record so a long resolution is
//! distinguishable from a hang.

use std::time::Instant;

use crate::observe::{bump, elapsed_ms};

use super::survivor::Preference;

/// Pair evaluations between two progress records.
const SUBSUME_PROGRESS_INTERVAL: u64 = 10_000_000;

/// Running counts for one subsumption pass over the ranked list.
#[derive(Debug)]
pub(super) struct SubsumeTally {
    /// Ranked views the pass started from.
    views: usize,
    /// Distinct file sets those views name.
    file_sets: usize,
    /// Pairs whose region relation was evaluated.
    evaluated: u64,
    /// Pairs that describe the same duplication.
    same_region: u64,
    /// Views absorbed by a published view.
    absorbed: u64,
    /// Regions whose survivor order had a cycle.
    cycles: u64,
    /// Rounds that removed straddling views and resolved again.
    straddle_rounds: u64,
    /// Views removed as straddlers.
    straddled: u64,
    /// When the pass began.
    started: Instant,
}

impl SubsumeTally {
    /// Starts counting for `views` ranked views over `file_sets` file sets.
    pub(super) fn new(views: usize, file_sets: usize) -> Self {
        Self {
            views,
            file_sets,
            evaluated: 0,
            same_region: 0,
            absorbed: 0,
            cycles: 0,
            straddle_rounds: 0,
            straddled: 0,
            started: Instant::now(),
        }
    }

    /// Counts one evaluated pair and its verdict.
    pub(super) fn evaluated(&mut self, preference: Preference) {
        bump(&mut self.evaluated);
        if preference != Preference::Neither {
            bump(&mut self.same_region);
        }
        if self.evaluated.checked_rem(SUBSUME_PROGRESS_INTERVAL) == Some(0) {
            self.report("cross-cluster subsumption in progress", None);
        }
    }

    /// Counts one view absorbed by a published view.
    pub(super) fn absorbed(&mut self) {
        bump(&mut self.absorbed);
    }

    /// Counts one cycle decided by the coverage-mass-id order.
    pub(super) fn cycle_broken(&mut self) {
        bump(&mut self.cycles);
    }

    /// Counts one straddle round that removed `pairs` straddling pairs.
    pub(super) fn straddle_round(&mut self, pairs: usize) {
        bump(&mut self.straddle_rounds);
        let removed = u64::try_from(pairs).unwrap_or(u64::MAX).saturating_mul(2);
        self.straddled = self.straddled.saturating_add(removed);
    }

    /// Emits the completion record with the number of published views.
    pub(super) fn complete(&self, survivors: usize) {
        self.report("cross-cluster subsumption complete", Some(survivors));
    }

    /// One aggregate record; counts and durations only.
    fn report(&self, message: &'static str, survivors: Option<usize>) {
        tracing::info!(
            stage = "cross_cluster_subsume",
            views = self.views,
            file_sets = self.file_sets,
            evaluated = self.evaluated,
            same_region = self.same_region,
            absorbed = self.absorbed,
            cycles = self.cycles,
            straddle_rounds = self.straddle_rounds,
            straddled = self.straddled,
            survivors,
            elapsed_ms = elapsed_ms(self.started),
            "{message}"
        );
    }
}
