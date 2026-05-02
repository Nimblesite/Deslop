//! Regression coverage for GH#45 pipeline observability.

use std::{collections::BTreeSet, fmt, path::{Path, PathBuf}, sync::{Arc, Mutex}};

use anyhow::{Context, Result};
use deslop_core::{
    pipeline::{run, EmbeddingSettings, PipelineConfig},
    EmbeddingMode,
};
use tracing::{
    field::{Field, Visit},
    span::{Attributes, Id, Record},
    Event, Metadata, Subscriber,
};

#[test]
fn issue_45_pipeline_emits_stage_observability_events() -> Result<()> {
    let captured = CapturedEvents::default();
    let subscriber = CaptureSubscriber::new(captured.clone());
    tracing::subscriber::with_default(subscriber, || run_pipeline())?;

    assert!(
        captured.contains_fields(&[
            "survived",
            "dropped_below_fused",
            "dropped_lsh_only_jaccard",
            "dropped_lsh_only_node_count",
        ]),
        "GH#45: pair survival stage must log structured outcome counts: {captured:?}",
    );
    assert!(
        captured.contains_fields(&["total", "dropped_below_min_members", "largest_weight"]),
        "GH#45: cluster distribution stage must log structured rank/build counts: {captured:?}",
    );
    assert!(
        captured.contains_fields(&[
            "identical",
            "nearly_identical",
            "loosely_similar",
            "same_behavior",
        ]),
        "GH#45: bucket classification stage must log structured bucket counts: {captured:?}",
    );
    Ok(())
}

fn run_pipeline() -> Result<()> {
    let report = run(&PipelineConfig {
        root: fixture("csharp-small"),
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

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("deslop")
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[derive(Clone, Debug, Default)]
struct CapturedEvents {
    events: Arc<Mutex<Vec<CapturedEvent>>>,
}

impl CapturedEvents {
    fn push(&self, event: CapturedEvent) {
        if let Ok(mut events) = self.events.lock() {
            events.push(event);
        }
    }

    fn contains_fields(&self, required: &[&str]) -> bool {
        let Ok(events) = self.events.lock() else {
            return false;
        };
        events.iter().any(|event| event.has_fields(required))
    }
}

#[derive(Debug)]
struct CapturedEvent {
    fields: BTreeSet<String>,
}

impl CapturedEvent {
    fn has_fields(&self, required: &[&str]) -> bool {
        required.iter().all(|field| self.fields.contains(*field))
    }
}

#[derive(Debug)]
struct CaptureSubscriber {
    captured: CapturedEvents,
}

impl CaptureSubscriber {
    fn new(captured: CapturedEvents) -> Self {
        Self { captured }
    }
}

impl Subscriber for CaptureSubscriber {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.target().starts_with("deslop_core")
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut visitor = FieldCollector::default();
        event.record(&mut visitor);
        self.captured.push(CapturedEvent {
            fields: visitor.fields,
        });
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

#[derive(Default)]
struct FieldCollector {
    fields: BTreeSet<String>,
}

impl Visit for FieldCollector {
    fn record_debug(&mut self, field: &Field, _value: &dyn fmt::Debug) {
        let _inserted = self.fields.insert(field.name().to_owned());
    }
}
