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

/// How often the rescue loop reports progress, counted in cross-file
/// pairs measured.
///
/// [PERF-FLUTTER-TODO-OBSERVABILITY] A stage that runs for a quarter of an
/// hour has to be distinguishable from a hang, but one record per pair is
/// not progress reporting. The Flutter corpus produced 793,076 of them —
/// 72 MB of log by the halfway point — and the formatting and I/O
/// measurably slowed the very stage being diagnosed, while burying the
/// stage events a reader needs under repetition. An interval keeps the
/// record count bounded by how long the stage runs rather than
/// proportional to the work it does.
const RESCUE_PROGRESS_INTERVAL: usize = 50_000;

/// What one rescue pass did, counted rather than narrated.
///
/// Counts at each gate, not just the last one: "measured 793,076 pairs"
/// says nothing about whether the population is large because the
/// eligibility test is too loose or because the corpus genuinely has that
/// many cross-file near-misses. Separating scanned from eligible from
/// cross-file from measured answers that from one record.
#[derive(Debug, Default)]
pub(super) struct RescueTally {
    /// Candidate pairs examined, whatever became of them.
    pub(super) scanned: usize,
    /// Pairs the fused threshold would drop despite token corroboration.
    pub(super) eligible: usize,
    /// Eligible pairs whose endpoints live in different files.
    pub(super) cross_file: usize,
    /// Cross-file pairs whose endpoints both resolved and were measured.
    pub(super) rescued_pairs: usize,
    /// Cross-file pairs abandoned because an endpoint did not resolve.
    pub(super) unresolved: usize,
}

impl RescueTally {
    /// Records one measurement attempt, reporting progress on schedule.
    pub(super) fn record(&mut self, measured: bool) {
        if measured {
            self.rescued_pairs = self.rescued_pairs.saturating_add(1);
        } else {
            self.unresolved = self.unresolved.saturating_add(1);
        }
        if self.cross_file.checked_rem(RESCUE_PROGRESS_INTERVAL) == Some(0) {
            tracing::debug!(
                scanned = self.scanned,
                eligible = self.eligible,
                cross_file = self.cross_file,
                rescued_pairs = self.rescued_pairs,
                unresolved = self.unresolved,
                "shared-subtree rescue in progress"
            );
        }
    }

    /// Emits the pass's totals. Always emitted, including when the stage
    /// found nothing eligible — an absent event and an empty population
    /// are otherwise indistinguishable in a log.
    pub(super) fn report_total(&self) {
        tracing::debug!(
            scanned = self.scanned,
            eligible = self.eligible,
            cross_file = self.cross_file,
            rescued_pairs = self.rescued_pairs,
            unresolved = self.unresolved,
            "shared-subtree rescue overlaps measured"
        );
    }
}
