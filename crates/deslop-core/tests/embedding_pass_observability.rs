//! Regression coverage for GH#94 embedding-pass observability.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use deslop_core::{
    embedding::{EmbeddingCache, EmbeddingMode},
    pipeline::{run, EmbeddingSettings, PipelineConfig},
    EmbeddingProvider, EmbeddingSpec, ProviderError,
};
use tracing::{
    field::{Field, Visit},
    span::{Attributes, Id, Record},
    Event, Level, Metadata, Subscriber,
};

const VECTOR_DIMS: usize = 4;
const SLOW_BATCH: usize = 2;

#[test]
fn issue_94_embedding_pass_emits_batch_observability_events() -> Result<()> {
    let root = tempfile::tempdir().context("tempdir")?;
    write_issue_94_fixture(root.path()).context("write fixture")?;

    let discover_provider = Arc::new(ObservabilityProvider::new("discover", 1_024, None));
    run_pipeline(root.path(), discover_provider.as_ref()).context("discover snippets")?;
    let inputs = discover_provider.inputs()?;
    assert!(
        inputs.len() >= 4,
        "fixture must produce enough unique embedding inputs: {}",
        inputs.len(),
    );

    let provider = Arc::new(ObservabilityProvider::new(
        "observability",
        1,
        Some(SLOW_BATCH),
    ));
    let to_prime = inputs.get(3..).context("prime range")?;
    prime_cache(root.path(), &provider.spec(), to_prime).context("prime cache")?;
    let captured = CapturedEvents::default();
    let subscriber = CaptureSubscriber::new(captured.clone());
    tracing::subscriber::with_default(subscriber, || run_pipeline(root.path(), provider.as_ref()))
        .context("observed pipeline run")?;

    assert_eq!(
        provider.batch_calls(),
        3,
        "expected exactly three provider batches"
    );
    let cache = captured.event("embedding cache phase complete")?;
    assert_eq!(cache.target, "deslop_core::pipeline::embedding_pass");
    assert_field_eq(&cache, "cache_hits", &inputs.len().saturating_sub(3));
    assert_field_eq(&cache, "cache_misses", &3);
    assert_field_eq(&cache, "queued_for_provider", &3);

    let dispatch = captured.events("embedding provider batch dispatch starting")?;
    assert_eq!(
        dispatch.len(),
        3,
        "expected one dispatch event per provider batch"
    );
    let first_dispatch = dispatch.first().context("first dispatch")?;
    assert_field_eq(first_dispatch, "batch_index", &1);
    assert_field_eq(first_dispatch, "total_batches", &3);
    assert_field_eq(first_dispatch, "batch_size", &1);

    let completion = captured.events("embedding provider batch complete")?;
    assert_eq!(
        completion.len(),
        3,
        "expected one completion event per provider batch"
    );
    let first_completion = completion.first().context("first completion")?;
    assert_has_fields(first_completion, &["elapsed_ms", "tokens"]);

    let slow = captured.event("embedding provider batch slow")?;
    assert_eq!(slow.level, "WARN");
    assert_field_eq(&slow, "batch_index", &SLOW_BATCH);
    assert_has_fields(&slow, &["elapsed_ms", "tokens", "batch_size"]);

    let complete = captured.event("embedding pass complete")?;
    assert_has_fields(
        &complete,
        &[
            "cache_hit_pct",
            "provider_p50_ms",
            "provider_p99_ms",
            "total_pass_ms",
        ],
    );
    assert_field_eq(&complete, "provider_batches", &3);
    assert_positive_field(&complete, "total_pass_ms")?;
    Ok(())
}

fn run_pipeline(root: &Path, provider: &dyn EmbeddingProvider) -> Result<()> {
    let report = run(&PipelineConfig {
        root: root.to_path_buf(),
        min_nodes: 2,
        config_path: None,
        embedding: EmbeddingSettings {
            mode: EmbeddingMode::Required,
            provider: Some(provider),
            batch_yield: None,
            progress: None,
        },
        incremental: false,
    })
    .context("pipeline run")?;
    assert!(
        report.embedding_provenance.is_some(),
        "embedding pass should record provenance",
    );
    Ok(())
}

fn prime_cache(root: &Path, spec: &EmbeddingSpec, inputs: &[String]) -> Result<()> {
    let cache = EmbeddingCache::open(&root.join(".deslop-cache"), spec).context("open cache")?;
    for input in inputs {
        cache
            .store(input, &vector_for(input))
            .with_context(|| format!("store cached input with {} chars", input.chars().count()))?;
    }
    Ok(())
}

