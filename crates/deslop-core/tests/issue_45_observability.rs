//! Regression coverage for GH#45 pipeline observability.

use std::fs;

use anyhow::{Context, Result};
use deslop_core::{
    pipeline::{run, EmbeddingSettings, PipelineConfig},
    EmbeddingMode,
};

use crate::common::*;

#[test]
fn issue_45_pipeline_emits_stage_observability_events() -> Result<()> {
    let captured = CapturedEvents::default();
    let subscriber = CaptureSubscriber::new(captured.clone());
    tracing::subscriber::with_default(subscriber, run_pipeline)?;

    assert!(
        captured.len() >= 3,
        "GH#45: expected at least three pipeline observability events: {captured:?}",
    );

    let pair = captured.event("pair survival outcome")?;
    assert_eq!(pair.target, "deslop_core::pair");
    assert_has_fields(
        &pair,
        &[
            "survived",
            "dropped_below_fused",
            "dropped_lsh_only_jaccard",
            "dropped_lsh_only_node_count",
        ],
    );

    let cluster = captured.event("ranked clusters built")?;
    assert_eq!(cluster.target, "deslop_core::cluster");
    assert_has_fields(
        &cluster,
        &["total", "dropped_below_min_members", "largest_mass"],
    );

    // The bucket distribution event is retired with the bucket surface:
    // the mass-only report emits its own visibility telemetry instead.
    let report = captured.event("mass-ranked report built")?;
    assert_eq!(report.target, "deslop_core::report");
    assert_has_fields(
        &report,
        &["visible_clusters", "clusters_hidden", "highest_mass"],
    );
    Ok(())
}

fn run_pipeline() -> Result<()> {
    let root = tempfile::tempdir().context("tempdir")?;
    let src = root.path().join("src");
    fs::create_dir_all(&src).context("create fixture src dir")?;
    fs::write(src.join("Alpha.cs"), OBSERVABILITY_ALPHA).context("write Alpha.cs")?;
    fs::write(src.join("Beta.cs"), OBSERVABILITY_BETA).context("write Beta.cs")?;

    let report = run(&PipelineConfig {
        root: root.path().to_path_buf(),
        min_nodes: 15,
        config_path: None,
        embedding: EmbeddingSettings {
            mode: EmbeddingMode::Off,
            provider: None,
            batch_yield: None,
            progress: None,
        },
        incremental: false,
    })
    .context("pipeline run")?;
    assert!(
        !report.clusters.is_empty(),
        "fixture must produce clusters so stage logging is meaningful",
    );
    Ok(())
}

fn assert_has_fields(event: &CapturedEvent, required: &[&str]) {
    assert!(
        event.has_fields(required),
        "GH#45: event {:?} missing required fields {required:?}",
        event.message(),
    );
}

const OBSERVABILITY_ALPHA: &str = r"
namespace Observability;

public sealed class AlphaPipelineProbe
{
    public int Compute(int input)
    {
        if (input < 0) { return 0; }
        int total = 0;
        for (int i = 0; i < input; i = i + 1) { total = total + i; }
        return total;
    }
}
";

const OBSERVABILITY_BETA: &str = r"
namespace Observability;

public sealed class BetaPipelineProbe
{
    public int Run(int limit)
    {
        if (limit < 0) { return 0; }
        int acc = 0;
        for (int j = 0; j < limit; j = j + 1) { acc = acc + j; }
        return acc;
    }
}
";
