//! [PERF-SAMPLE] Bounded-duration scan sample for Windows-specific
//! performance work.
//!
//! Spawns the release `deslop` binary against a real corpus checkout for a
//! short, configurable window (default 10s), then kills it and reports the
//! engine's own observability counters from the run's log file: per-250-file
//! corpus progress throughput and, when the window is long enough to reach
//! it, the `fingerprint corpus built` stage attribution
//! (`read_ms`/`parse_ms`/`fingerprint_ms`/`signature_ms`).
//!
//! The point of the test is not to pass or fail the corpus gates — it never
//! lets the scan finish — but to produce a *comparable sample* of where the
//! first N seconds of a scan go, so two builds (or two platforms) can be
//! compared on identical early work. Profiling tools that need a live
//! process (ETW/samply) can also be pointed at the child this spawns, since
//! the invocation is exactly the corpus harness's own `SCAN_FLAGS`.
//!
//! `#[ignore]`d like the corpus suite: it needs a corpus clone on disk and
//! the release binary built (`make test-corpus` does both). Run it via
//! `cargo test --release -p deslop --test perf_sample -- --ignored --nocapture`.
//!
//! Environment:
//! - `DESLOP_PERF_ROOT` — scan root (default: the flutter corpus clone at
//!   `<repo>/.corpus/flutter-3.38.9`, the pinned checkout from
//!   `scripts/corpus/fetch-corpus.mjs`).
//! - `DESLOP_PERF_SECONDS` — sample window in seconds (default 10).

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};

/// Sample window when `DESLOP_PERF_SECONDS` is unset.
const DEFAULT_SECONDS: u64 = 10;

/// The corpus harness's `SCAN_FLAGS`, mirrored so a sample measures exactly
/// what the corpus gate measures: cold, incremental off, embeddings off.
const SCAN_FLAGS: [&str; 7] = [
    "--no-incremental",
    "--embeddings",
    "off",
    "--no-fail-over",
    "--no-color",
    "--notext",
    "--nohtml",
];

#[test]
#[ignore = "[SKIP-TOO-LARGE-FOR-CI] GH #422 [CORPUS-CI] docs/plans/corpus-assertion.md — \
            samples the first `DESLOP_PERF_SECONDS` of a corpus scan (default 10 s) against \
            the release binary and a clone on disk (default: the pinned flutter checkout); \
            never finishes the scan, so the release gate compiles this target and leaves it \
            ignored. Run via `cargo test --release -p deslop --test perf_sample -- --ignored --nocapture`"]
fn perf_sample_bounded_scan() -> Result<()> {
    let root = scan_root()?;
    let seconds = env::var("DESLOP_PERF_SECONDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_SECONDS);
    let output = repo_root()?.join("target").join("perf-sample");

    // A fresh logs dir keeps "newest log file" unambiguous when several
    // samples are taken in a row: each run gets its own prefix.
    let prefix_stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |stamp| stamp.as_secs());
    let output = output.with_extension(prefix_stamp.to_string());

    let started = Instant::now();
    let mut child = Command::new(release_binary()?)
        .arg(&root)
        .arg("--output")
        .arg(&output)
        .args(SCAN_FLAGS)
        .arg("--log-level")
        .arg("info")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("spawn release deslop binary")?;

    // Poll for early exit instead of sleeping the whole window: a scan
    // that finishes inside the window should report its own wall time,
    // not the window.
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let exited = loop {
        if let Some(_status) = child.try_wait().context("poll sample scan")? {
            break true;
        }
        if Instant::now() >= deadline {
            break false;
        }
        thread::sleep(Duration::from_millis(250));
    };
    let status = if exited {
        None
    } else {
        child.kill().context("kill sample scan")?;
        Some(seconds)
    };
    let wall = started.elapsed();

    let log = newest_log(&output)?;
    let text =
        fs::read_to_string(&log).with_context(|| format!("read sample log {}", log.display()))?;
    println!("perf sample: log={} bytes={}", log.display(), text.len());

    println!("perf sample: root={}", root.display());
    println!(
        "perf sample: window={}s killed_early={:?} actual_wall={:.1}s",
        seconds,
        status.is_some(),
        wall.as_secs_f64()
    );

    let mut last_progress: Option<(u64, u64)> = None;
    for line in text.lines() {
        // Records are `<ts>  INFO <message> key=value...`; match on the
        // message, not the line start, so timestamp-width changes cannot
        // break the parser.
        if let Some(rest) = after_marker(line, "fingerprint corpus progress ") {
            let fields = parse_fields(rest);
            let files_done = required_field(&fields, "files_done")?;
            let files_total = required_field(&fields, "files_total")?;
            let fingerprints = required_field(&fields, "fingerprints")?;
            let elapsed_ms = required_field(&fields, "elapsed_ms")?;
            let rate = files_per_second(files_done, elapsed_ms)?;
            println!(
                "perf sample: progress files_done={files_done}/{files_total} fingerprints={fingerprints} elapsed_ms={elapsed_ms} rate={rate:.1} files/s"
            );
            last_progress = Some((files_done, elapsed_ms));
        } else if let Some(rest) = after_marker(line, "fingerprint corpus built ") {
            println!("perf sample: corpus built: {rest}");
        } else if let Some(rest) = after_marker(line, "file discovery complete ") {
            println!("perf sample: discovery: {rest}");
        }
    }

    assert!(
        text.contains("fingerprint corpus progress") || text.contains("fingerprint corpus built"),
        "sample window ({}s) produced no corpus progress records — scan did not reach the \
         corpus stage; log was {} with {} bytes",
        seconds,
        log.display(),
        text.len()
    );
    if let Some((files_done, elapsed_ms)) = last_progress {
        let rate = files_per_second(files_done, elapsed_ms)?;
        println!(
            "perf sample: LAST files_done={files_done} elapsed_ms={elapsed_ms} rate={rate:.1} files/s"
        );
    }
    Ok(())
}

