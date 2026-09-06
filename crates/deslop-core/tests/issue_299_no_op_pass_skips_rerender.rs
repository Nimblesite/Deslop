//! Regression tests for #299 ([LIVE-SCHEDULER-NOOP], [LIVE-SCHEDULER],
//! [LIVE-WATCHER]).
//!
//! `PipelineSession::update_files` ran the full LSH → embedding →
//! candidate-pair → transitive-closure → rank → render pipeline over the
//! *entire* corpus on every pass, including passes where every changed path
//! was filtered out before it could touch a single analysed file. The parse
//! stage honoured the incremental cache; everything downstream did not.
//!
//! In production one `deslop-lsp` instance burned **11h17m of CPU across
//! 1086 passes** on a 172-file workspace, 139 of 157 logged passes reporting
//! `0 added, 0 removed, 0 updated` — a byte-identical report, re-derived from
//! 422,711 fingerprints and 366,765 candidate pairs, roughly every 30
//! seconds. Each one also bumped the generation and broadcast
//! `report/changed`, so every subscriber re-fetched a report that had not
//! changed.
//!
//! #314 is the same wound one layer up: even after the render early-out, the
//! scheduler still announced every pass, so a no-op pass sent `report/changed`
//! and each subscriber answered with `reportDelta` → `reportGet` over an
//! identical report. Two hours of build churn produced 281 such `reportGet`
//! calls against a 262,000-fingerprint corpus.
//!
//! Black-box only: drives the public [`AnalysisSession`] and [`Scheduler`]
//! surfaces, exactly as the watcher, the freshness tracker, and an LSP
//! `didSave` feed them.

#![cfg(feature = "live")]

use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use anyhow::{bail, Context, Result};
use deslop_core::live::{AnalysisState, Clock, Scheduler, CAP_MS};
use tokio::sync::broadcast::{error::TryRecvError, Receiver};

mod common;
use crate::common::*;

/// C# that clusters with the fixture's `Alpha.cs`, so if any gate were to
/// wrongly admit one of the inert paths the corpus would visibly grow rather
/// than failing silently.
const DUPLICATE_SOURCE: &str = r"namespace Vendored
{
    public class Processor
    {
        public int Compute(int input)
        {
            if (input < 0)
            {
                return 0;
            }
            int total = 0;
            for (int index = 0; index < input; index = index + 1)
            {
                total = total + index;
            }
            return total;
        }
    }
}
";

/// Builds a workspace whose every extra path is *inert* — it can generate
/// filesystem events forever without ever entering the corpus. One path per
/// gate in `apply_one_change`, so a regression in any single gate is caught:
///
/// * `build.log` — unsupported extension.
/// * `excluded/Skipped.cs` — matched by `.deslop.toml` `exclude`.
/// * `ignored/Skipped.cs` — matched by an `.ignore` rule.
/// * `Ghost.cs` — never created; the deletion/temp-file event for a path the
///   corpus never held (editor atomic saves produce these constantly).
///
/// Returns the temp dir alongside the inert paths and the subset of them that
/// exists on disk and can therefore be rewritten between rounds.
fn workspace_with_inert_paths() -> Result<(tempfile::TempDir, Vec<PathBuf>, Vec<PathBuf>)> {
    let tmp = copy_fixture("csharp-small")?;
    let root = tmp.path();

    fs::write(
        root.join(".deslop.toml"),
        "[defaults]\nexclude = [\"excluded/**\"]\n",
    )
    .context("write .deslop.toml")?;
    // `.ignore` is honoured with no repository present, unlike `.gitignore`.
    fs::write(root.join(".ignore"), "/ignored/\n").context("write .ignore")?;

    let build_log = root.join("build.log");
    fs::write(&build_log, "linking...\n").context("write build.log")?;
    let mut on_disk = vec![build_log];
    for dir in ["excluded", "ignored"] {
        let target = root.join(dir);
        fs::create_dir_all(&target).with_context(|| format!("mkdir {dir}"))?;
        let file = target.join("Skipped.cs");
        fs::write(&file, DUPLICATE_SOURCE).with_context(|| format!("write {dir}/Skipped.cs"))?;
        on_disk.push(file);
    }

    let mut inert = on_disk.clone();
    inert.push(root.join("Ghost.cs"));
    Ok((tmp, inert, on_disk))
}

