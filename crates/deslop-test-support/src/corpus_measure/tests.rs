//! [CORPUS-MEASURE] [CORPUS-CEILINGS] Reading a measurement back out of the
//! tool that took it, isolated.
//!
//! Every figure the corpus scorecard prints about cost passes through these
//! parsers. A parser that misreads a unit, or reads a missing figure as zero,
//! turns every ceiling and every comparison built on it into fiction.

use anyhow::Result;

use super::{
    binary_sha256, cpu_seconds, measurement, peak_cpu_percent, peak_rss_mb, CpuSample, Measurement,
    CPU_SAMPLE_WINDOW,
};

/// Peak memory both readings below describe.
const PEAK_MB: u64 = 7168;
/// That peak as GNU `time -v` and the Windows monitor print it.
const PEAK_KBYTES: u64 = PEAK_MB * 1024;
/// That peak as BSD `time -l` prints it.
const PEAK_BYTES: u64 = PEAK_KBYTES * 1024;
/// A scan's user and system CPU seconds, and their sum.
const USER_SECONDS: f64 = 341.25;
const SYSTEM_SECONDS: f64 = 12.5;
const TOTAL_CPU_SECONDS: f64 = USER_SECONDS + SYSTEM_SECONDS;
/// Busy cores in the busiest window of the peak-CPU fixtures, and the peak
/// that makes, in percent of one core.
const FOUR_CORES: u128 = 4;
const FOUR_CORES_PERCENT: f64 = 400.0;
/// The tolerance a parsed decimal is compared with.
const EPSILON: f64 = 1e-9;

/// GNU `/usr/bin/time -v` reports kbytes, and the label says so.
#[test]
fn a_gnu_peak_is_read_as_kbytes() -> Result<()> {
    let stderr = format!("\tMaximum resident set size (kbytes): {PEAK_KBYTES}\n");
    assert_eq!(
        peak_rss_mb(&stderr)?,
        PEAK_MB,
        "{PEAK_KBYTES} kbytes is {PEAK_MB} MB"
    );
    Ok(())
}

/// BSD `/usr/bin/time -l` reports bytes, with no unit in the label.
#[test]
fn a_bsd_peak_is_read_as_bytes() -> Result<()> {
    let stderr = format!("         {PEAK_BYTES}  maximum resident set size\n");
    assert_eq!(
        peak_rss_mb(&stderr)?,
        PEAK_MB,
        "{PEAK_BYTES} bytes is {PEAK_MB} MB"
    );
    Ok(())
}

/// A measurement that never appeared is an error, never a zero: a silent
/// zero would clear every memory ceiling in the corpus at once.
#[test]
fn a_missing_peak_is_an_error_rather_than_zero() {
    let outcome = peak_rss_mb("nothing useful here\n");
    let message = outcome
        .as_ref()
        .err()
        .map(ToString::to_string)
        .unwrap_or_default();
    assert!(
        outcome.is_err(),
        "a stderr with no peak line must not yield a number, got: {outcome:?}"
    );
    assert!(
        message.contains("maximum resident set size"),
        "the error must name what it could not find, got: {message}"
    );
}

/// BSD prints one summary line; the value sits before each unit word.
#[test]
fn bsd_cpu_time_is_user_plus_system() -> Result<()> {
    let stderr = format!("      295.04 real {USER_SECONDS} user {SYSTEM_SECONDS} sys\n");
    let parsed = cpu_seconds(&stderr)?;
    assert!(
        (parsed - TOTAL_CPU_SECONDS).abs() < EPSILON,
        "BSD CPU time must be user + sys = {TOTAL_CPU_SECONDS}, got {parsed}"
    );
    Ok(())
}

/// GNU `time -v` and the Windows monitor print the same labelled lines, so the
/// figure a Windows scan reports is read by exactly the parser Linux uses.
#[test]
fn gnu_and_windows_cpu_time_is_user_plus_system() -> Result<()> {
    let stderr = format!(
        "Maximum resident set size (kbytes): {PEAK_KBYTES}\n\
         User time (seconds): {USER_SECONDS}\n\
         System time (seconds): {SYSTEM_SECONDS}\n"
    );
    let parsed = cpu_seconds(&stderr)?;
    assert!(
        (parsed - TOTAL_CPU_SECONDS).abs() < EPSILON,
        "labelled CPU time must be user + system = {TOTAL_CPU_SECONDS}, got {parsed}"
    );
    assert_eq!(
        peak_rss_mb(&stderr)?,
        PEAK_MB,
        "the same reading carries the peak"
    );
    Ok(())
}