/// Returns the text after `marker` anywhere in `line`, so record parsing does
/// not depend on the timestamp/level prefix width.
fn after_marker<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    line.find(marker)
        .and_then(|position| position.checked_add(marker.len()))
        .and_then(|start| line.get(start..))
}

/// `files/s` from a count and a millisecond elapsed, guarding the zero case.
fn files_per_second(files: u64, elapsed_ms: u64) -> Result<f64> {
    if elapsed_ms == 0 {
        return Ok(0.0);
    }
    let files = u32::try_from(files).context("sample file count exceeds u32")?;
    Ok(f64::from(files) / Duration::from_millis(elapsed_ms).as_secs_f64())
}

/// Parses space-separated `key=value` tokens into a name→u64 map. Keys with
/// non-numeric values (e.g. `embeddings="off"`) are skipped rather than fatal.
fn parse_fields(rest: &str) -> std::collections::HashMap<&str, u64> {
    rest.split_whitespace()
        .filter_map(|token| token.split_once('='))
        .filter_map(|(key, value)| value.parse().ok().map(|number| (key, number)))
        .collect()
}

/// Required numeric field from one structured progress record.
fn required_field(fields: &std::collections::HashMap<&str, u64>, name: &str) -> Result<u64> {
    fields
        .get(name)
        .copied()
        .with_context(|| format!("progress record missing {name}"))
}

/// Repo root derived from this test's manifest location.
fn repo_root() -> Result<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .context("repo root exists")
}

/// Scan root from `DESLOP_PERF_ROOT`, else the pinned flutter corpus.
fn scan_root() -> Result<PathBuf> {
    if let Some(root) = env::var_os("DESLOP_PERF_ROOT") {
        return Ok(PathBuf::from(root));
    }
    let root = repo_root()?.join(".corpus").join("flutter-3.38.9");
    anyhow::ensure!(
        root.is_dir(),
        "flutter corpus clone missing at {} — run `node scripts/corpus/fetch-corpus.mjs flutter`",
        root.display()
    );
    Ok(root)
}

/// Release binary path, mirroring the corpus harness's expectation.
fn release_binary() -> Result<PathBuf> {
    let binary = repo_root()?
        .join("target")
        .join("release")
        .join(format!("deslop{}", std::env::consts::EXE_SUFFIX));
    anyhow::ensure!(
        binary.is_file(),
        "release binary missing at {} — build it first (`cargo build --release -p deslop`)",
        binary.display()
    );
    Ok(binary)
}

/// Newest `deslop-*.log` under the run's logs dir. The log file name is the
/// process start epoch second, so newest-by-name is newest-by-start.
fn newest_log(output_prefix: &Path) -> Result<PathBuf> {
    // The CLI puts logs under `<dir of --output>/logs`, named
    // `deslop-<epoch>.log` (see `logging.rs` / `paths::logs_dir`).
    let logs_dir = output_prefix
        .parent()
        .context("sample output prefix must have a parent")?
        .join("logs");
    let mut newest: Option<PathBuf> = None;
    for entry in fs::read_dir(&logs_dir).context("logs dir readable")? {
        let path = entry.context("read logs dir entry")?.path();
        let is_log = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("deslop-"))
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("log"));
        if !is_log {
            continue;
        }
        let newer = match &newest {
            None => true,
            Some(current) => path.file_name() >= current.file_name(),
        };
        if newer {
            newest = Some(path);
        }
    }
    newest.with_context(|| format!("no deslop log in {}", logs_dir.display()))
}
