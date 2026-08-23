//! Lightweight in-process CPU/work observability for `deslop-lsp`.
//!
//! This backs the `deslop/cpuReport` custom method. It deliberately
//! keeps only a small rolling history so bug reports can include enough
//! context to tell "idle spin" from legitimate analysis work without
//! shipping a full log stream.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

/// Maximum number of completed phases retained in a CPU report.
const PHASE_HISTORY_LIMIT: usize = 100;

/// Coarse work phases exposed to users diagnosing high CPU.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuPhase {
    /// No known analysis or request work is running.
    Idle,
    /// Source parsing is running.
    Parsing,
    /// Fingerprint extraction is running.
    Fingerprinting,
    /// Candidate pair clustering is running.
    Clustering,
    /// Embedding provider work is running.
    Embedding,
    /// Report rendering or projection is running.
    ReportRendering,
    /// File watcher debounce work is pending.
    WatchingDebounce,
}

/// One completed CPU/work phase in the rolling history.
#[derive(Clone, Debug, Serialize)]
pub struct CpuPhaseRecord {
    /// Phase name.
    phase: CpuPhase,
    /// Wall-clock start timestamp in Unix epoch milliseconds.
    started_at_ms: u64,
    /// Wall-clock duration in milliseconds.
    duration_ms: u64,
    /// Best-effort CPU milliseconds. Currently mirrors wall time until
    /// platform CPU accounting is wired in.
    cpu_ms: u64,
    /// Files touched by this phase, when known.
    files_touched: Vec<String>,
}

/// Snapshot of currently in-flight work queues.
#[derive(Clone, Debug, Default, Serialize)]
pub struct CpuInFlight {
    /// Pending filesystem watcher events.
    pending_watcher_events: u64,
    /// Pending embedding requests.
    pending_embed_requests: u64,
    /// Files in the current parse batch, when a parse batch is active.
    in_progress_parse_batch: Option<u64>,
}

/// Full `deslop/cpuReport` response body.
#[derive(Clone, Debug, Serialize)]
pub struct CpuReport {
    /// Current coarse work phase.
    current_phase: CpuPhase,
    /// Rolling history of the last completed phases.
    last_100_phases: Vec<CpuPhaseRecord>,
    /// Cumulative handler invocation counts for this LSP process.
    handler_counts: BTreeMap<String, u64>,
    /// Current in-flight queues and batches.
    in_flight: CpuInFlight,
}

/// Shared recorder used by the LSP backend and custom methods.
#[derive(Clone, Debug)]
pub struct Observability {
    /// Mutable observability state.
    inner: Arc<Mutex<CpuSnapshot>>,
}

impl Default for Observability {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(CpuSnapshot::default())),
        }
    }
}

impl Observability {
    /// Increments the named handler counter.
    pub fn record_handler(&self, name: &'static str) {
        if let Ok(mut inner) = self.inner.lock() {
            let entry = inner.handler_counts.entry(name.to_owned()).or_insert(0);
            *entry = entry.saturating_add(1);
        }
    }

    /// Starts a coarse work phase and returns a guard that records its
    /// duration when dropped.
    #[must_use]
    pub fn start_phase(&self, phase: CpuPhase, files_touched: Vec<String>) -> PhaseGuard {
        let started_at_ms = now_ms();
        if let Ok(mut inner) = self.inner.lock() {
            inner.current_phase = phase;
        }
        PhaseGuard {
            observability: self.clone(),
            phase,
            started_at_ms,
            started: Instant::now(),
            files_touched,
        }
    }

    /// Returns a stable snapshot suitable for JSON serialisation.
    #[must_use]
    pub fn snapshot(&self) -> CpuReport {
        self.inner.lock().map_or_else(
            |_| CpuReport {
                current_phase: CpuPhase::Idle,
                last_100_phases: Vec::new(),
                handler_counts: BTreeMap::new(),
                in_flight: CpuInFlight::default(),
            },
            |inner| CpuReport {
                current_phase: inner.current_phase,
                last_100_phases: inner.history.clone(),
                handler_counts: inner.handler_counts.clone(),
                in_flight: inner.in_flight.clone(),
            },
        )
    }

    /// Records one completed phase.
    fn finish_phase(&self, record: CpuPhaseRecord) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.current_phase = CpuPhase::Idle;
            if inner.history.len() >= PHASE_HISTORY_LIMIT {
                let _removed = inner.history.remove(0);
            }
            inner.history.push(record);
        }
    }
}

/// Mutable state behind [`Observability`].
#[derive(Debug)]
struct CpuSnapshot {
    /// Current coarse work phase.
    current_phase: CpuPhase,
    /// Rolling phase history.
    history: Vec<CpuPhaseRecord>,
    /// Cumulative handler counts.
    handler_counts: BTreeMap<String, u64>,
    /// Current in-flight work.
    in_flight: CpuInFlight,
}

impl Default for CpuSnapshot {
    fn default() -> Self {
        Self {
            current_phase: CpuPhase::Idle,
            history: Vec::new(),
            handler_counts: BTreeMap::new(),
            in_flight: CpuInFlight::default(),
        }
    }
}

/// Guard returned by [`Observability::start_phase`].
#[derive(Debug)]
pub struct PhaseGuard {
    /// Recorder to update on drop.
    observability: Observability,
    /// Phase being recorded.
    phase: CpuPhase,
    /// Wall-clock start timestamp.
    started_at_ms: u64,
    /// Monotonic start used for duration.
    started: Instant,
    /// Files touched by the phase.
    files_touched: Vec<String>,
}

