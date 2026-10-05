//! [LIVE-CACHE-SEED-READINESS] A prepared pipeline is not ready before commit.

use super::*;

const QUEUED_PATH: &str = "Queued.cs";
const EMPTY_SOURCE: &str = "";
const ADDED_FILE_COUNT: usize = 1;
const RUNNING_STATE: &str = "running";
const ERRORED_STATE: &str = "errored";
const FAILURE_MESSAGE: &str = "cold startup failed";
const MESSAGE_POINTER: &str = "/params/message";

/// Shared error boundary for the startup lifecycle fixture.
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Reads a state notification while retaining its fields for additional checks.
async fn state_frame(socket: &mut ClientSocket, expected: &str) -> TestResult<Value> {
    let frame = next_client_frame(socket).await?;
    assert_eq!(
        frame.pointer(STATE_POINTER).and_then(Value::as_str),
        Some(expected)
    );
    Ok(frame)
}

/// Requires the original failure state and diagnostic on every delivery.
async fn assert_failure(socket: &mut ClientSocket, expected: &str) -> TestResult {
    let frame = state_frame(socket, ERRORED_STATE).await?;
    assert_eq!(
        frame.pointer(METHOD_POINTER).and_then(Value::as_str),
        Some(ANALYSIS_STATE)
    );
    assert_eq!(
        frame.pointer(MESSAGE_POINTER).and_then(Value::as_str),
        Some(expected)
    );
    Ok(())
}

/// [LIVE-CACHE-SEED-READINESS] Failed cold work remains failed for late clients.
#[tokio::test]
async fn late_initialization_preserves_the_cold_refresh_error() -> TestResult {
    let root = tempfile::tempdir()?;
    let startup = new_startup_state(true);
    let mut fixture = seeded_refresh(root.path(), Arc::clone(&startup)).await?;
    let session = Arc::clone(&fixture.task.session);
    let client = fixture.task.client.clone();
    let error = LiveError::SchedulerBusy {
        message: FAILURE_MESSAGE.to_owned(),
    };
    let expected = error.to_string();
    finish_refresh(fixture.task, Err(error)).await;
    assert_failure(&mut fixture.socket, &expected).await?;
    assert!(session.lock().await.is_seed_only());
    push_initial_state(&client, &startup).await;
    assert_failure(&mut fixture.socket, &expected).await
}

/// Opens a persisted seed so the cold pass must install its real pipeline.
async fn seeded_refresh(
    root: &Path,
    startup: Arc<Mutex<AnalysisState>>,
) -> TestResult<RefreshFixture> {
    drop(refresh_fixture(root, Arc::clone(&startup)).await?);
    let fixture = refresh_fixture(root, startup).await?;
    assert!(fixture.seeded);
    assert!(fixture.task.session.lock().await.is_seed_only());
    Ok(fixture)
}

/// Adds an event after the prepared cold snapshot, requiring replay at commit.
fn queue_new_file(session: &mut AnalysisSession, root: &Path) -> TestResult<usize> {
    let changed = root.join(QUEUED_PATH);
    std::fs::write(&changed, EMPTY_SOURCE)?;
    let _delta = session.apply_changes(&[changed])?;
    session
        .report()
        .files_analysed
        .checked_add(ADDED_FILE_COUNT)
        .ok_or_else(|| std::io::Error::other("fixture file count exceeds usize").into())
}

/// Checks ordered completion notifications and the fully replayed session.
async fn assert_committed(
    session: &Mutex<AnalysisSession>,
    startup: &Mutex<AnalysisState>,
    socket: &mut ClientSocket,
    expected_files: usize,
) -> TestResult {
    let changed = next_client_frame(socket).await?;
    assert_eq!(
        changed.pointer(METHOD_POINTER).and_then(Value::as_str),
        Some(REPORT_CHANGED)
    );
    let _idle = state_frame(socket, IDLE_STATE).await?;
    assert!(matches!(*startup.lock().await, AnalysisState::Idle));
    let guard = session.lock().await;
    assert!(!guard.is_seed_only());
    assert_eq!(guard.report().files_analysed, expected_files);
    Ok(())
}

/// A late editor sees Running while commit is blocked on the session lock.
async fn assert_initial_running(
    client: &Client,
    startup: &Mutex<AnalysisState>,
    socket: &mut ClientSocket,
) -> TestResult {
    assert!(
        matches!(*startup.lock().await, AnalysisState::Running { .. }),
        "a cold pass waiting to commit must remain active"
    );
    push_initial_state(client, startup).await;
    let _initial = state_frame(socket, RUNNING_STATE).await?;
    Ok(())
}

#[tokio::test]
async fn cold_pass_remains_active_until_pending_changes_are_committed() -> TestResult {
    let root = tempfile::tempdir()?;
    let startup = new_startup_state(true);
    let mut fixture = seeded_refresh(root.path(), Arc::clone(&startup)).await?;
    let session = Arc::clone(&fixture.task.session);
    let pipeline = initialise_in_background(&fixture.task).await?;
    let mut guard = session.lock().await;
    let expected_files = queue_new_file(&mut guard, root.path())?;
    let client = fixture.task.client.clone();
    let mut completion = Box::pin(finish_refresh(fixture.task, Ok(pipeline)));
    assert!(futures::poll!(completion.as_mut()).is_pending());
    assert_initial_running(&client, &startup, &mut fixture.socket).await?;
    drop(guard);
    let ((), committed) = tokio::join!(
        completion,
        assert_committed(&session, &startup, &mut fixture.socket, expected_files)
    );
    committed
}