/// A reading with no CPU time is an error: a scan that appears to have cost
/// no CPU is a figure nobody measured.
#[test]
fn a_missing_cpu_time_is_an_error_rather_than_zero() {
    let stderr = format!("Maximum resident set size (kbytes): {PEAK_KBYTES}\n");
    let outcome = cpu_seconds(&stderr);
    let message = outcome
        .as_ref()
        .err()
        .map(ToString::to_string)
        .unwrap_or_default();
    assert!(
        outcome.is_err(),
        "no CPU line must not yield a number, got {outcome:?}"
    );
    assert!(
        message.contains("CPU time"),
        "the error must name what it could not find, got: {message}"
    );
}

/// A sample `window` windows into the scan, with `cpu_ms` consumed by then.
fn sample(window: u32, cpu_ms: u64) -> CpuSample {
    CpuSample {
        at: CPU_SAMPLE_WINDOW.saturating_mul(window),
        cpu_ms,
    }
}

/// CPU milliseconds four busy cores consume across one sampling window.
fn four_cores() -> u64 {
    u64::try_from(CPU_SAMPLE_WINDOW.as_millis().saturating_mul(FOUR_CORES)).unwrap_or(u64::MAX)
}

/// [CORPUS-MEASURE-PEAK-CPU] The peak is the busiest window, not the average:
/// a scan that runs one core, then four, then one reads 400%, where CPU time
/// over wall time would read 200%.
#[test]
fn peak_cpu_is_the_busiest_window_in_percent_of_one_core() {
    let one_core = u64::try_from(CPU_SAMPLE_WINDOW.as_millis()).unwrap_or(u64::MAX);
    let samples = [
        sample(0, 0),
        sample(1, one_core),
        sample(2, one_core + four_cores()),
        sample(3, one_core + four_cores() + one_core),
    ];
    let peak = peak_cpu_percent(&samples);
    assert!(
        peak.is_some_and(|percent| (percent - FOUR_CORES_PERCENT).abs() < EPSILON),
        "one, four, then one busy core must peak at 400%, got {peak:?}"
    );
}

/// [CORPUS-MEASURE-PEAK-CPU] When a process in the tree exits, its CPU time
/// leaves the sum. That fall is not a negative rate, and cannot become the
/// peak or drag it down.
#[test]
fn a_process_leaving_the_tree_reads_as_no_work_not_negative_work() {
    let samples = [sample(0, 0), sample(1, four_cores()), sample(2, 0)];
    let peak = peak_cpu_percent(&samples);
    assert!(
        peak.is_some_and(|percent| (percent - FOUR_CORES_PERCENT).abs() < EPSILON),
        "the busy window must stand as the peak, got {peak:?}"
    );
}

/// [CORPUS-MEASURE-PEAK-CPU] One sample is a moment, not a rate. A scan that
/// finished inside its first window has no peak, and says so.
#[test]
fn fewer_than_two_samples_have_no_peak() {
    assert_eq!(peak_cpu_percent(&[]), None, "no samples, no peak");
    assert_eq!(
        peak_cpu_percent(&[sample(0, 0)]),
        None,
        "one sample, no peak"
    );
}

/// [CORPUS-MEASURE] The fingerprint matches what `shasum -a 256` prints, so a
/// record written by the test harness and one written by the corpus scripts
/// name the same binary by the same string.
#[test]
fn the_binary_fingerprint_is_lowercase_hex_sha256() -> Result<()> {
    let file = tempfile::NamedTempFile::new()?;
    std::fs::write(file.path(), "abc")?;
    assert_eq!(
        binary_sha256(file.path())?,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "sha256(\"abc\") is the FIPS 180-2 test vector"
    );
    Ok(())
}

/// [CORPUS-CEILINGS] Every platform the corpus gate runs on must be able to
/// read a *true* peak. A platform with no measurement leaves every memory
/// ceiling in `corpus/*.json` unasserted while the suite still reports green.
#[test]
fn the_harness_measures_peak_rss_on_this_platform() {
    match measurement() {
        Measurement::PosixTime { flag } => assert!(
            flag == "-v" || flag == "-l",
            "`/usr/bin/time` takes `-v` (GNU) or `-l` (BSD); `{flag}` would be rejected and \
             kill every scan before a check ran"
        ),
        Measurement::WindowsProcessMonitor { script } => assert!(
            script.is_file(),
            "Windows has no `/usr/bin/time`, so the harness needs its process monitor at {}. \
             Without it every corpus test dies before it scans anything.",
            script.display()
        ),
    }
}