/// The core regression: a pass that mutates no analysed file must not publish
/// a new report generation, and must not disturb the report it kept.
///
/// The generation is the observable proxy for the wasted work — it is bumped
/// in the same breath as the render, so a stable generation across repeated
/// inert passes is exactly the property that was missing in production. Five
/// rounds, because the production failure was a *sustained* stream of inert
/// events, not one.
#[tokio::test(flavor = "multi_thread")]
async fn inert_paths_never_publish_a_new_generation() -> Result<()> {
    let (tmp, inert, on_disk) = workspace_with_inert_paths()?;
    let mut session = live_session(tmp.path())?;

    let baseline_generation = session.generation();
    let baseline = session.report();
    assert_eq!(
        baseline.files_analysed, 2,
        "baseline corpus must be exactly Alpha.cs and Beta.cs; the excluded, \
         ignored and unsupported paths must never be discovered. \
         files_analysed={}",
        baseline.files_analysed,
    );
    assert!(
        !baseline.clusters.is_empty(),
        "fixture must produce at least one cluster, or a preserved report \
         proves nothing",
    );

    for round in 1..=5 {
        for path in &on_disk {
            fs::write(path, format!("{DUPLICATE_SOURCE}// round {round}\n"))
                .with_context(|| format!("rewrite round {round}"))?;
        }
        let delta = session
            .apply_changes(&inert)
            .with_context(|| format!("apply round {round}"))?;

        assert_eq!(
            session.generation(),
            baseline_generation,
            "round {round}: a pass that mutates no analysed file must reuse \
             the previous report, not re-render and publish generation {}",
            session.generation(),
        );
        assert!(
            delta.clusters_added.is_empty()
                && delta.clusters_removed.is_empty()
                && delta.clusters_updated.is_empty(),
            "round {round}: inert paths must yield an empty delta, got {delta:?}",
        );
    }

    let after = session.report();
    assert_eq!(
        after.files_analysed, baseline.files_analysed,
        "the preserved report must still describe the original corpus",
    );
    assert_eq!(
        after.clusters.len(),
        baseline.clusters.len(),
        "skipping the render must preserve the clusters, not drop them",
    );
    for component in ["excluded", "ignored"] {
        assert_eq!(
            occurrences_with_component(&after, component),
            0,
            "no occurrence may appear under `{component}`: {:?}",
            after.clusters,
        );
    }
    Ok(())
}

/// Issue #314: duplicate editor/watcher delivery for an unchanged analysed
/// file must not run the whole-corpus render or publish a new generation.
#[tokio::test(flavor = "multi_thread")]
async fn unchanged_analysed_file_does_not_publish_new_generation() -> Result<()> {
    let tmp = copy_fixture("csharp-small")?;
    let mut session = live_session(tmp.path())?;
    let unchanged = tmp.path().join("Alpha.cs");
    let baseline_generation = session.generation();

    let _delta = session
        .apply_changes(std::slice::from_ref(&unchanged))
        .context("apply unchanged analysed file")?;

    assert_eq!(
        session.generation(),
        baseline_generation,
        "unchanged analysed bytes must reuse the current report instead of re-rendering",
    );
    Ok(())
}

