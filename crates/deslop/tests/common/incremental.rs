//! Shared helpers for the incremental-analysis suites
//! ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE],
//! [PIPELINE-INCREMENTAL-ANALYSIS-REUSE]): exact `cache_stats`
//! assertions, the strip-and-compare view the equivalence contract is
//! judged by, and the `fingerprint corpus built` reuse counters. One
//! copy, shared by every incremental suite — they must all agree on
//! what "identical modulo `cache_stats`" and "reused, not rebuilt"
//! mean.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context as _};
use assert_cmd::Command;
use serde_json::Value;

use super::{field, Result};

/// The `fingerprint corpus built` counters for one pass — the
/// structured observable [PIPELINE-INCREMENTAL-ANALYSIS-REUSE] is
/// judged by. Timing assertions are banned, so reuse is proven by
/// these counts and never by a stopwatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReuseCounters {
    /// Files the corpus loop walked.
    pub(crate) files_processed: u64,
    /// Fingerprints the pass carries in total.
    pub(crate) fingerprints: u64,
    /// Files served from the on-disk parse store.
    pub(crate) cache_hits: u64,
    /// Files that missed and were parsed from source.
    pub(crate) cache_misses: u64,
    /// Signatures constructed from token streams this pass.
    pub(crate) signatures_built: u64,
    /// Signatures attached from the parse store this pass.
    pub(crate) signatures_reused: u64,
}

impl ReuseCounters {
    /// Parses the single `fingerprint corpus built` event out of a run's
    /// log body.
    pub(crate) fn from_log(log_body: &str) -> Result<Self> {
        let line = corpus_event_line(log_body)?;
        Ok(Self {
            files_processed: event_field(&line, "files_processed")?,
            fingerprints: event_field(&line, "fingerprints")?,
            cache_hits: event_field(&line, "cache_hits")?,
            cache_misses: event_field(&line, "cache_misses")?,
            signatures_built: event_field(&line, "signatures_built")?,
            signatures_reused: event_field(&line, "signatures_reused")?,
        })
    }

    /// Every invariant that must hold on **every** pass, warm or cold.
    ///
    /// Conservation is the load-bearing one: each fingerprint carries
    /// exactly one signature, either built this pass or attached from
    /// the store. A pass that loses or double-counts signatures breaks
    /// the positional 1:1 binding the LSH stage relies on, so it is
    /// checked everywhere rather than in one place.
    pub(crate) fn assert_invariants(&self, label: &str) {
        assert_eq!(
            self.signatures_built.saturating_add(self.signatures_reused),
            self.fingerprints,
            "{label}: every fingerprint must carry exactly one signature — \
             built + reused must equal the fingerprint count: {self:?}"
        );
        assert!(
            self.fingerprints > 0,
            "{label}: a corpus that fingerprints nothing asserts nothing: {self:?}"
        );
    }

    /// Asserts the store served exactly `hits` files and re-parsed
    /// exactly `misses`, and that the two account for every processed
    /// file.
    pub(crate) fn assert_cache(&self, hits: u64, misses: u64, label: &str) {
        assert_eq!(
            (self.cache_hits, self.cache_misses),
            (hits, misses),
            "{label}: event (cache_hits, cache_misses): {self:?}"
        );
        assert_eq!(
            self.cache_hits.saturating_add(self.cache_misses),
            self.files_processed,
            "{label}: every processed file must either hit or miss the store: {self:?}"
        );
    }

    /// Asserts exactly `built` signatures were constructed and exactly
    /// `reused` attached from the store.
    pub(crate) fn assert_signatures(&self, built: u64, reused: u64, label: &str) {
        assert_eq!(
            (self.signatures_built, self.signatures_reused),
            (built, reused),
            "{label}: event (signatures_built, signatures_reused): {self:?}"
        );
    }

    /// Asserts the pass never consulted the store — `--no-incremental`
    /// or the `[analysis] incremental = false` config opt-out
    /// ([CONFIG-INCREMENTAL-OPTOUT]). Hits and misses are both exactly
    /// zero and every signature is built from token streams. The
    /// store-on conservation rule of [`Self::assert_cache`]
    /// (`hits + misses == files_processed`) deliberately does not apply:
    /// a store that is never consulted accounts for no file at all.
    pub(crate) fn assert_store_disabled(&self, label: &str) {
        assert_eq!(
            (self.cache_hits, self.cache_misses),
            (0, 0),
            "{label}: a disabled store must never hit or miss: {self:?}"
        );
        assert_signatures_disabled(self, label);
    }
}

