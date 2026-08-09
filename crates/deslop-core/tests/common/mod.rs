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

pub(crate) mod clusters;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use deslop_core::{
    ast::ByteRange,
    cluster::Cluster,
    fingerprint::Fingerprint,
    pair::PairScore,
    render_report,
    report::CacheStats,
    report_metrics::AnalysedLines,
    state::{FileId, FileRegistry},
    EmbeddingProvenance, ExclusionConfig, Report, ReportInputs,
};
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

/// Counts live cluster occurrences whose path contains `component` as a
/// whole path component. Component matching, never substring, so `pkg`
/// cannot be satisfied by an unrelated `pkg_twin` sibling — the
/// prefix-eviction guard for #223 and the ignore-tree guard for #287.
pub(crate) fn occurrences_with_component(report: &Report, component: &str) -> usize {
    report
        .clusters
        .iter()
        .flat_map(|cluster| cluster.occurrences.iter())
        .filter(|occurrence| {
            occurrence
                .path
                .components()
                .any(|part| part.as_os_str() == component)
        })
        .count()
}

/// Copies the fixture tree into a temp dir so destructive edits never
/// pollute the source repo.
pub(crate) fn copy_fixture(name: &str) -> Result<tempfile::TempDir> {
    let src = fixture(name);
    let dir = tempfile::tempdir().context("tempdir")?;
    copy_recursive(&src, dir.path())?;
    Ok(dir)
}

