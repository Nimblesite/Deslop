//! [LIVE-SCHEDULER-IDLE] A quiet scheduler must not wake on a polling timer.

use std::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Wake, Waker},
};

use anyhow::{Context as _, Result};
use tokio::sync::{broadcast::error::TryRecvError, mpsc};

use super::*;
use crate::{
    embedding::{EmbeddingMode, NoopProvider},
    live::CAP_MS,
};

const CHANNEL_SIZE: usize = 4;
const MIN_NODES: u32 = 30;
const NO_WAKEUPS: usize = 0;
const SINGLE_WAKEUP: usize = 1;
const IDLE_WINDOW: Duration = Duration::from_secs(60);
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(CAP_MS);
const UNTRACKED_PATH: &str = "untracked.rs";

/// Counts wakeups without polling the scheduler while virtual time advances.
#[derive(Debug, Default)]
struct WakeCounter(AtomicUsize);

impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        let _previous = self.0.fetch_add(SINGLE_WAKEUP, Ordering::SeqCst);
    }
}

/// Makes the debouncer read the same virtual time as Tokio's timers.
#[derive(Debug)]
struct VirtualClock(time::Instant);

impl Clock for VirtualClock {
    fn now_ms(&self) -> u64 {
        u64::try_from(self.0.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

/// Creates an empty analysis session so the test isolates scheduler work.
fn session(root: &std::path::Path) -> Result<Arc<Mutex<AnalysisSession>>> {
    Ok(Arc::new(Mutex::new(AnalysisSession::new(
        root.to_path_buf(),
        MIN_NODES,
        false,
        None,
        Arc::new(NoopProvider::new()),
    )?)))
}

/// Polls one turn while keeping the task available for subsequent events.
fn assert_pending(future: Pin<&mut impl Future<Output = ()>>, context: &mut Context<'_>) {
    assert!(
        future.poll(context).is_pending(),
        "scheduler stays available"
    );
}

/// Advances virtual time while the scheduler is parked on its input channel.
async fn assert_dormant(
    future: Pin<&mut impl Future<Output = ()>>,
    context: &mut Context<'_>,
    wakes: &WakeCounter,
) {
    assert_pending(future, context);
    wakes.0.store(NO_WAKEUPS, Ordering::SeqCst);
    time::advance(IDLE_WINDOW).await;
    assert_eq!(
        wakes.0.load(Ordering::SeqCst),
        NO_WAKEUPS,
        "a scheduler with no pending file changes must not wake on a timer"
    );
}

/// Owns watcher inputs and subscribed outputs for one scheduler task.
struct SchedulerInputs {
    /// Workspace retained until the scheduler finishes.
    root: tempfile::TempDir,
    /// An open channel keeps the quiet scheduler alive.
    events: mpsc::UnboundedSender<PathBuf>,
    /// Report publications must remain absent for an untracked path.
    reports: broadcast::Receiver<ReportChangedNotification>,
    /// Analysis lifecycle proves queued events still dispatch.
    states: broadcast::Receiver<AnalysisState>,
}

/// Builds the real scheduler future without spawning it, exposing its waker.
fn scheduler_fixture(root: tempfile::TempDir) -> Result<(SchedulerTaskState, SchedulerInputs)> {
    let (events, receiver) = mpsc::unbounded_channel();
    let (report_sender, reports) = broadcast::channel(CHANNEL_SIZE);
    let (state_sender, states) = broadcast::channel(CHANNEL_SIZE);
    let task = SchedulerTaskState::new(
        session(root.path())?,
        receiver,
        Arc::new(VirtualClock(time::Instant::now())),
        report_sender,
        state_sender,
    );
    let inputs = SchedulerInputs {
        root,
        events,
        reports,
        states,
    };
    Ok((task, inputs))
}

impl SchedulerInputs {
    /// Drives a queued event through the deadline and into a completed pass.
    async fn dispatch(
        &mut self,
        mut future: Pin<&mut impl Future<Output = ()>>,
        context: &mut Context<'_>,
        wakes: &WakeCounter,
    ) -> Result<()> {
        self.events.send(self.root.path().join(UNTRACKED_PATH))?;
        assert_eq!(wakes.0.load(Ordering::SeqCst), SINGLE_WAKEUP);
        assert_pending(future.as_mut(), context);
        time::advance(DEBOUNCE_WINDOW).await;
        assert_pending(future, context);
        assert_completed_pass(&mut self.states, &mut self.reports)
    }
}

/// Pins both state transitions and the absence of a changed report.
fn assert_completed_pass(
    states: &mut broadcast::Receiver<AnalysisState>,
    reports: &mut broadcast::Receiver<ReportChangedNotification>,
) -> Result<()> {
    assert!(matches!(states.try_recv()?, AnalysisState::Running { .. }));
    assert!(matches!(states.try_recv()?, AnalysisState::Idle));
    assert!(matches!(reports.try_recv(), Err(TryRecvError::Empty)));
    Ok(())
}

/// An event followed by channel closure must end the task without a timer.
fn assert_stopped(future: Pin<&mut impl Future<Output = ()>>, context: &mut Context<'_>) {
    assert!(
        future.poll(context).is_ready(),
        "closed watcher stops the scheduler"
    );
}

#[tokio::test(start_paused = true)]
async fn empty_scheduler_sleeps_before_and_after_a_pass() -> Result<()> {
    let (task, mut inputs) = scheduler_fixture(tempfile::tempdir()?)?;
    let wakes = Arc::new(WakeCounter::default());
    let task_waker = Waker::from(Arc::clone(&wakes));
    let mut context = Context::from_waker(&task_waker);
    let mut future = Box::pin(task.run());
    assert_dormant(future.as_mut(), &mut context, &wakes).await;
    inputs
        .dispatch(future.as_mut(), &mut context, &wakes)
        .await?;
    assert_dormant(future.as_mut(), &mut context, &wakes).await;
    drop(inputs);
    assert_stopped(future.as_mut(), &mut context);
    Ok(())
}

/// [LIVE-WATCHER-DELIVERY] Already accepted changes finish when inputs close.
#[tokio::test(start_paused = true)]
async fn closed_watcher_flushes_queued_changes_before_scheduler_stops() -> Result<()> {
    let (task, inputs) = scheduler_fixture(tempfile::tempdir()?)?;
    inputs
        .events
        .send(inputs.root.path().join(UNTRACKED_PATH))?;
    let SchedulerInputs {
        root: _root,
        events,
        mut states,
        mut reports,
    } = inputs;
    let _report_sender = task.report_changed.clone();
    drop(events);
    let stopped_at = time::Instant::now();
    task.run().await;
    assert_eq!(stopped_at.elapsed(), Duration::ZERO);
    assert_completed_pass(&mut states, &mut reports)
}

/// Reopens the fixture's persisted report before its real pipeline is installed.
async fn use_cached_seed(task: &mut SchedulerTaskState, root: &std::path::Path) -> Result<()> {
    let seeded = AnalysisSession::try_seeded_from_cache(
        root.to_path_buf(),
        MIN_NODES,
        false,
        None,
        Arc::new(NoopProvider::new()),
        EmbeddingMode::Off,
    )
    .context("fixture must persist a usable report seed")?;
    assert!(seeded.is_seed_only());
    task.last_announced_generation = Some(seeded.generation());
    *task.session.lock().await = seeded;
    Ok(())
}

/// Queued seed changes expose Running without announcing completion or a report.
fn assert_deferred_pass(inputs: &mut SchedulerInputs) -> Result<()> {
    assert!(matches!(
        inputs.states.try_recv()?,
        AnalysisState::Running { .. }
    ));
    assert!(
        matches!(inputs.states.try_recv(), Err(TryRecvError::Empty)),
        "queued seed-only work must not publish Idle before the pipeline exists"
    );
    assert!(matches!(
        inputs.reports.try_recv(),
        Err(TryRecvError::Empty)
    ));
    Ok(())
}

/// [LIVE-CACHE-SEED-READINESS] Queuing work cannot finish an outstanding cold scan.
#[tokio::test]
async fn seed_only_scheduler_stays_running_after_queuing_changes() -> Result<()> {
    let (mut task, mut inputs) = scheduler_fixture(tempfile::tempdir()?)?;
    use_cached_seed(&mut task, inputs.root.path()).await?;
    task.debouncer.push(inputs.root.path().join(UNTRACKED_PATH));
    task.dispatch_pending().await;
    assert_deferred_pass(&mut inputs)?;
    assert!(task.session.lock().await.is_seed_only());
    Ok(())
}
