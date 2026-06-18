//! Shared fixture-copy helpers for the live-feature E2E suites.
//!
//! Each integration test file is a separate binary, so cross-file reuse
//! is wired via `mod common;` declarations rather than `pub use`. This
//! keeps the per-test files small and prevents byte-identical fixture
//! plumbing from being duplicated across the `live`-gated suites
//! (`live.rs`, `issue_117.rs`).
//!
//! A test binary that pulls in only a subset of these helpers is fine,
//! so the unused-symbol lint is silenced for this module.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use tracing::{
    field::{Field, Visit},
    span::{Attributes, Id, Record},
    Event, Level, Metadata, Subscriber,
};

/// Returns the absolute fixture path used by the CLI tests.
pub(crate) fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("deslop")
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Copies the fixture tree into a temp dir so destructive edits never
/// pollute the source repo.
pub(crate) fn copy_fixture(name: &str) -> Result<tempfile::TempDir> {
    let src = fixture(name);
    let dir = tempfile::tempdir().context("tempdir")?;
    copy_recursive(&src, dir.path())?;
    Ok(dir)
}

/// Recursively copies `src` into `dst`, creating directories as needed.
pub(crate) fn copy_recursive(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst).with_context(|| format!("mkdir {}", dst.display()))?;
        for entry in fs::read_dir(src).with_context(|| format!("read_dir {}", src.display()))? {
            let entry = entry.context("dir entry")?;
            let target = dst.join(entry.file_name());
            copy_recursive(&entry.path(), &target)?;
        }
    } else {
        let _bytes = fs::copy(src, dst).with_context(|| format!("copy {}", src.display()))?;
    }
    Ok(())
}

/// Thread-safe accumulator of `tracing` events captured during a pipeline run.
#[derive(Clone, Debug, Default)]
pub(crate) struct CapturedEvents {
    pub(crate) events: Arc<Mutex<Vec<CapturedEvent>>>,
}

impl CapturedEvents {
    /// Records a single captured event, ignoring a poisoned mutex.
    pub(crate) fn push(&self, event: CapturedEvent) {
        if let Ok(mut events) = self.events.lock() {
            events.push(event);
        }
    }

    /// Returns the number of captured events (`0` if the mutex is poisoned).
    pub(crate) fn len(&self) -> usize {
        self.events.lock().map_or(0, |events| events.len())
    }

    /// Returns the first event whose message equals `message`.
    pub(crate) fn event(&self, message: &str) -> Result<CapturedEvent> {
        self.events(message)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("missing event {message:?}; captured: {:?}", self.all()))
    }

    /// Returns every captured event whose message equals `message`.
    pub(crate) fn events(&self, message: &str) -> Result<Vec<CapturedEvent>> {
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

    /// Returns a snapshot of every captured event in capture order.
    pub(crate) fn all(&self) -> Vec<CapturedEvent> {
        self.events
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default()
    }
}

/// A single `tracing` event reduced to the fields the suites assert against.
#[derive(Clone, Debug)]
pub(crate) struct CapturedEvent {
    pub(crate) target: String,
    pub(crate) level: String,
    pub(crate) fields: BTreeSet<String>,
    pub(crate) values: BTreeMap<String, String>,
}

impl CapturedEvent {
    /// Returns the event's `message` field, if present.
    pub(crate) fn message(&self) -> Option<&str> {
        self.values.get("message").map(String::as_str)
    }

    /// Reports whether every name in `required` was recorded on this event.
    pub(crate) fn has_fields(&self, required: &[&str]) -> bool {
        required.iter().all(|field| self.fields.contains(*field))
    }
}

/// An in-process `tracing::Subscriber` that records `deslop_core` events.
#[derive(Debug)]
pub(crate) struct CaptureSubscriber {
    pub(crate) captured: CapturedEvents,
}

impl CaptureSubscriber {
    /// Builds a subscriber that pushes into the shared `captured` buffer.
    pub(crate) fn new(captured: CapturedEvents) -> Self {
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

/// Renders a `tracing::Level` as its canonical uppercase name (e.g. `WARN`).
pub(crate) fn level_name(level: Level) -> String {
    level.as_str().to_owned()
}

/// A `tracing::field::Visit` that flattens recorded fields into string maps.
#[derive(Default)]
pub(crate) struct FieldCollector {
    pub(crate) fields: BTreeSet<String>,
    pub(crate) values: BTreeMap<String, String>,
}

impl FieldCollector {
    /// Records `field` under both the field-name set and the value map.
    pub(crate) fn record_value(&mut self, field: &Field, value: String) {
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