/// Builds an [`AnalysisSession`] over `root` at `min_nodes = 15` with a
/// deterministic [`StubProvider`] — the shared preamble behind every
/// live-feature suite. Extracted because the three-line
/// `Arc::new(StubProvider::new())` → `AnalysisSession::new(.., 15, false,
/// None, provider)` pairing is an `identical` cluster in Deslop's own
/// report; new suites reuse this instead of adding a copy.
#[cfg(feature = "live")]
pub(crate) fn live_session(root: &Path) -> Result<deslop_core::live::AnalysisSession> {
    let provider = Arc::new(deslop_core::embedding::test_support::StubProvider::new());
    deslop_core::live::AnalysisSession::new(root.to_path_buf(), 15, false, None, provider)
        .context("session")
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

/// Shared `render_report` fixture for the report-rendering regression
/// suites (`issue_98_99_108_120_122_thresholds`,
/// `issue_121_pytest_fixture_boilerplate`, `issue_239_csharp_reparse`).
/// Registers
/// sources under one scan root, stamps every file with a single
/// `language`, and renders fabricated clusters through the production
/// [`render_report`] path. Consolidated here from the byte-identical
/// copies previously duplicated per test file (Deslop cluster
/// `a59932844b88648e`).
pub(crate) struct ReportFixture {
    scan_root: PathBuf,
    language: &'static str,
    registry: FileRegistry,
    file_languages: HashMap<FileId, &'static str>,
    sources: HashMap<FileId, Vec<u8>>,
    analysed_lines: AnalysedLines,
}

impl ReportFixture {
    /// Creates an empty fixture whose files all carry `language`.
    pub(crate) fn new(scan_root: &Path, language: &'static str) -> Self {
        Self {
            scan_root: scan_root.to_owned(),
            language,
            registry: FileRegistry::new(),
            file_languages: HashMap::new(),
            sources: HashMap::new(),
            analysed_lines: HashMap::new(),
        }
    }

    /// Registers `path` with `source` bytes exactly once and returns its
    /// [`FileId`]. Use with [`ReportFixture::fingerprint_at`] when one
    /// file must carry several cluster members at distinct byte ranges —
    /// `FileRegistry::register` appends unconditionally, so registering
    /// per member would fork the same path into divergent ids.
    pub(crate) fn file(&mut self, path: &str, source: &str) -> FileId {
        let file_id = self.registry.register(self.scan_root.join(path));
        let _old = self.sources.insert(file_id, source.as_bytes().to_vec());
        let _old = self.file_languages.insert(file_id, self.language);
        let _old = self.analysed_lines.insert(
            file_id,
            u64::try_from(source.lines().count()).unwrap_or(u64::MAX),
        );
        file_id
    }

    /// Builds a member fingerprint covering `byte_range` of a file
    /// previously registered via [`ReportFixture::file`].
    pub(crate) fn fingerprint_at(
        file_id: FileId,
        byte_range: ByteRange,
        node_count: usize,
        hash_seed: usize,
    ) -> Fingerprint {
        Fingerprint {
            hash: [u8::try_from(hash_seed).unwrap_or(u8::MAX); 32],
            file_id,
            byte_range,
            node_count,
        }
    }

    /// Registers one whole-file source per snippet and clusters them.
    pub(crate) fn cluster(
        &mut self,
        id: &str,
        snippets: Vec<(&str, &str)>,
        node_count: usize,
        signals: PairScore,
    ) -> Cluster {
        let members = snippets
            .into_iter()
            .enumerate()
            .map(|(index, (path, source))| self.member(path, source, node_count, index))
            .collect::<Vec<_>>();
        Cluster {
            id: id.to_owned(),
            members,
            weight: 10_000.0,
            signals,
            content_agreement: 1.0,
        }
    }

    /// Registers `source` at `path` and returns a whole-file member.
    fn member(
        &mut self,
        path: &str,
        source: &str,
        node_count: usize,
        hash_seed: usize,
    ) -> Fingerprint {
        let file_id = self.file(path, source);
        Self::fingerprint_at(
            file_id,
            ByteRange {
                start: 0,
                end: source.len(),
            },
            node_count,
            hash_seed,
        )
    }

    /// Renders `clusters` through the production report pipeline.
    pub(crate) fn render(&self, clusters: &[Cluster]) -> deslop_core::Report {
        let exclusion = ExclusionConfig::empty();
        render_report(ReportInputs {
            clusters,
            registry: &self.registry,
            file_languages: &self.file_languages,
            files_analysed: self.sources.len(),
            min_nodes: 15,
            scan_root: &self.scan_root,
            exclusion: &exclusion,
            embedding_provenance: Some(EmbeddingProvenance {
                provider_id: "stub".to_owned(),
                model_id: "report-fixture".to_owned(),
                model_version: "test".to_owned(),
                dimensions: 3,
                attempted_subtrees: self.sources.len(),
                indexed_subtrees: self.sources.len(),
                failed_subtrees: 0,
            }),
            cache_stats: CacheStats::default(),
            sources: &self.sources,
            analysed_lines: &self.analysed_lines,
            boilerplate_ranges: &[],
        })
    }
}

/// Minimum subtree size shared by the refactor E2E suites — matches
/// the LSP server default so the same clusters appear on both
/// surfaces ([AUTOFIX-EXTRACT-TESTING]).
pub(crate) const REFACTOR_MIN_NODES: u32 = 30;

/// Resolves a golden file under the LSP code-action fixture directory
/// ([AUTOFIX-EXTRACT-TESTING] golden location), shared with the LSP
/// E2E suite.
pub(crate) fn refactor_golden(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../deslop-lsp/tests/fixtures/code_action")
        .join(name)
}

/// Runs the full analysis pipeline over a fixture workspace with the
/// refactor suites' settings (embeddings off, incremental off).
pub(crate) fn analyse_refactor_fixture(root: &Path) -> Result<deslop_core::report::Report> {
    deslop_core::run(&deslop_core::PipelineConfig {
        root: root.to_path_buf(),
        min_nodes: REFACTOR_MIN_NODES,
        config_path: None,
        embedding: refactor_embedding_settings(),
        incremental: false,
    })
    .map_err(|error| anyhow!("pipeline failed: {error}"))
}

/// Initialises a live pipeline session over a fixture workspace with
/// the refactor suites' settings.
pub(crate) fn refactor_pipeline_session(
    root: &Path,
) -> Result<(deslop_core::PipelineSession, deslop_core::report::Report)> {
    deslop_core::PipelineSession::initialise(
        root.to_path_buf(),
        REFACTOR_MIN_NODES,
        false,
        None,
        refactor_embedding_settings(),
    )
    .map_err(|error| anyhow!("session init failed: {error}"))
}

/// The refactor suites' embedding policy: off, no provider.
fn refactor_embedding_settings() -> deslop_core::EmbeddingSettings<'static> {
    deslop_core::EmbeddingSettings {
        mode: deslop_core::EmbeddingMode::Off,
        provider: None,
        batch_yield: None,
        progress: None,
    }
}
