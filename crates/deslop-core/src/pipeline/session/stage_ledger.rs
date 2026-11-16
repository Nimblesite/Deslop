//! [PERF-FLUTTER-TODO-OBSERVABILITY] Per-stage boundary records for one render.

use std::time::Instant;

/// One bounded cluster-stage boundary record
/// ([PERF-FLUTTER-TODO-OBSERVABILITY]). A corpus-scale run spends
/// minutes between candidate survival and ranked clusters; at most a
/// handful of these per render, they make the running stage tellable
/// from a hang at the default `info` level, with the elapsed time that
/// attributes the gap.
///
/// The same rows are replayed as one `pipeline stage` event each after
/// the render completes, so a finished run's log reads as a compact
/// per-stage table instead of dense interleaved progress — the summary
/// is the small chunks the full log is broken up into
/// ([PERF-FLUTTER-TODO-OBSERVABILITY]).
#[derive(Default)]
pub(super) struct StageLedger {
    /// Completed stage boundaries, in run order.
    rows: Vec<StageRow>,
}

/// One recorded stage boundary: name, elapsed time, input and output
/// cardinality.
struct StageRow {
    /// Stage name.
    stage: &'static str,
    /// Wall time spent in the stage, milliseconds.
    elapsed_ms: u64,
    /// Items handed to the stage.
    input: usize,
    /// Items the stage produced.
    output: usize,
}

impl StageLedger {
    /// Records one completed stage boundary.
    pub(super) fn record(
        &mut self,
        stage: &'static str,
        input: usize,
        output: usize,
        started: Instant,
    ) {
        let elapsed_ms = crate::observe::elapsed_ms(started);
        tracing::info!(
            stage,
            input,
            output,
            elapsed_ms,
            rss_mib = crate::observe::resident_mib(),
            "cluster stage complete"
        );
        self.rows.push(StageRow {
            stage,
            elapsed_ms,
            input,
            output,
        });
    }

    /// Replays every recorded row as one compact event per stage, in
    /// run order, after the render completes.
    pub(super) fn log_summary(&self) {
        for row in &self.rows {
            tracing::info!(
                stage = row.stage,
                input = row.input,
                output = row.output,
                elapsed_ms = row.elapsed_ms,
                "pipeline stage"
            );
        }
    }
}
