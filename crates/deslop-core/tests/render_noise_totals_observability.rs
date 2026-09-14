//! Regression coverage for the render-stage cluster-noise observability
//! gap behind the gh #434 misdiagnosis.
//!
//! The cluster-noise counters live on the run-shared `ParseCache`, but
//! they were emitted only from the noise-split pass. Every filter the
//! *render* pass ran — the pass that actually hides a cluster — was
//! recorded into the same counters and discarded unread, so a reader
//! saw a partial total presented as the whole run's and twice concluded
//! the filters had never examined a cluster they had in fact examined.
//!
//! Tests [PERF-FLUTTER-TODO-OBSERVABILITY].

use std::{fs, path::Path};

use anyhow::{anyhow, Context, Result};
use deslop_core::{
    pipeline::{run, EmbeddingSettings, PipelineConfig},
    report::Report,
    EmbeddingMode,
};

use crate::common::*;

/// Message shared by every cluster-noise totals record.
const NOISE_TOTALS_MESSAGE: &str = "cluster noise filter totals";
/// Stage label the noise-split pass stamps on its own totals.
const SPLIT_STAGE: &str = "noise_verbatim_split";
/// Stage label stamped on the totals emitted once the render pass has
/// finished convicting. Named a run cumulative, not a stage, because the
/// shared counters hold the split pass *plus* the render pass by then.
const RUN_CUMULATIVE_STAGE: &str = "run_cumulative_after_report_render";
/// The filter that convicts pytest fixture row builders
/// ([CLONE-NOISE-PY-PYTEST-FIXTURE]).
const FIXTURE_FILTER: &str = "language_specific";
/// `min_nodes` small enough that the noise split runs the filters too,
/// so both records exist and can be compared.
const MIN_NODES_SPLIT_ACTIVE: u32 = 4;
/// `min_nodes` at which the noise split runs no filter at all and the
/// render pass is the only stage that convicts.
const MIN_NODES_RENDER_ONLY: u32 = 15;
/// The fixture's one pytest fixture family, hidden as setup boilerplate.
const HIDDEN_FIXTURE_CLUSTERS: usize = 1;
/// Members in that family — one row builder per fixture file.
const FIXTURE_MEMBERS: u64 = 3;

#[test]
fn render_stage_noise_convictions_reach_the_emitted_totals() -> Result<()> {
    // Interaction 1 — a run where the split pass may also exercise the
    // filters. Either way the run-cumulative record must carry the
    // render-stage conviction that used to be discarded: the fixture
    // family is hidden at render, and the emitted totals must show it.
    let both = capture_run(MIN_NODES_SPLIT_ACTIVE)?;
    assert_eq!(
        both.report.clusters_hidden, HIDDEN_FIXTURE_CLUSTERS,
        "the pytest fixture family must be convicted at render, or this test proves nothing",
    );
    let cumulative = totals(&both.captured, RUN_CUMULATIVE_STAGE, FIXTURE_FILTER)?;
    assert_eq!(cumulative.target, "deslop_core::cluster_filters::snippets");
    assert!(
        field(&cumulative, "fired")? >= 1,
        "the render-stage conviction must reach the run-cumulative totals",
    );
    assert_eq!(
        field(&cumulative, "members")?,
        FIXTURE_MEMBERS,
        "the emitted totals must carry the convicted family's member count",
    );

    // Interaction 2 — the shape that produced the misdiagnosis. The
    // split pass runs no filter, so before the fix a run that hid a
    // cluster emitted no noise record whatsoever.
    let render_only = capture_run(MIN_NODES_RENDER_ONLY)?;
    assert_eq!(
        render_only.report.clusters_hidden, HIDDEN_FIXTURE_CLUSTERS,
        "the render pass must still convict the fixture family",
    );
    assert!(
        stage_records(&render_only.captured, SPLIT_STAGE)?.is_empty(),
        "this run's split pass must run no filter, leaving the render pass the only source",
    );
    let only = totals(&render_only.captured, RUN_CUMULATIVE_STAGE, FIXTURE_FILTER)?;
    assert_eq!(
        field(&only, "fired")?,
        HIDDEN_FIXTURE_CLUSTERS as u64,
        "the conviction that hid the cluster must appear in the emitted totals",
    );
    assert_eq!(
        field(&only, "members")?,
        FIXTURE_MEMBERS,
        "the emitted totals must carry the convicted family's member count",
    );

    // Every filter consulted is reported, not just the one that fired —
    // a reader must be able to tell "examined and declined" from
    // "never examined", which is exactly the distinction gh #434 lost.
    let reported = stage_records(&render_only.captured, RUN_CUMULATIVE_STAGE)?;
    assert!(
        reported.len() > HIDDEN_FIXTURE_CLUSTERS,
        "every consulted filter must be reported, not only the one that fired: {reported:?}",
    );
    assert!(
        reported
            .iter()
            .filter_map(|event| event.values.get("filter"))
            .any(|filter| filter != FIXTURE_FILTER),
        "a filter that examined the cluster and declined must still be reported",
    );
    Ok(())
}

