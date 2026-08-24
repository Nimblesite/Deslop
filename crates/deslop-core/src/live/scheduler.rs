//! Single-flight coalescing re-analysis scheduler ([LIVE-SCHEDULER]).
//!
//! Owns one `tokio` task that consumes file-change events from the
//! watcher, feeds them into a [`Debouncer`], and dispatches an
//! [`AnalysisSession::apply_changes`] pass when the debouncer reports
//! ready. Only one pass runs at a time — incoming events queue while
//! the previous pass finishes.

use std::{path::PathBuf, sync::Arc, time::Duration};

use tokio::{
    sync::{
        broadcast::{self, Sender as BroadcastSender},
        mpsc::Receiver,
        Mutex,
    },
    time,
};

use super::{
    clock::{Clock, SystemClock},
    debouncer::Debouncer,
    notifications::{broadcast_report_changed, broadcast_state},
    session::AnalysisSession,
    wire::{AnalysisState, ChangeSummary, ReportChangedNotification},
};

/// Channel capacity for the broadcast channels. Larger capacities
/// trade memory for tolerance to slow subscribers ([LIVE-PERF-BUDGETS]).
const BROADCAST_CAPACITY: usize = 64;

/// Background scheduler handle.
#[derive(Debug)]
pub struct Scheduler {
    /// Broadcaster for `report/changed`.
    report_changed: BroadcastSender<ReportChangedNotification>,
    /// Broadcaster for `analysis/state`.
    analysis_state: BroadcastSender<AnalysisState>,
}

impl Scheduler {
    /// Spawns the scheduler task. The returned [`Scheduler`] handle
    /// outlives the task and exposes subscription endpoints.
    #[must_use]
    pub fn start(
        session: Arc<Mutex<AnalysisSession>>,
        watcher_rx: Receiver<PathBuf>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let (report_changed, _) =
            broadcast::channel::<ReportChangedNotification>(BROADCAST_CAPACITY);
        let (analysis_state, _) = broadcast::channel::<AnalysisState>(BROADCAST_CAPACITY);
        let task_state = SchedulerTaskState::new(
            session,
            watcher_rx,
            clock,
            report_changed.clone(),
            analysis_state.clone(),
        );
        let _join = tokio::spawn(task_state.run());
        Self {
            report_changed,
            analysis_state,
        }
    }

    /// Subscribes to `report/changed` notifications.
    #[must_use]
    pub fn subscribe_report_changed(
        &self,
    ) -> tokio::sync::broadcast::Receiver<ReportChangedNotification> {
        self.report_changed.subscribe()
    }

    /// Returns the underlying `report/changed` broadcast sender so
    /// out-of-band paths (cache-seed cold pass, IPC subscribers) can
    /// route through the same fan-out as the scheduler's own passes.
    /// Cloning a `Sender` is cheap; consumers call `.subscribe()` on
    /// the clone to attach.
    #[must_use]
    pub fn report_changed_sender(
        &self,
    ) -> tokio::sync::broadcast::Sender<ReportChangedNotification> {
        self.report_changed.clone()
    }

    /// Subscribes to `analysis/state` notifications.
    #[must_use]
    pub fn subscribe_state(&self) -> tokio::sync::broadcast::Receiver<AnalysisState> {
        self.analysis_state.subscribe()
    }

    /// Convenience constructor for the default [`SystemClock`].
    #[must_use]
    pub fn with_system_clock(
        session: Arc<Mutex<AnalysisSession>>,
        watcher_rx: Receiver<PathBuf>,
    ) -> Self {
        Self::start(session, watcher_rx, Arc::new(SystemClock::new()))
    }
}

