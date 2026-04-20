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
    let path = log_file_path(log_dir);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create log directory {}", parent.display()))?;
        }
    }
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
        let Ok(mut guard) = self.inner.lock() else {
            return Err(io::Error::other("log mutex poisoned"));
        };
        guard.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let Ok(mut guard) = self.inner.lock() else {
            return Err(io::Error::other("log mutex poisoned"));
        };
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

/// Renders the default `codededup-<yyyymmddTHHMMSS>.log` path under
/// `log_dir`. The timestamp is UTC — keeping it timezone-neutral so
/// two runs one second apart never collide regardless of local DST.
fn log_file_path(log_dir: &Path) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let stamp = format_utc(now.as_secs());
    log_dir.join(format!("codededup-{stamp}.log"))
}

/// Formats a UNIX-epoch second count as `yyyymmddTHHMMSS` in UTC.
fn format_utc(epoch_seconds: u64) -> String {
    let (year, month, day, hour, minute, second) = break_down(epoch_seconds);
    format!("{year:04}{month:02}{day:02}T{hour:02}{minute:02}{second:02}")
}

/// Decomposes `epoch_seconds` into `(year, month, day, hour, minute, second)`
/// using Howard Hinnant's `civil_from_days` algorithm — Gregorian
/// proleptic, no leap-second adjustment (matches RFC 3339 §5.6).
fn break_down(epoch_seconds: u64) -> (u32, u32, u32, u32, u32, u32) {
    let day_seconds: u64 = 86_400;
    let days_since_epoch = epoch_seconds / day_seconds;
    let time_of_day = epoch_seconds.rem_euclid(day_seconds);
    let hour = u32::try_from(time_of_day / 3_600).unwrap_or(0);
    let minute = u32::try_from((time_of_day % 3_600) / 60).unwrap_or(0);
    let second = u32::try_from(time_of_day % 60).unwrap_or(0);
    let (year, month, day) = days_to_ymd(days_since_epoch);
    (year, month, day, hour, minute, second)
}

/// Converts days-since-1970-01-01 to Gregorian `(year, month, day)`.
/// Implementation adapted from Hinnant's `civil_from_days`.
fn days_to_ymd(days_since_epoch: u64) -> (u32, u32, u32) {
    let days = days_since_epoch.saturating_add(719_468);
    let era = days / 146_097;
    let doe = days.saturating_sub(era.saturating_mul(146_097));
    let numerator = doe
        .saturating_sub(doe / 1_460)
        .saturating_add(doe / 36_524)
        .saturating_sub(doe / 146_096);
    let yoe = numerator / 365;
    let y = yoe.saturating_add(era.saturating_mul(400));
    let subtrahend = 365_u64
        .saturating_mul(yoe)
        .saturating_add(yoe / 4)
        .saturating_sub(yoe / 100);
    let doy = doe.saturating_sub(subtrahend);
    let mp = 5_u64.saturating_mul(doy).saturating_add(2) / 153;
    let d = doy
        .saturating_sub(153_u64.saturating_mul(mp).saturating_add(2) / 5)
        .saturating_add(1);
    let m = if mp < 10 {
        mp.saturating_add(3)
    } else {
        mp.saturating_sub(9)
    };
    let year = if m <= 2 { y.saturating_add(1) } else { y };
    (
        u32::try_from(year).unwrap_or(0),
        u32::try_from(m).unwrap_or(1),
        u32::try_from(d).unwrap_or(1),
    )
}