/// One observed pipeline run: the report it produced and every
/// `deslop_core` event it emitted.
struct CapturedRun {
    /// The rendered report, for the conviction count.
    report: Report,
    /// Events captured for the whole run.
    captured: CapturedEvents,
}

/// Runs the fixture through the production pipeline under a capturing
/// subscriber at `min_nodes`.
fn capture_run(min_nodes: u32) -> Result<CapturedRun> {
    let root = tempfile::tempdir().context("tempdir")?;
    write_fixture(root.path()).context("write fixture")?;
    let captured = CapturedEvents::default();
    let subscriber = CaptureSubscriber::new(captured.clone());
    let report =
        tracing::subscriber::with_default(subscriber, || run_pipeline(root.path(), min_nodes))
            .context("observed pipeline run")?;
    Ok(CapturedRun { report, captured })
}

/// Analyses `root` with embeddings off, so the run is deterministic.
fn run_pipeline(root: &Path, min_nodes: u32) -> Result<Report> {
    run(&PipelineConfig {
        root: root.to_path_buf(),
        min_nodes,
        config_path: None,
        embedding: EmbeddingSettings {
            mode: EmbeddingMode::Off,
            provider: None,
            batch_yield: None,
            progress: None,
        },
        incremental: false,
    })
    .context("pipeline run")
}

/// Every noise-totals record carrying `stage`.
fn stage_records(captured: &CapturedEvents, stage: &str) -> Result<Vec<CapturedEvent>> {
    Ok(captured
        .events(NOISE_TOTALS_MESSAGE)?
        .into_iter()
        .filter(|event| event.values.get("stage").map(String::as_str) == Some(stage))
        .collect())
}

/// The one noise-totals record for `stage` and `filter`.
fn totals(captured: &CapturedEvents, stage: &str, filter: &str) -> Result<CapturedEvent> {
    stage_records(captured, stage)?
        .into_iter()
        .find(|event| event.values.get("filter").map(String::as_str) == Some(filter))
        .ok_or_else(|| {
            anyhow!(
                "no {NOISE_TOTALS_MESSAGE} record for stage {stage:?} filter {filter:?}; captured: {:?}",
                captured.events(NOISE_TOTALS_MESSAGE),
            )
        })
}

/// Reads one numeric counter off a totals record.
fn field(event: &CapturedEvent, name: &str) -> Result<u64> {
    event
        .values
        .get(name)
        .ok_or_else(|| anyhow!("totals record has no {name}: {event:?}"))?
        .parse()
        .with_context(|| format!("parse {name}"))
}

/// One pytest fixture row builder per file — the [CLONE-NOISE-PY-PYTEST-FIXTURE]
/// family the render pass convicts as setup boilerplate.
fn write_fixture(root: &Path) -> Result<()> {
    for (name, source) in [
        ("test_conversations.py", FIXTURE_CONVERSATIONS),
        ("test_messages.py", FIXTURE_MESSAGES),
        ("test_runs.py", FIXTURE_RUNS),
    ] {
        fs::write(root.join(name), source).with_context(|| format!("write {name}"))?;
    }
    Ok(())
}

const FIXTURE_CONVERSATIONS: &str = r#"
import uuid
import pytest


@pytest.fixture
async def conversation(db_session, tenant):
    row = Conversation(id=uuid.uuid4(), tenant_id=tenant.id, title="chat")
    db_session.add(row)
    await db_session.commit()
    await db_session.refresh(row)
    return row
"#;

const FIXTURE_MESSAGES: &str = r#"
import uuid
import pytest


@pytest.fixture
async def message(db_session, tenant):
    row = Message(id=uuid.uuid4(), tenant_id=tenant.id, body="hello")
    db_session.add(row)
    await db_session.commit()
    await db_session.refresh(row)
    return row
"#;

const FIXTURE_RUNS: &str = r#"
import uuid
import pytest


@pytest.fixture
async def run(db_session, tenant):
    row = AgentRun(id=uuid.uuid4(), tenant_id=tenant.id, status="queued")
    db_session.add(row)
    await db_session.commit()
    await db_session.refresh(row)
    return row
"#;
