//! [CORPUS-MEASURE] What one scan cost: wall time, CPU time, peak CPU and
//! peak memory, measured the same way on Windows, macOS and Linux.
//! Spec: `docs/specs/corpus.md` [CORPUS-MEASURE] and [CORPUS-CEILINGS].
//!
//! This is the one place a measured process is spawned. The corpus ceiling
//! suite and the [CORPUS-SCORE] scorecard both read their figures from here,
//! so the two can never disagree about what a scan cost.
//!
//! Peak memory and CPU time come from counters the kernel keeps for the
//! process, read once it exits, so neither can miss a spike: POSIX wraps the
//! scan in `/usr/bin/time`, and Windows — which has no such tool — has a
//! PowerShell monitor read the same counters for the scan's pid. Both leave
//! GNU-style labelled lines on stderr, so one parser reads every platform.
//!
//! Peak CPU is the one figure no kernel keeps, because it is a rate. It is
//! sampled by [`cpu`] through one cross-platform implementation, and the
//! window it is sampled over is part of its definition.

mod cpu;

use std::{
    ffi::OsString,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};

pub use cpu::{peak_cpu_percent, CpuSample, CPU_SAMPLE_WINDOW};

use crate::corpus::repo_root;

/// `/usr/bin/time` flag that reports peak resident set size. BSD (macOS)
/// spells it `-l`; GNU (Linux, which is what the scheduled corpus workflow
/// runs on) has no `-l` at all and rejects the invocation outright, so a
/// hard-coded `-l` would kill every scan before a single check ran.
const PEAK_RSS_FLAG: &str = if cfg!(target_os = "macos") {
    "-l"
} else {
    "-v"
};

/// The label every platform's peak-memory line carries.
const PEAK_RSS_LABEL: &str = "maximum resident set size";
/// Bytes in a kibibyte, and kibibytes in a mebibyte.
const KIBIBYTE: u64 = 1024;

/// How this platform measures a child process's cost.
///
/// [CORPUS-CEILINGS] needs a *true* peak, not a sampled one: a sample taken
/// every few hundred milliseconds is a lower bound, and a lower bound on a
/// ceiling assertion produces false passes. Both arms below read a counter
/// the kernel maintains, so neither can miss a spike.
#[derive(Debug)]
pub enum Measurement {
    /// POSIX: `/usr/bin/time <flag>` wraps the scan and reports the peak and
    /// the CPU time on stderr when it exits.
    PosixTime {
        /// The peak-RSS flag this platform's `time` accepts.
        flag: &'static str,
    },
    /// Windows has no `/usr/bin/time`. The scan is spawned directly and a
    /// PowerShell monitor reads `PeakWorkingSet64` — the OS's own
    /// monotonically increasing peak counter — and, once the scan exits, its
    /// user and kernel processor time.
    WindowsProcessMonitor {
        /// The monitor script this platform runs.
        script: PathBuf,
    },
}

/// The measurement this platform uses.
#[must_use]
pub fn measurement() -> Measurement {
    if cfg!(windows) {
        Measurement::WindowsProcessMonitor {
            script: windows_monitor_script(),
        }
    } else {
        Measurement::PosixTime {
            flag: PEAK_RSS_FLAG,
        }
    }
}

/// The PowerShell monitor that reports a pid's peak working set and CPU time.
fn windows_monitor_script() -> PathBuf {
    repo_root()
        .join("scripts")
        .join("corpus")
        .join("process-monitor.ps1")
}

/// What one measured process cost.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessCost {
    /// Wall-clock duration of the process.
    pub wall: Duration,
    /// Peak resident set size in mebibytes.
    pub peak_rss_mb: u64,
    /// User + system CPU seconds the process consumed.
    pub cpu_seconds: f64,
    /// The highest CPU use over any one [`CPU_SAMPLE_WINDOW`], in percent of
    /// one core. Absent when the process ended inside its first window, which
    /// leaves no interval to measure a rate over.
    pub peak_cpu_percent: Option<f64>,
}

/// One command run under this platform's measurement.
#[derive(Debug)]
pub struct MeasuredRun {
    /// Everything the process wrote, and how it exited.
    pub output: Output,
    /// What it cost.
    pub cost: ProcessCost,
}

/// Runs `program` with `args` under this platform's measurement.
///
/// # Errors
///
/// Returns an error when the process cannot be spawned, or when the
/// measurement reported no peak resident set size or no CPU time.
pub fn measured_run(program: &Path, args: &[OsString]) -> Result<MeasuredRun> {
    let started = Instant::now();
    let (output, reading, samples) = match measurement() {
        Measurement::PosixTime { flag } => posix_run(flag, program, args)?,
        Measurement::WindowsProcessMonitor { script } => windows_run(&script, program, args)?,
    };
    let wall = started.elapsed();
    let measured = format!("{}\n{reading}", String::from_utf8_lossy(&output.stderr));
    Ok(MeasuredRun {
        cost: ProcessCost {
            wall,
            peak_rss_mb: peak_rss_mb(&measured)?,
            cpu_seconds: cpu_seconds(&measured)?,
            peak_cpu_percent: peak_cpu_percent(&samples),
        },
        output,
    })
}

/// What one run left behind: its own output, whatever a separate monitor
/// read about it, and the CPU samples.
type RunOutcome = (Output, String, Vec<CpuSample>);

