//! Monotonic millisecond clock abstraction used by the debouncer.
//!
//! Implements the testable timekeeping seam called for in
//! [LIVE-WATCHER]: the production clock reads the system wall clock,
//! the test clock returns externally-controlled values so debouncer
//! E2E tests do not depend on `sleep` or wall-clock timing
//! ([CLAUDE.md] testing rules).

use std::time::{SystemTime, UNIX_EPOCH};

/// Monotonic-ish millisecond clock. Implementations only need to be
/// stable for the duration of a single session — the debouncer does
/// not measure absolute time, only deltas.
pub trait Clock: Send + Sync + std::fmt::Debug {
    /// Returns the number of milliseconds since the UNIX epoch as a
    /// `u64`. Saturates on overflow rather than panicking.
    fn now_ms(&self) -> u64;
}

/// Production clock backed by [`std::time::SystemTime::now`]. Saturates
/// to `u64::MAX` if the host clock is implausibly far in the future
/// rather than panicking — every value the debouncer compares against
/// is also saturated, so the worst case is "the next flush fires
/// immediately" rather than a crash.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl SystemClock {
    /// Constructs a new system clock.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|delta| u64::try_from(delta.as_millis()).unwrap_or(u64::MAX))
            .unwrap_or(0)
    }
}