/// The signature half of [`ReuseCounters::assert_store_disabled`], split
/// out to keep the method under the function-size limit.
fn assert_signatures_disabled(events: &ReuseCounters, label: &str) {
    events.assert_signatures(events.fingerprints, 0, label);
    assert!(
        events.files_processed > 0,
        "{label}: a pass that processed no files proves nothing about the \
         store being disabled: {events:?}"
    );
}

/// Whether a run consults the on-disk parse store. The store is on by
/// default and always lands in the *scan root* ([OUTPUT-DIR]), so
/// `common::deslop_cmd` hard-codes the opt-out and cannot serve these
/// suites — this is the one axis every incremental test varies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Store {
    /// Default behaviour: read and write `<scan_root>/.deslop/cache`.
    On,
    /// `--no-incremental`: never read or write the store.
    Off,
}

impl Store {
    /// The flags this mode adds to a run.
    fn flags(self) -> &'static [&'static str] {
        match self {
            Self::On => &[],
            Self::Off => &["--no-incremental"],
        }
    }

    /// The mode a suite's `incremental` flag selects.
    pub(crate) fn incremental(incremental: bool) -> Self {
        if incremental {
            Self::On
        } else {
            Self::Off
        }
    }
}

/// Runs `deslop <scan_root>`, writing the report under `<out_dir>/report`
/// and tracing under `<out_dir>/logs/`, and returns the report's raw
/// bytes alongside the pass's reuse counters. A fresh `out_dir` per run
/// keeps that run's single timestamped log unambiguous. `extra_args` is
/// appended after the fixed `--min-nodes <min_nodes> --embeddings off`
/// and the `store` flags.
///
/// Raw bytes rather than a parsed [`Value`] because the golden suites
/// pin the serialisation itself — member order and formatting included,
/// not just the decoded document.
pub(crate) fn run_capturing_bytes(
    scan_root: &Path,
    out_dir: &Path,
    min_nodes: u32,
    store: Store,
    extra_args: &[&str],
) -> Result<(Vec<u8>, ReuseCounters)> {
    fs::create_dir_all(out_dir)?;
    let output = out_dir.join("report");
    let mut cmd = Command::cargo_bin("deslop")?;
    let _cmd = cmd.env_remove("RUST_LOG").arg(scan_root);
    let _output = cmd.arg("--output").arg(&output);
    let _fixed = cmd.args(["--min-nodes", &min_nodes.to_string(), "--embeddings", "off"]);
    let _store = cmd.args(store.flags());
    let _extra = cmd.args(extra_args);
    let _assertion = cmd.assert().success();
    let bytes = fs::read(output.with_extension("json"))?;
    let counters = ReuseCounters::from_log(&read_single_log(out_dir)?)?;
    Ok((bytes, counters))
}

/// [`run_capturing_bytes`] with the parse store on and the report parsed.
pub(crate) fn run_store_on(
    scan_root: &Path,
    out_dir: &Path,
    min_nodes: u32,
    extra_args: &[&str],
) -> Result<(Value, ReuseCounters)> {
    let (bytes, counters) =
        run_capturing_bytes(scan_root, out_dir, min_nodes, Store::On, extra_args)?;
    Ok((serde_json::from_slice(&bytes)?, counters))
}

