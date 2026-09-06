//! [PERF-FLUTTER-TODO-OBSERVABILITY] What one shared-subtree rescue pass
//! did, counted rather than narrated.
//!
//! The rescue loop used to emit one `tracing::debug!` per measured pair.
//! On the pinned Flutter corpus that was 793,076 records — 72 MB of log by
//! the halfway point — and the formatting and I/O measurably slowed the
//! very stage being diagnosed, while burying the stage events a reader
//! needs under repetition. Counts at every gate say strictly more, in a
//! volume bounded by how long the stage runs rather than by how much work
//! it does.

use std::time::Instant;

use crate::observe::{bump, elapsed_ms};

use super::MeasureStats;

/// How often the rescue loop reports progress, counted in pairs measured.
///
/// [PERF-FLUTTER-TODO-OBSERVABILITY] A stage that runs for a quarter of an
/// hour has to be distinguishable from a hang, but one record per pair is
/// not progress reporting. An interval keeps the record count bounded by
/// how long the stage runs rather than proportional to the work it does.
/// Counted on measured pairs specifically: those are the ones that cost
/// alignment time, so the cadence tracks the work rather than the scan.
const RESCUE_PROGRESS_INTERVAL: u64 = 50_000;

/// What one rescue pass did, counted rather than narrated.
///
/// Counts at each gate, not just the last one: "measured 793,076 pairs"
/// says nothing about whether the population is large because the
/// eligibility test is too loose or because the corpus genuinely has that
/// many cross-file near-misses. Separating scanned from eligible from
/// cross-file from measured from rescued answers that from one record.
#[derive(Debug, Clone)]
pub(super) struct RescueTally {
    /// Candidate pairs examined, whatever became of them.
    pub(super) scanned: u64,
    /// Pairs whose alignment must be measured: dropped by the fused
    /// threshold despite token corroboration, or carried by the token axis
    /// alone ([`crate::pair::alignment_required`]).
    pub(super) eligible: u64,
    /// Eligible pairs whose endpoints live in different files — the
    /// population handed to the measurer.
    pub(super) cross_file: u64,
    /// Cross-file pairs the measurer answered, from any route.
    pub(super) measured: u64,
    /// Measured pairs whose overlap cleared
    /// [`crate::pair::SHARED_SUBTREE_MIN_OVERLAP`] — the pairs the
    /// rescue actually admits. Distinct from `measured`, which counts
    /// every pair the route looked at: conflating the two reports a
    /// rescue population that never existed.
    pub(super) rescued: u64,
    /// Measured pairs whose overlap cleared the floor but whose own
    /// content agreement did not ([FUSED-CONTENT-GATE], gh #458): the
    /// rescue looked, then refused. Distinct from `rescued` because a
    /// Merkle-identical signature can clear the overlap floor while the
    /// endpoints' collapsed leaves share nothing (the
    /// `verbatim-plus-stranger` stranger measures 0.0436) — admitting
    /// those would launder a false duplicate into a proven family's
    /// act-now cluster.
    pub(super) content_gate_rejected: u64,
    /// Measured pairs whose overlap cleared the floor but whose shared
    /// mass, beyond an exact whole-function clone both endpoints
    /// enclose, fell short of [`crate::pair::SHARED_SUBTREE_MIN_NODE_COUNT`]
    /// ([FUSED-SHARED-SUBTREE-ECHO]): a container echoing a clone the
    /// anchor axis already proved, refused so it cannot eat that clone.
    pub(super) container_echo_rejected: u64,
    /// Stage start, for the throughput a reader needs to tell slow from
    /// stuck.
    started: Instant,
}

impl RescueTally {
    /// Opens a tally, starting the stage clock.
    pub(super) fn new() -> Self {
        Self {
            scanned: 0,
            eligible: 0,
            cross_file: 0,
            measured: 0,
            rescued: 0,
            content_gate_rejected: 0,
            container_echo_rejected: 0,
            started: Instant::now(),
        }
    }

    /// Records one candidate examined.
    pub(super) fn scan(&mut self) {
        bump(&mut self.scanned);
    }

    /// Records one pair past the eligibility gate.
    pub(super) fn eligible(&mut self) {
        bump(&mut self.eligible);
    }

    /// Records one pair past the cross-file gate.
    pub(super) fn cross_file(&mut self) {
        bump(&mut self.cross_file);
    }

    /// Records one measured pair and whether it cleared the admission
    /// floor, reporting progress on schedule.
    pub(super) fn measure(&mut self, rescued: bool, measure: MeasureStats) {
        bump(&mut self.measured);
        if rescued {
            bump(&mut self.rescued);
        }
        if self.measured.checked_rem(RESCUE_PROGRESS_INTERVAL) == Some(0) {
            self.report("shared-subtree rescue in progress", measure);
        }
    }

    /// Records one pair the content gate refused to rescue.
    pub(super) fn content_gate_rejected(&mut self) {
        bump(&mut self.content_gate_rejected);
    }

    /// Records a pair the container-echo rule refused.
    pub(super) fn container_echo_rejected(&mut self) {
        bump(&mut self.container_echo_rejected);
    }

    /// Folds another tally's counts into this one. The stage clock stays
    /// this tally's own — shard tallies share the pass start, so the
    /// merged elapsed time is the pass's ([PERF-FLUTTER-TODO-RESCUE]).
    pub(super) fn absorb(&mut self, other: &RescueTally) {
        self.scanned = self.scanned.saturating_add(other.scanned);
        self.eligible = self.eligible.saturating_add(other.eligible);
        self.cross_file = self.cross_file.saturating_add(other.cross_file);
        self.measured = self.measured.saturating_add(other.measured);
        self.rescued = self.rescued.saturating_add(other.rescued);
        self.content_gate_rejected = self
            .content_gate_rejected
            .saturating_add(other.content_gate_rejected);
        self.container_echo_rejected = self
            .container_echo_rejected
            .saturating_add(other.container_echo_rejected);
    }

    /// Emits the pass's totals. Always emitted, including when the stage
    /// found nothing eligible — an absent event and an empty population
    /// are otherwise indistinguishable in a log.
    pub(super) fn report_total(&self, measure: MeasureStats) {
        self.report("shared-subtree rescue overlaps measured", measure);
    }

    /// One aggregate record. `info`, not `debug`: the whole point is that
    /// a default-level run of a corpus-scale repository can be told apart
    /// from a hung one, and the Flutter run that prompted this emitted
    /// nothing at `info` for fifteen minutes.
    fn report(&self, message: &'static str, measure: MeasureStats) {
        tracing::info!(
            scanned = self.scanned,
            eligible = self.eligible,
            cross_file = self.cross_file,
            measured = self.measured,
            rescued_pairs = self.rescued,
            content_gate_rejected = self.content_gate_rejected,
            container_echo_rejected = self.container_echo_rejected,
            alignments = measure.alignments,
            credit_fallbacks = measure.credit_fallbacks,
            hash_equal = measure.hash_equal,
            exact_hits = measure.exact_hits,
            bound_hits = measure.bound_hits,
            bound_skips = measure.bound_skips,
            order_skips = measure.order_skips,
            unresolved = measure.unresolved,
            elapsed_ms = elapsed_ms(self.started),
            message
        );
    }
}