/// Runs the scan under `/usr/bin/time`, which reports the peak and the CPU
/// time itself on the scan's own stderr, while the sampler watches the
/// process tree it heads.
fn posix_run(flag: &str, program: &Path, args: &[OsString]) -> Result<RunOutcome> {
    let child = Command::new("/usr/bin/time")
        .arg(flag)
        .arg(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn /usr/bin/time")?;
    let cpu_sampler = cpu::Sampler::start(child.id());
    let output = child.wait_with_output().context("scan did not complete")?;
    Ok((output, String::new(), cpu_sampler.finish()?))
}

/// Runs the scan directly and has PowerShell read its kernel counters.
///
/// The monitor takes only a pid, so no path has to survive a shell quoting
/// round-trip. Its reading is kept apart from the scan's own stderr, so a
/// crashed scan's message is the scan's alone.
fn windows_run(script: &Path, program: &Path, args: &[OsString]) -> Result<RunOutcome> {
    let child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {}", program.display()))?;
    let cpu_sampler = cpu::Sampler::start(child.id());
    let monitor = spawn_process_monitor(script, child.id())?;
    let output = child.wait_with_output().context("scan did not complete")?;
    let rate_readings = cpu_sampler.finish()?;
    Ok((output, monitor_reading(monitor)?, rate_readings))
}

/// What the monitor printed once the scan exited.
fn monitor_reading(monitor: std::process::Child) -> Result<String> {
    let reading = monitor
        .wait_with_output()
        .context("process monitor did not complete")?;
    Ok(String::from_utf8_lossy(&reading.stdout).into_owned())
}

/// Starts the PowerShell monitor watching `process_id`.
fn spawn_process_monitor(script: &Path, process_id: u32) -> Result<std::process::Child> {
    Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script)
        .arg("-ProcessId")
        .arg(process_id.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {}", script.display()))
}

/// [CORPUS-CEILINGS] Extracts peak RSS in mebibytes from `/usr/bin/time -l`
/// (BSD/macOS, bytes) or GNU-style `(kbytes)` output. The unit is decided by
/// the label itself rather than by the host, so a mislabelled build cannot be
/// silently misread by three orders of magnitude.
pub(crate) fn peak_rss_mb(stderr: &str) -> Result<u64> {
    let line = stderr
        .lines()
        .find(|line| line.to_ascii_lowercase().contains(PEAK_RSS_LABEL))
        .ok_or_else(|| anyhow!("the measurement reported no {PEAK_RSS_LABEL}"))?;

    let value: u64 = line
        .split_whitespace()
        .find_map(|token| token.parse().ok())
        .ok_or_else(|| anyhow!("no numeric peak RSS in {line:?}"))?;

    let in_kbytes = line.to_ascii_lowercase().contains("kbytes");
    Ok(if in_kbytes {
        value / KIBIBYTE
    } else {
        value / (KIBIBYTE * KIBIBYTE)
    })
}

/// [CORPUS-MEASURE] Extracts CPU seconds (user + system) in either dialect.
///
/// BSD prints one summary line — `0.53 real 0.42 user 0.09 sys`; GNU, and the
/// Windows monitor, print labelled `User time (seconds): 0.42` lines. A reading
/// with neither shape is an error, never zero: a scan that appears to have
/// cost nothing is a claim nobody measured.
pub(crate) fn cpu_seconds(stderr: &str) -> Result<f64> {
    bsd_cpu_seconds(stderr)
        .or_else(|| gnu_cpu_seconds(stderr))
        .ok_or_else(|| anyhow!("the measurement reported no user and system CPU time"))
}

/// The BSD summary line: the value sits immediately before its unit word.
fn bsd_cpu_seconds(stderr: &str) -> Option<f64> {
    let line = stderr
        .lines()
        .find(|line| line.contains(" user") && line.contains(" sys"))?;
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let before = |unit: &str| {
        tokens
            .iter()
            .position(|token| *token == unit)
            .and_then(|at| at.checked_sub(1))
            .and_then(|at| tokens.get(at))
            .and_then(|token| token.parse::<f64>().ok())
    };
    Some(before("user")? + before("sys")?)
}

/// The GNU labelled lines: the value is whatever follows the final colon.
fn gnu_cpu_seconds(stderr: &str) -> Option<f64> {
    let labelled = |prefix: &str| {
        stderr
            .lines()
            .find(|line| line.trim().to_ascii_lowercase().starts_with(prefix))
            .and_then(|line| line.rsplit(':').next())
            .and_then(|value| value.trim().parse::<f64>().ok())
    };
    Some(labelled("user time")? + labelled("system time")?)
}

/// The sha256 of the file at `path`, in lowercase hex — the fingerprint every
/// timing record carries, so a figure always names the binary that produced
/// it. The same digest `shasum -a 256` prints, which is what the corpus
/// scripts record for the builds they measure.
///
/// # Errors
///
/// Returns an error naming `path` when it cannot be read.
pub fn binary_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("unreadable: {}", path.display()))?;
    Ok(Sha256::digest(&bytes)
        .iter()
        .fold(String::new(), |mut hex, byte| {
            // Writing into a `String` cannot fail.
            let _written = write!(hex, "{byte:02x}");
            hex
        }))
}

#[cfg(test)]
mod tests;