impl Drop for PhaseGuard {
    fn drop(&mut self) {
        let duration_ms = duration_ms(self.started);
        self.observability.finish_phase(CpuPhaseRecord {
            phase: self.phase,
            started_at_ms: self.started_at_ms,
            duration_ms,
            cpu_ms: duration_ms,
            files_touched: self.files_touched.clone(),
        });
    }
}

/// Returns milliseconds elapsed since `started`.
fn duration_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Returns the current Unix epoch timestamp in milliseconds.
fn now_ms() -> u64 {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPORT_GET_METHOD: &str = "deslop/reportGet";

    #[test]
    fn records_handler_counts_and_phase_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
        let observability = Observability::default();
        observability.record_handler(REPORT_GET_METHOD);
        observability.record_handler(REPORT_GET_METHOD);

        let guard = observability.start_phase(CpuPhase::Parsing, vec!["src/Alpha.cs".to_owned()]);
        let running = observability.snapshot();
        assert_eq!(running.current_phase, CpuPhase::Parsing);
        assert_eq!(
            running.handler_counts.get(REPORT_GET_METHOD),
            Some(&2),
            "handler counts must saturating-increment by method name"
        );
        assert!(
            running.last_100_phases.is_empty(),
            "phase history is recorded when the guard drops"
        );

        drop(guard);
        let finished = observability.snapshot();
        assert_eq!(finished.current_phase, CpuPhase::Idle);
        assert_eq!(finished.last_100_phases.len(), 1);
        let record = finished
            .last_100_phases
            .first()
            .ok_or_else(|| std::io::Error::other("finished phase missing"))?;
        assert_eq!(record.phase, CpuPhase::Parsing);
        assert_eq!(record.files_touched, vec!["src/Alpha.cs"]);
        assert_eq!(record.cpu_ms, record.duration_ms);
        assert!(record.started_at_ms <= now_ms());
        Ok(())
    }

    #[test]
    fn phase_history_keeps_only_the_last_hundred_records() -> Result<(), Box<dyn std::error::Error>>
    {
        let observability = Observability::default();

        for index in 0..=PHASE_HISTORY_LIMIT {
            observability.finish_phase(CpuPhaseRecord {
                phase: CpuPhase::ReportRendering,
                started_at_ms: index as u64,
                duration_ms: index as u64,
                cpu_ms: index as u64,
                files_touched: vec![format!("file-{index}.rs")],
            });
        }

        let snapshot = observability.snapshot();
        assert_eq!(snapshot.current_phase, CpuPhase::Idle);
        assert_eq!(snapshot.last_100_phases.len(), PHASE_HISTORY_LIMIT);
        let first = snapshot
            .last_100_phases
            .first()
            .ok_or_else(|| std::io::Error::other("first phase missing"))?;
        let last = snapshot
            .last_100_phases
            .last()
            .ok_or_else(|| std::io::Error::other("last phase missing"))?;
        assert_eq!(
            first.files_touched,
            vec!["file-1.rs"],
            "oldest phase must be evicted when the rolling window is full"
        );
        assert_eq!(
            last.files_touched,
            vec![format!("file-{PHASE_HISTORY_LIMIT}.rs")]
        );
        Ok(())
    }

    #[test]
    fn snapshot_falls_back_to_empty_idle_report_when_lock_is_poisoned() {
        let observability = Observability::default();
        let poisoned = observability.clone();
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let _poisoned = std::panic::catch_unwind(move || {
            if let Ok(_guard) = poisoned.inner.lock() {
                assert_eq!(1, 2, "poison observability lock for fallback coverage");
            }
        });
        std::panic::set_hook(previous_hook);

        observability.record_handler("ignored-after-poison");
        let guard = observability.start_phase(CpuPhase::Embedding, Vec::new());
        drop(guard);

        let snapshot = observability.snapshot();
        assert_eq!(snapshot.current_phase, CpuPhase::Idle);
        assert!(snapshot.last_100_phases.is_empty());
        assert!(snapshot.handler_counts.is_empty());
        assert_eq!(snapshot.in_flight.pending_watcher_events, 0);
        assert_eq!(snapshot.in_flight.pending_embed_requests, 0);
        assert_eq!(snapshot.in_flight.in_progress_parse_batch, None);
    }

    #[test]
    fn cpu_phase_serializes_with_snake_case_names() -> Result<(), serde_json::Error> {
        assert_eq!(serde_json::to_value(CpuPhase::Idle)?, "idle");
        assert_eq!(
            serde_json::to_value(CpuPhase::Fingerprinting)?,
            "fingerprinting"
        );
        assert_eq!(serde_json::to_value(CpuPhase::Clustering)?, "clustering");
        assert_eq!(serde_json::to_value(CpuPhase::Embedding)?, "embedding");
        assert_eq!(
            serde_json::to_value(CpuPhase::ReportRendering)?,
            "report_rendering"
        );
        assert_eq!(
            serde_json::to_value(CpuPhase::WatchingDebounce)?,
            "watching_debounce"
        );
        Ok(())
    }

    #[test]
    fn duration_and_timestamp_helpers_are_bounded() {
        let started = Instant::now();
        let elapsed = duration_ms(started);
        assert!(
            elapsed < u64::MAX,
            "duration helper should convert ordinary monotonic elapsed time"
        );
        assert!(
            now_ms() > 0,
            "wall-clock helper should return a Unix epoch millisecond timestamp"
        );
    }
}