/// Encapsulates the long-running task state. Lives on its own
/// `tokio` task spawned by [`Scheduler::start`].
#[derive(Debug)]
struct SchedulerTaskState {
    /// Shared session lock.
    session: Arc<Mutex<AnalysisSession>>,
    /// Watcher channel.
    watcher_rx: Receiver<PathBuf>,
    /// Debouncer instance, keyed off `clock`.
    debouncer: Debouncer,
    /// `report/changed` broadcaster.
    report_changed: BroadcastSender<ReportChangedNotification>,
    /// `analysis/state` broadcaster.
    analysis_state: BroadcastSender<AnalysisState>,
    /// Shared clock used for tick timestamps.
    clock: Arc<dyn Clock>,
    /// Generation carried by the last `report/changed` this scheduler
    /// sent. `None` until the first one goes out. Reads through
    /// [`LiveApi`](crate::live::LiveApi) refresh the session
    /// out-of-band and can advance the generation without announcing
    /// it, so "did this pass change anything" is the wrong question —
    /// the right one is "do subscribers already know about this
    /// generation" ([LIVE-SCHEDULER-NOOP]).
    last_announced_generation: Option<u64>,
}

impl SchedulerTaskState {
    /// Constructs a fresh task state.
    fn new(
        session: Arc<Mutex<AnalysisSession>>,
        watcher_rx: Receiver<PathBuf>,
        clock: Arc<dyn Clock>,
        report_changed: BroadcastSender<ReportChangedNotification>,
        analysis_state: BroadcastSender<AnalysisState>,
    ) -> Self {
        let debouncer = Debouncer::new(Arc::clone(&clock));
        Self {
            session,
            watcher_rx,
            debouncer,
            report_changed,
            analysis_state,
            clock,
            last_announced_generation: None,
        }
    }

    /// Top-level event loop.
    async fn run(mut self) {
        // `initialize` / `reportGet` hand every subscriber this
        // generation before the first watcher event can land, so it is
        // not news and must not be re-announced ([LIVE-SCHEDULER-NOOP]).
        self.last_announced_generation = Some(self.session.lock().await.generation());
        let mut tick = time::interval(Duration::from_millis(50));
        loop {
            tokio::select! {
                maybe_path = self.watcher_rx.recv() => {
                    match maybe_path {
                        Some(path) => self.debouncer.push(path),
                        None => break,
                    }
                }
                _ = tick.tick() => {
                    self.maybe_dispatch().await;
                }
            }
        }
    }

    /// Drains the debouncer and runs a re-analysis pass when due.
    async fn maybe_dispatch(&mut self) {
        if !self.debouncer.has_pending() || !self.debouncer.ready_to_flush() {
            return;
        }
        let changed = self.debouncer.flush();
        let started_at = self.clock.now_ms();
        broadcast_state(
            &self.analysis_state,
            AnalysisState::Running {
                started_at_ms: started_at,
            },
        );
        let outcome = self.run_pass(&changed).await;
        match outcome {
            Ok(notification) => {
                if let Some(notification) = notification {
                    broadcast_report_changed(&self.report_changed, notification);
                }
                broadcast_state(&self.analysis_state, AnalysisState::Idle);
            }
            Err(message) => {
                broadcast_state(&self.analysis_state, AnalysisState::Errored { message });
            }
        }
    }

    /// Runs a single `apply_changes` pass and translates the result
    /// into a wire notification.
    ///
    /// Returns `None` when subscribers already hold this generation
    /// ([LIVE-SCHEDULER-NOOP]) — announcing it only makes the panel,
    /// the diagnostics publisher and the MCP round-trip `reportDelta`
    /// → `reportGet` to re-fetch identical bytes. One production LSP
    /// served 281 such `reportGet` calls in two hours of build churn
    ///.
    ///
    /// The baseline is the last generation *announced*, never the one
    /// this pass happened to start from, so a generation an
    /// out-of-band read published silently still reaches subscribers
    /// here rather than being mistaken for a no-op.
    async fn run_pass(
        &mut self,
        changed: &[PathBuf],
    ) -> Result<Option<ReportChangedNotification>, String> {
        let mut guard = self.session.lock().await;
        let delta = guard
            .apply_changes(changed)
            .map_err(|err| err.to_string())?;
        let generation = guard.generation();
        drop(guard);
        if self.last_announced_generation == Some(generation) {
            tracing::debug!(
                generation,
                paths = changed.len(),
                "no-op pass; nothing broadcast"
            );
            return Ok(None);
        }
        self.last_announced_generation = Some(generation);
        Ok(Some(ReportChangedNotification {
            generation,
            summary: ChangeSummary::from_delta(&delta),
        }))
    }
}