/// Renders one report over `scan_root` at `min_nodes` with the store in
/// the given mode, into a throwaway output directory. The store-aware
/// counterpart to `common::run_report`, which can only ever run with the
/// store off.
pub(crate) fn run_report_with_store(
    scan_root: &Path,
    min_nodes: u32,
    store: Store,
) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let (bytes, _counters) = run_capturing_bytes(scan_root, tmp.path(), min_nodes, store, &[])?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Reads the single `deslop-<ts>.log` under `<out_dir>/logs/`
/// ([OUTPUT-DIR]) — the ANSI-free default sink the CLI routes tracing
/// events to (`tests/cli/logging.rs` pins that routing).
pub(crate) fn read_single_log(out_dir: &Path) -> Result<String> {
    let logs_dir = out_dir.join("logs");
    let logs: Vec<PathBuf> = fs::read_dir(&logs_dir)
        .with_context(|| format!("no logs directory under {}", out_dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| is_timestamped_log(path))
        .collect();
    match logs.as_slice() {
        [only] => Ok(fs::read_to_string(only)?),
        other => Err(anyhow!(
            "expected exactly one deslop-*.log under {}, found {other:?}",
            logs_dir.display()
        )),
    }
}

/// True for the CLI's `deslop-<unix-seconds>.log` file names.
fn is_timestamped_log(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("deslop-"))
        && path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("log"))
}

/// The single `fingerprint corpus built` event line in `log_body`.
/// `fingerprint_corpus` emits it exactly once per batch build, so
/// exactly one line keeps the field parse unambiguous.
fn corpus_event_line(log_body: &str) -> Result<String> {
    let lines: Vec<&str> = log_body
        .lines()
        .filter(|line| line.contains("fingerprint corpus built"))
        .collect();
    match lines.as_slice() {
        [only] => Ok((*only).to_owned()),
        other => Err(anyhow!(
            "expected exactly one \"fingerprint corpus built\" event, found {}: {log_body}",
            other.len()
        )),
    }
}

/// A `name=value` field off a tracing fmt event line, failing by name
/// when the field is absent.
fn event_field(line: &str, name: &str) -> Result<u64> {
    let prefix = format!("{name}=");
    let raw = line
        .split_whitespace()
        .find_map(|token| token.strip_prefix(prefix.as_str()))
        .ok_or_else(|| {
            anyhow!(
                "\"fingerprint corpus built\" event has no `{name}` field \
                 ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE] requires it): {line}"
            )
        })?;
    raw.parse::<u64>()
        .with_context(|| format!("`{name}` field is not a u64 in: {line}"))
}

/// Asserts the report's exact `cache_stats` — the one member allowed to
/// differ between an incremental and a cold pass.
pub(crate) fn assert_cache_stats(report: &Value, hits: u64, misses: u64, label: &str) {
    let stats = field(report, "cache_stats");
    let actual = (
        field(stats, "hits").as_u64(),
        field(stats, "misses").as_u64(),
    );
    assert_eq!(
        actual,
        (Some(hits), Some(misses)),
        "{label}: cache (hits, misses): {report}"
    );
}

/// Asserts one pass's two independent cache surfaces agree with each
/// other and with `(hits, misses)`: the rendered `cache_stats` member,
/// and the `fingerprint corpus built` event. A pass whose report and
/// event disagree is describing a store it did not actually use, so both
/// are always checked together, along with every per-pass invariant.
pub(crate) fn assert_pass(
    report: &Value,
    events: &ReuseCounters,
    hits: u64,
    misses: u64,
    label: &str,
) {
    assert_cache_stats(report, hits, misses, label);
    events.assert_cache(hits, misses, label);
    events.assert_invariants(label);
}

/// [`assert_pass`] for a fully-cold pass over `files` files: the store
/// served nothing, and every signature was built from token streams.
pub(crate) fn assert_cold_pass(report: &Value, events: &ReuseCounters, files: u64, label: &str) {
    assert_pass(report, events, 0, files, label);
    events.assert_signatures(events.fingerprints, 0, label);
}

/// [`assert_pass`] for a fully-warm pass over `files` files: the store
/// served every file, and no signature was rebuilt
/// ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]).
pub(crate) fn assert_warm_pass(report: &Value, events: &ReuseCounters, files: u64, label: &str) {
    assert_pass(report, events, files, 0, label);
    events.assert_signatures(0, events.fingerprints, label);
}

