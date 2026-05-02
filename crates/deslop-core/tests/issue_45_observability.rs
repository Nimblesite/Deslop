//! Regression coverage for GH#45 pipeline observability.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Context, Result};
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
        &["total", "dropped_below_min_members", "largest_weight"],
    );

    let bucket = captured.event("bucket distribution")?;
    assert_eq!(bucket.target, "deslop_core::report");
    assert_has_fields(
        &bucket,
        &[
            "identical",
            "nearly_identical",
            "loosely_similar",
            "same_behavior",
        ],
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

fn assert_has_fields(event: &CapturedEvent, required: &[&str]) {
    assert!(
        event.has_fields(required),
        "GH#45: event {:?} missing required fields {required:?}",
        event.message(),
    );
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

    fn len(&self) -> usize {
        self.events.lock().map_or(0, |events| events.len())
    }

    fn event(&self, message: &str) -> Result<CapturedEvent> {
        let events = self
            .events
            .lock()
            .map_err(|_| anyhow!("captured events mutex poisoned"))?;
        events
            .iter()
            .find(|event| event.message() == Some(message))
            .cloned()
            .ok_or_else(|| anyhow!("missing event {message:?}; captured: {events:?}"))
    }
}

#[derive(Clone, Debug)]
struct CapturedEvent {
    target: String,
    fields: BTreeSet<String>,
    values: BTreeMap<String, String>,
}

impl CapturedEvent {
    fn message(&self) -> Option<&str> {
        self.values.get("message").map(String::as_str)
    }

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
            target: event.metadata().target().to_owned(),
            fields: visitor.fields,
            values: visitor.values,
        });
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

#[derive(Default)]
struct FieldCollector {
    fields: BTreeSet<String>,
    values: BTreeMap<String, String>,
}

impl FieldCollector {
    fn record_value(&mut self, field: &Field, value: String) {
        let name = field.name().to_owned();
        let _inserted = self.fields.insert(name.clone());
        let _previous = self.values.insert(name, value);
    }
}

impl Visit for FieldCollector {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let raw = format!("{value:?}");
        let normalized = raw
            .strip_prefix('"')
            .and_then(|inner| inner.strip_suffix('"'))
            .unwrap_or(&raw)
            .to_owned();
        self.record_value(field, normalized);
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_value(field, value.to_owned());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record_value(field, value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record_value(field, value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record_value(field, value.to_string());
    }
}
