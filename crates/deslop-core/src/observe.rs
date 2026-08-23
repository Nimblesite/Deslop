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
