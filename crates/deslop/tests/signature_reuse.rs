//! Warm runs must reuse persisted `MinHash` signatures instead of
//! rebuilding every one from token streams
//! ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]).
//!
//! BORN RED BY DESIGN — this is the test-first pin for
//! docs/plans/incremental-analysis-plan.md, Phase 2 (signature
//! persistence). Today the pipeline rebuilds every signature on every
//! pass, warm or cold, and the "fingerprint corpus built" info event
//! (crates/deslop-core/src/pipeline/corpus.rs, target
//! `deslop_core::pipeline::corpus`) carries only `files_processed`,
//! `fingerprints`, `cache_hits`, and `cache_misses`. The Phase 2
//! implementation — persisting each subtree's `MinHash` signature beside
//! its fingerprints in the on-disk parse store and attaching the
//! stored signatures on cache hits — turns this test green by adding
//! two structured fields to that same event:
//!
//! - `signatures_built`  — signatures constructed from token streams
//!   this pass;
//! - `signatures_reused` — signatures attached from the on-disk parse
//!   store.
//!
//! Contract: cold default run → `signatures_reused=0`,
//! `signatures_built=F` where F is the total fingerprint count;
//! fully-warm run → `signatures_built=0`, `signatures_reused=F` (same
//! F); `--no-incremental` → `signatures_built=F`,
//! `signatures_reused=0`.
//!
//! The observable is the structured tracing surface — timing
//! assertions are banned. The CLI's default sink writes ANSI-free fmt
//! lines to `<output dir>/logs/deslop-<ts>.log` (routing pinned by
//! tests/cli/logging.rs), so each run's event fields are parsed as
//! `key=value` tokens from the single log file that run wrote. And the
//! reuse can never be bought with a report difference: the warm report
//! must equal the cold report with `/cache_stats` removed.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use assert_cmd::Command;
use serde_json::Value;

mod common;
use crate::common::*;

/// The clone body shared verbatim by `alpha.rs` and `beta.rs`. Seven
/// lines, byte-identical in both files, so one cluster spanning the
/// pair is guaranteed at `--min-nodes 8`.
const CLONE_BODY: &str = "pub fn compute(items: &[i32]) -> i32 {\n\
    \x20   let mut total = 0;\n\
    \x20   for item in items {\n\
    \x20       if *item > 0 { total += item * 2; } else { total -= item; }\n\
    \x20   }\n\
    \x20   total\n\
}\n";

/// A genuinely different function for `gamma.rs` — real code that
/// duplicates nothing, so the corpus has exactly one clone pair.
const DISTINCT_SOURCE: &str = "pub fn label(count: usize) -> String {\n\
    \x20   match count {\n\
    \x20       0 => \"none\".to_owned(),\n\
    \x20       1 => \"one\".to_owned(),\n\
    \x20       other => format!(\"{other} items\"),\n\
    \x20   }\n\
}\n";

/// Seeds three byte-distinct Rust files: the `alpha.rs`/`beta.rs`
/// clone pair (distinct leading comments keep the file bytes — and so
/// the content-addressed cache keys — distinct) plus the unrelated
/// `gamma.rs`, so cold/warm cache stats are exactly {0,3}/{3,0}.
fn seed_corpus(scan_root: &Path) -> Result<()> {
    fs::create_dir_all(scan_root)?;
    fs::write(
        scan_root.join("alpha.rs"),
        format!("// alpha: the canonical copy.\n{CLONE_BODY}"),
    )?;
    fs::write(
        scan_root.join("beta.rs"),
        format!("// beta: the pasted copy.\n{CLONE_BODY}"),
    )?;
    fs::write(scan_root.join("gamma.rs"), DISTINCT_SOURCE)?;
    Ok(())
}

/// Runs `deslop <scan_root>` with the incremental cache on (the
/// default — the store lands at `<scan_root>/.deslop/cache`), writing
/// reports under `<out_dir>/report` and the tracing log under
/// `<out_dir>/logs/`. A fresh out dir per run keeps each run's single
/// timestamped log file unambiguous. Returns the parsed JSON report
/// and the raw log body.
fn run_default_incremental(scan_root: &Path, out_dir: &Path) -> Result<(Value, String)> {
    fs::create_dir_all(out_dir)?;
    let output = out_dir.join("report");
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .env_remove("RUST_LOG")
        .arg(scan_root)
        .arg("--output")
        .arg(&output)
        .args(["--min-nodes", "8", "--embeddings", "off"])
        .assert()
        .success();
    let report = load_json(&output.with_extension("json"))?;
    let log_body = read_single_log(out_dir)?;
    Ok((report, log_body))
}