fn write_issue_94_fixture(root: &Path) -> Result<()> {
    fs::write(root.join("Alpha.cs"), class_source("Alpha", "Add", "+")).context("Alpha.cs")?;
    fs::write(root.join("Beta.cs"), class_source("Beta", "Mul", "*")).context("Beta.cs")?;
    fs::write(root.join("Gamma.cs"), class_source("Gamma", "Sub", "-")).context("Gamma.cs")?;
    Ok(())
}

fn class_source(class_name: &str, method_name: &str, operator: &str) -> String {
    format!(
        "namespace Issue94 {{ public class {class_name} {{ public int {method_name}(int seed) {{ var total = seed; total = total {operator} 2; if (total > 10) {{ total = total - 1; }} return total; }} }} }}\n"
    )
}

#[derive(Debug)]
struct ObservabilityProvider {
    model_version: &'static str,
    max_batch_size: usize,
    slow_batch: Option<usize>,
    batch_calls: AtomicUsize,
    inputs: Mutex<Vec<String>>,
}

impl ObservabilityProvider {
    fn new(model_version: &'static str, max_batch_size: usize, slow_batch: Option<usize>) -> Self {
        Self {
            model_version,
            max_batch_size,
            slow_batch,
            batch_calls: AtomicUsize::new(0),
            inputs: Mutex::new(Vec::new()),
        }
    }

    fn batch_calls(&self) -> usize {
        self.batch_calls.load(Ordering::SeqCst)
    }

    fn inputs(&self) -> Result<Vec<String>> {
        self.inputs
            .lock()
            .map(|inputs| inputs.clone())
            .map_err(|_| anyhow!("input lock poisoned"))
    }
}

impl EmbeddingProvider for ObservabilityProvider {
    fn spec(&self) -> EmbeddingSpec {
        EmbeddingSpec {
            provider_id: "issue-94".to_owned(),
            model_id: "observability-fixture".to_owned(),
            model_version: self.model_version.to_owned(),
            dimensions: VECTOR_DIMS,
        }
    }

    fn probe(&self) -> Result<(), ProviderError> {
        Ok(())
    }

    fn embed(&self, input: &str) -> Result<Vec<f32>, ProviderError> {
        Ok(vector_for(input))
    }

    fn max_batch_size(&self) -> usize {
        self.max_batch_size
    }

    fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, ProviderError> {
        let call = self
            .batch_calls
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        if self.slow_batch == Some(call) {
            thread::sleep(Duration::from_millis(2_050));
        }
        let mut recorded = self.inputs.lock().map_err(|_| ProviderError::Malformed {
            provider_id: "issue-94".to_owned(),
            message: "input lock poisoned".to_owned(),
        })?;
        recorded.extend(inputs.iter().cloned());
        Ok(inputs.iter().map(|input| vector_for(input)).collect())
    }
}

fn vector_for(input: &str) -> Vec<f32> {
    let length = u16::try_from(input.chars().count()).unwrap_or(u16::MAX);
    let length = f32::from(length);
    vec![1.0, length, length.rem_euclid(7.0), 0.5]
}

fn assert_has_fields(event: &CapturedEvent, required: &[&str]) {
    assert!(
        event.has_fields(required),
        "GH#94: event {:?} missing required fields {required:?}",
        event.message(),
    );
}

fn assert_field_eq<T>(event: &CapturedEvent, field: &str, expected: &T)
where
    T: ToString + ?Sized,
{
    let expected_string = expected.to_string();
    assert_eq!(
        event.values.get(field).map(String::as_str),
        Some(expected_string.as_str()),
        "GH#94: event {:?} has wrong {field}",
        event.message(),
    );
}

fn assert_positive_field(event: &CapturedEvent, field: &str) -> Result<()> {
    let value = event
        .values
        .get(field)
        .ok_or_else(|| anyhow!("missing field {field}"))?
        .parse::<u64>()
        .with_context(|| format!("parse field {field}"))?;
    assert!(value > 0, "GH#94: {field} should be positive");
    Ok(())
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

    fn event(&self, message: &str) -> Result<CapturedEvent> {
        self.events(message)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("missing event {message:?}; captured: {:?}", self.all()))
    }

    fn events(&self, message: &str) -> Result<Vec<CapturedEvent>> {
        let events = self
            .events
            .lock()
            .map_err(|_| anyhow!("captured events mutex poisoned"))?;
        Ok(events
            .iter()
            .filter(|event| event.message() == Some(message))
            .cloned()
            .collect())
    }

    fn all(&self) -> Vec<CapturedEvent> {
        self.events
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
struct CapturedEvent {
    target: String,
    level: String,
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
            level: level_name(*event.metadata().level()),
            fields: visitor.fields,
            values: visitor.values,
        });
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

fn level_name(level: Level) -> String {
    level.as_str().to_owned()
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
