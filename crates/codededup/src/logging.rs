//! Tracing setup for the CLI.
//!
//! The CLI's stderr is reserved for the human-readable preamble +
//! summary (see [`crate::summary`]); log lines go to a timestamped
//! file next to the report output by default. `--log-to-console`
//! bounces them back onto stderr, and `--log-level` filters both
//! sinks.
//!
//! The log file name is `codededup-<yyyymmddTHHMMSS>.log`, placed in
//! the same directory as the rendered report. The timestamp comes
//! from `SystemTime::now()` formatted manually so we do not pull in
//! the `time` crate for one line of formatting.

use std::{
    fs, io,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use tracing::Level;
use tracing_subscriber::{fmt, EnvFilter};

/// Where tracing writes its output for the current run.
#[derive(Debug)]
pub enum LogSink {
    /// Timestamped log file at this path.
    File(PathBuf),
    /// Standard error.
    Console,
}

/// Initialises the global tracing subscriber.
///
/// When `log_to_console` is false (the default), creates a
/// timestamped `codededup-<ts>.log` file under `log_dir` and writes
/// all log events there. When true, writes to stderr. `level` is the
/// minimum severity emitted; `RUST_LOG` overrides it when set.
///
/// # Errors
///
/// Returns an error if the log file cannot be created, the filter
/// cannot be parsed, or `tracing` has already been initialised.
pub fn init(log_dir: &Path, log_to_console: bool, level: Level) -> Result<LogSink> {
    let filter = build_filter(level)?;
    if log_to_console {
        fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_writer(std::io::stderr)
            .try_init()
            .map_err(|err| anyhow::anyhow!("failed to initialise tracing: {err}"))?;
        return Ok(LogSink::Console);
    }
    fs::create_dir_all(log_dir)
        .with_context(|| format!("create log directory {}", log_dir.display()))?;
    let path = log_file_path(log_dir);
    let file =
        fs::File::create(&path).with_context(|| format!("create log file {}", path.display()))?;
    // Leak the Mutex so `MakeWriter` can hand out a `&'static`
    // reference on every event — required by tracing's writer trait
    // and acceptable because the CLI runs once per process.
    let shared: &'static Mutex<fs::File> = Box::leak(Box::new(Mutex::new(file)));
    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_ansi(false)
        .with_writer(move || FileSink { inner: shared })
        .try_init()
        .map_err(|err| anyhow::anyhow!("failed to initialise tracing: {err}"))?;
    Ok(LogSink::File(path))
}

/// `io::Write` shim around a shared log file. Each tracing event
/// calls the `MakeWriter` closure which hands back one of these;
/// the `Mutex` serialises writes so interleaved events do not shred
/// each other's bytes.
struct FileSink {
    /// Shared handle into the log file (see [`init`]).
    inner: &'static Mutex<fs::File>,
}

impl io::Write for FileSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // `fs::File::write` on append-open mode is atomic and also
        // flushes to the OS, so `tracing` never needs to call our
        // `flush` explicitly. `flush` is still required by the
        // `Write` trait; it just delegates to the same locked handle.
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("log mutex poisoned"))?;
        guard.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("log mutex poisoned"))?;
        guard.flush()
    }
}

/// Parses `--log-level <level>` and composes it with `RUST_LOG`.
///
/// `RUST_LOG` (if set) wins, matching Rust-ecosystem convention; the
/// CLI flag is the fallback so users who never set `RUST_LOG` get
/// exactly the severity they asked for.
fn build_filter(level: Level) -> Result<EnvFilter> {
    if let Ok(filter) = EnvFilter::try_from_default_env() {
        return Ok(filter);
    }
    EnvFilter::from_str(level.as_str())
        .map_err(|err| anyhow::anyhow!("invalid --log-level {level}: {err}"))
}

/// Renders the default `codededup-<unix-seconds>.log` path under
/// `log_dir`. Using the raw epoch second count keeps the file name
/// unambiguous and unique across DST shifts without dragging in a
/// calendar crate; operators who want a human-readable stamp can
/// pipe the log through `ls -lT` or `stat`.
fn log_file_path(log_dir: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    log_dir.join(format!("codededup-{stamp}.log"))
}