/// Reads the single `deslop-<ts>.log` under `<out_dir>/logs/`
/// ([OUTPUT-DIR]) — the ANSI-free default sink the CLI routes tracing
/// events to (tests/cli/logging.rs pins that routing).
fn read_single_log(out_dir: &Path) -> Result<String> {
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

/// The single "fingerprint corpus built" event line in `log_body`.
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

/// A `name=value` field off a tracing fmt event line. An absent field
/// fails naming the missing field — exactly how this test stays red
/// until [PIPELINE-INCREMENTAL-ANALYSIS-REUSE] lands the
/// `signatures_built` / `signatures_reused` fields on the event.
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

/// Asserts a report's `cache_stats` block is exactly `{hits, misses}`.
fn assert_cache_stats(report: &Value, label: &str, hits: u64, misses: u64) {
    let stats = field(report, "cache_stats");
    assert_eq!(
        field(stats, "hits").as_u64(),
        Some(hits),
        "{label} run must report exactly {hits} cache hits: {report}"
    );
    assert_eq!(
        field(stats, "misses").as_u64(),
        Some(misses),
        "{label} run must report exactly {misses} cache misses: {report}"
    );
}

/// `report` with the top-level `cache_stats` block removed — the only
/// field allowed to differ between the cold and the warm run.
fn without_cache_stats(report: &Value) -> Value {
    let mut clone = report.clone();
    if let Some(object) = clone.as_object_mut() {
        let _removed = object.remove("cache_stats");
    }
    clone
}

// Implements [PIPELINE-INCREMENTAL-ANALYSIS-REUSE]: a MinHash
// signature is a pure function of one subtree's normalised token
// k-grams, so a fully-warm pass may rebuild none of them — and must
// say so on the structured tracing surface, while rendering a report
// identical to the cold run's outside `cache_stats`.
#[test]
fn warm_run_reuses_persisted_signatures_instead_of_rebuilding() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_corpus(&scan_root)?;

    let (cold, cold_log) = run_default_incremental(&scan_root, &tmp.path().join("cold"))?;
    let (warm, warm_log) = run_default_incremental(&scan_root, &tmp.path().join("warm"))?;

    // Mechanics are proven first, so the only failure the contract
    // block below can produce is the missing signature fields.

    // The corpus is real: three files analysed, one genuine clone pair.
    assert_eq!(
        field(&cold, "files_analysed").as_u64(),
        Some(3),
        "cold run must analyse all three seeded files: {cold}"
    );
    assert_eq!(
        field(&warm, "files_analysed").as_u64(),
        Some(3),
        "warm run must analyse all three seeded files: {warm}"
    );
    let clone = expect_cluster_spanning(&cold, &["alpha.rs", "beta.rs"])?;
    assert_eq!(
        cluster_bucket(clone),
        "identical",
        "the seeded pair is byte-identical code inside distinct files: {cold}"
    );
    assert_eq!(
        cluster_size(clone),
        2,
        "the clone must span exactly the two seeded occurrences: {cold}"
    );

    // All three byte-distinct files miss the cold cache and hit warm.
    assert_cache_stats(&cold, "cold", 0, 3);
    assert_cache_stats(&warm, "warm", 3, 0);

    // Reuse can never be bought with a report difference: outside
    // `cache_stats`, the warm report IS the cold report.
    assert_eq!(
        without_cache_stats(&warm),
        without_cache_stats(&cold),
        "warm report must equal the cold report with /cache_stats removed"
    );

    // The event line parses, and its existing fields agree with the
    // rendered cache stats — the key=value extraction is sound, so a
    // missing-field failure below indicts the event, not the parser.
    let cold_event = corpus_event_line(&cold_log)?;
    let warm_event = corpus_event_line(&warm_log)?;
    assert_eq!(
        event_field(&cold_event, "files_processed")?,
        3,
        "cold event must cover all three files: {cold_event}"
    );
    assert_eq!(
        event_field(&warm_event, "files_processed")?,
        3,
        "warm event must cover all three files: {warm_event}"
    );
    assert_eq!(
        event_field(&cold_event, "cache_hits")?,
        0,
        "cold event cache_hits must match the rendered report: {cold_event}"
    );
    assert_eq!(
        event_field(&cold_event, "cache_misses")?,
        3,
        "cold event cache_misses must match the rendered report: {cold_event}"
    );
    assert_eq!(
        event_field(&warm_event, "cache_hits")?,
        3,
        "warm event cache_hits must match the rendered report: {warm_event}"
    );
    assert_eq!(
        event_field(&warm_event, "cache_misses")?,
        0,
        "warm event cache_misses must match the rendered report: {warm_event}"
    );
    let total_fingerprints = event_field(&cold_event, "fingerprints")?;
    assert!(
        total_fingerprints > 0,
        "the corpus must fingerprint at least one subtree: {cold_event}"
    );
    assert_eq!(
        event_field(&warm_event, "fingerprints")?,
        total_fingerprints,
        "warm corpus must carry the same fingerprint count F: {warm_event}"
    );

    // The [PIPELINE-INCREMENTAL-ANALYSIS-REUSE] contract — red until
    // plan Phase 2 lands signature persistence.

    // Cold: nothing persisted yet, every signature built from tokens.
    assert_eq!(
        event_field(&cold_event, "signatures_reused")?,
        0,
        "cold run has no store to reuse signatures from: {cold_event}"
    );
    let cold_built = event_field(&cold_event, "signatures_built")?;
    assert_eq!(
        cold_built, total_fingerprints,
        "cold run must build one signature per fingerprint (F): {cold_event}"
    );

    // Warm: every signature attached from the parse store, none rebuilt.
    assert_eq!(
        event_field(&warm_event, "signatures_built")?,
        0,
        "fully-warm run must rebuild no signatures: {warm_event}"
    );
    assert_eq!(
        event_field(&warm_event, "signatures_reused")?,
        cold_built,
        "fully-warm run must reuse exactly the F signatures the cold run built: {warm_event}"
    );

    Ok(())
}