/// Seeds a fresh corpus under `<tmp>/src` via `seed` and runs the
/// store-filling cold pass over it, asserting every seeded file was
/// analysed and missed. The shared prologue of every "damage the store,
/// then prove it heals" and "prove two parsers never share an entry"
/// scenario — one definition of what a freshly-filled store means.
pub(crate) fn seeded_cold_pass(
    tmp: &Path,
    seed: impl Fn(&Path) -> Result<()>,
    min_nodes: u32,
    files: u64,
) -> Result<(PathBuf, Value, ReuseCounters)> {
    let scan_root = tmp.join("src");
    seed(&scan_root)?;
    let (cold, cold_events) = run_store_on(&scan_root, &tmp.join("cold"), min_nodes, &[])?;
    assert_eq!(
        field(&cold, "files_analysed").as_u64(),
        Some(files),
        "cold pass must analyse every seeded file: {cold:#}"
    );
    assert_cold_pass(&cold, &cold_events, files, "cold");
    Ok((scan_root, cold, cold_events))
}

/// Both passes of a cold-then-warm cycle over one scan root, with the
/// counters each reported.
pub(crate) struct ColdThenWarm {
    /// The store-filling pass.
    pub(crate) cold: Value,
    /// Counters the cold pass emitted.
    pub(crate) cold_events: ReuseCounters,
    /// The store-served pass over the same, unchanged corpus.
    pub(crate) warm: Value,
    /// Counters the warm pass emitted.
    pub(crate) warm_events: ReuseCounters,
}

/// Runs one cold and one warm store-on pass over `scan_root`, asserting
/// the whole [PIPELINE-INCREMENTAL-ANALYSIS-REUSE] contract for a corpus
/// of `files` byte-distinct files: the cold pass fills the store and
/// builds every signature from token streams, the warm pass serves every
/// file and rebuilds none, both passes fingerprint identically, and the
/// warm report owes the cold one field for field.
///
/// Every suite that opens with "warm the store, then prove reuse" starts
/// here, so there is exactly one definition of what that phrase means.
pub(crate) fn cold_then_warm(
    scan_root: &Path,
    out_root: &Path,
    min_nodes: u32,
    files: u64,
) -> Result<ColdThenWarm> {
    let (cold, cold_events) = run_store_on(scan_root, &out_root.join("cold"), min_nodes, &[])?;
    let (warm, warm_events) = run_store_on(scan_root, &out_root.join("warm"), min_nodes, &[])?;
    assert_cold_pass(&cold, &cold_events, files, "cold");
    assert_warm_pass(&warm, &warm_events, files, "warm");
    assert_eq!(
        warm_events.fingerprints, cold_events.fingerprints,
        "an unchanged corpus must carry the same fingerprint count F on both \
         passes: cold {cold_events:?} warm {warm_events:?}"
    );
    assert_reports_equal(&warm, &cold, "fully-warm pass vs cold pass");
    Ok(ColdThenWarm {
        cold,
        cold_events,
        warm,
        warm_events,
    })
}

/// The report minus its top-level `cache_stats` member — the exact view
/// the equivalence contract compares. Asserts the member existed, so a
/// schema drift can never make the strip (or its comparison) vacuous.
pub(crate) fn without_cache_stats(report: &Value) -> Value {
    let mut view = report.clone();
    let removed = view
        .as_object_mut()
        .and_then(|members| members.remove("cache_stats"));
    assert!(
        removed.is_some(),
        "report carries no top-level cache_stats member to strip: {report}"
    );
    view
}

/// Top-level members whose values differ between two stripped reports —
/// the first thing an equivalence failure message must name.
fn differing_members(left: &Value, right: &Value) -> Vec<String> {
    let member_names: BTreeSet<String> = [left, right]
        .iter()
        .filter_map(|value| value.as_object())
        .flat_map(|members| members.keys().cloned())
        .collect();
    member_names
        .into_iter()
        .filter(|name| left.get(name) != right.get(name))
        .collect()
}

/// Asserts the incremental report equals the cold report for the same
/// corpus state, field for field, after removing exactly the top-level
/// `cache_stats` member from both sides.
pub(crate) fn assert_reports_equal(incremental: &Value, cold: &Value, scenario: &str) {
    let incremental_view = without_cache_stats(incremental);
    let cold_view = without_cache_stats(cold);
    let diverging = differing_members(&incremental_view, &cold_view);
    assert_eq!(
        incremental_view, cold_view,
        "{scenario}: incremental report diverged from the cold report of the same corpus \
         state in top-level members {diverging:?}; cache_stats is the sole permitted \
         difference ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE])\n\
         incremental: {incremental:#}\ncold: {cold:#}"
    );
}