/// The control. A fix that simply stopped bumping the generation would pass
/// the test above and break the entire live loop, so prove the same session
/// still publishes when an analysed file genuinely changes — and that it does
/// so *from* the generation the inert rounds left untouched.
#[tokio::test(flavor = "multi_thread")]
async fn a_real_edit_still_publishes_after_inert_rounds() -> Result<()> {
    let (tmp, inert, _on_disk) = workspace_with_inert_paths()?;
    let mut session = live_session(tmp.path())?;

    let baseline_generation = session.generation();
    let _inert_delta = session.apply_changes(&inert).context("apply inert")?;
    assert_eq!(
        session.generation(),
        baseline_generation,
        "inert pass must leave the generation alone",
    );

    let edited = tmp.path().join("Beta.cs");
    fs::write(
        &edited,
        b"namespace Beta { public class Differ { public int Run(int x) { return x + 1; } } }\n",
    )
    .context("write Beta")?;
    let delta = session
        .apply_changes(std::slice::from_ref(&edited))
        .context("apply edit")?;

    assert_eq!(
        session.generation(),
        baseline_generation.saturating_add(1),
        "a real edit must publish exactly one new generation",
    );
    assert!(
        !(delta.clusters_added.is_empty()
            && delta.clusters_removed.is_empty()
            && delta.clusters_updated.is_empty()),
        "a real edit must produce a non-empty delta, got {delta:?}",
    );
    Ok(())
}

/// Hands out a timestamp a full [`CAP_MS`] beyond the previous one, so the
/// debouncer reports ready the first time the scheduler ticks. Keeps the
/// scheduler tests free of `sleep` and wall-clock timing.
#[derive(Debug, Default)]
struct RunawayClock {
    /// Monotonic counter; each read advances it by one debounce cap.
    elapsed_ms: AtomicU64,
}

impl Clock for RunawayClock {
    fn now_ms(&self) -> u64 {
        self.elapsed_ms.fetch_add(CAP_MS, Ordering::SeqCst)
    }
}

/// Blocks until the scheduler reports the pass finished. `Running` is the
/// leading edge of the same pass, so it is skipped; `Errored` fails the test
/// rather than hanging on a state that will never arrive.
async fn await_pass_complete(state_rx: &mut Receiver<AnalysisState>) -> Result<()> {
    loop {
        match state_rx.recv().await.context("analysis/state closed")? {
            AnalysisState::Idle => return Ok(()),
            AnalysisState::Errored { message } => bail!("scheduler pass failed: {message}"),
            AnalysisState::Running { .. } => {}
        }
    }
}

/// Issue #314: the scheduler must stay silent when a pass leaves the
/// generation where it was. `analysis/state` still reports the pass, because
/// the panel's spinner is driven by it and costs one enum on the wire; what
/// must not go out is the `report/changed` that makes every subscriber
/// re-fetch a report it already holds.
#[tokio::test(flavor = "multi_thread")]
async fn no_op_pass_broadcasts_no_report_changed() -> Result<()> {
    let tmp = copy_fixture("csharp-small")?;
    let session = Arc::new(tokio::sync::Mutex::new(live_session(tmp.path())?));
    let baseline_generation = session.lock().await.generation();
    let (events_tx, events_rx) = tokio::sync::mpsc::channel(8);
    let scheduler = Scheduler::start(
        Arc::clone(&session),
        events_rx,
        Arc::new(RunawayClock::default()),
    );
    let mut report_rx = scheduler.subscribe_report_changed();
    let mut state_rx = scheduler.subscribe_state();

    let path = tmp.path().join("Alpha.cs");
    events_tx
        .send(path.clone())
        .await
        .context("queue unchanged path")?;
    await_pass_complete(&mut state_rx).await?;
    assert!(
        matches!(report_rx.try_recv(), Err(TryRecvError::Empty)),
        "a pass over an unchanged analysed file must broadcast nothing — the \
         report is the object every subscriber already holds (#314)",
    );
    assert_eq!(
        session.lock().await.generation(),
        baseline_generation,
        "the no-op pass must leave the generation alone",
    );

    // The control. Suppressing every broadcast would pass the assertion
    // above and silence the live loop, so drive a real edit through the
    // same scheduler and require it to reach the same subscriber.
    fs::write(
        &path,
        b"namespace Alpha { public class Differ { public int Run(int x) { return x + 2; } } }\n",
    )
    .context("write Alpha")?;
    events_tx.send(path).await.context("queue real edit")?;
    await_pass_complete(&mut state_rx).await?;
    let published = report_rx
        .try_recv()
        .context("a real edit must broadcast report/changed")?;
    assert_eq!(
        published.generation,
        baseline_generation.saturating_add(1),
        "the edit must publish exactly one generation past the baseline",
    );
    assert_eq!(
        published.summary.clusters_removed, 1,
        "rewriting one half of the fixture's only clone pair must report the \
         cluster as removed, got {:?}",
        published.summary,
    );
    Ok(())
}

