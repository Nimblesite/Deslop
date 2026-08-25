//! Shared observability arithmetic ([PIPELINE-OBSERVABILITY-STAGES]).
//!
//! Saturating event counters and elapsed-time conversions used by the
//! aggregate stage events. One definition so every stage reports time
//! and counts the same way.

use std::time::{Duration, Instant};

/// Adds one to a saturating event counter.
pub fn bump(counter: &mut u64) {
    *counter = counter.saturating_add(1);
}

/// Elapsed whole milliseconds since `started`, saturating.
#[must_use]
pub fn elapsed_ms(started: Instant) -> u64 {
    duration_ms(started.elapsed())
}

/// A duration as saturated whole milliseconds.
#[must_use]
pub fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// The process's current resident set, in MiB, from the platform's own
/// accounting (`ps`). `None` when the query fails. Called at stage
/// boundaries only — never in a hot path — so a corpus-scale run can
/// attribute memory to stages ([PERF-FLUTTER-TODO-MEMORY]).
#[must_use]
pub fn resident_mib() -> Option<u64> {
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let kilobytes: u64 = text.trim().parse().ok()?;
    Some(kilobytes / 1024)
}