/// Issue #314, the trap the naive fix falls into. Every read through
/// `LiveApi` calls `refresh_if_stale`, which ingests edits and advances the
/// generation *without* broadcasting — so on a busy editor the read path
/// routinely wins the race against the watcher. A scheduler that asked "did
/// my own pass change anything" would then find nothing to do and stay
/// silent, and the panel would sit on a stale report forever. Ask instead
/// whether subscribers have heard about this generation.
#[tokio::test(flavor = "multi_thread")]
async fn generation_advanced_out_of_band_still_reaches_subscribers() -> Result<()> {
    let tmp = copy_fixture("csharp-small")?;
    let session = Arc::new(tokio::sync::Mutex::new(live_session(tmp.path())?));
    let baseline_generation = session.lock().await.generation();
    let (events_tx, events_rx) = tokio::sync::mpsc::channel(8);
    let scheduler = Scheduler::start(
        Arc::clone(&session),
        events_rx,
        Arc::new(RunawayClock::default()),
    );
    let mut report_rx = scheduler.subscribe_report_changed();
    let mut state_rx = scheduler.subscribe_state();

    // One quiet pass first, so the scheduler has definitely recorded the
    // baseline generation before the race below is set up.
    let path = tmp.path().join("Alpha.cs");
    events_tx
        .send(path.clone())
        .await
        .context("queue warm-up event")?;
    await_pass_complete(&mut state_rx).await?;

    // The read path beats the watcher: the edit is already ingested, and
    // its generation was published to nobody.
    fs::write(
        &path,
        b"namespace Alpha { public class Differ { public int Run(int x) { return x + 3; } } }\n",
    )
    .context("write Alpha")?;
    let mut guard = session.lock().await;
    let _delta = guard
        .apply_changes(std::slice::from_ref(&path))
        .context("out-of-band ingest")?;
    let silent_generation = guard.generation();
    drop(guard);
    assert_eq!(
        silent_generation,
        baseline_generation.saturating_add(1),
        "the out-of-band ingest must advance the generation",
    );

    // The watcher event for the same edit now arrives. Its bytes are
    // already in the corpus, so the pass itself changes nothing — and the
    // announcement still has to go out.
    events_tx.send(path).await.context("queue watcher event")?;
    await_pass_complete(&mut state_rx).await?;
    let published = report_rx
        .try_recv()
        .context("a generation nobody has heard about must still be broadcast")?;
    assert_eq!(
        published.generation, silent_generation,
        "the broadcast must carry the generation the silent ingest produced",
    );
    assert_eq!(
        session.lock().await.generation(),
        silent_generation,
        "re-reading identical bytes must not advance the generation again",
    );
    assert!(
        matches!(report_rx.try_recv(), Err(TryRecvError::Empty)),
        "exactly one announcement — the pass must not also emit a second",
    );
    Ok(())
}

/// A deletion of a file the corpus *does* hold is a real mutation and must
/// still re-render — the early-out keys off whether the corpus changed, never
/// off whether the path still exists.
#[tokio::test(flavor = "multi_thread")]
async fn deleting_an_analysed_file_still_publishes() -> Result<()> {
    let (tmp, _inert, _on_disk) = workspace_with_inert_paths()?;
    let mut session = live_session(tmp.path())?;

    let baseline_generation = session.generation();
    let doomed = tmp.path().join("Beta.cs");
    fs::remove_file(&doomed).context("remove Beta")?;
    let _delta = session
        .apply_changes(std::slice::from_ref(&doomed))
        .context("apply deletion")?;

    assert_eq!(
        session.generation(),
        baseline_generation.saturating_add(1),
        "removing an analysed file must publish a new generation",
    );
    assert_eq!(
        session.report().files_analysed,
        1,
        "the deleted file must leave the corpus",
    );
    Ok(())
}
