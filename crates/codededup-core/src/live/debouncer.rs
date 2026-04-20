//! Deterministic file-event debouncer ([LIVE-WATCHER]).
//!
//! Coalesces a burst of file events into a single re-analysis pass.
//! The window stays open for [`QUIET_MS`] of quiet time after the most
//! recent event, capped at [`CAP_MS`] from the first event so a
//! continuous stream of saves does not starve the scheduler. Time is
//! read through the injected [`Clock`] so the unit-of-coverage E2E
//! test can drive the debouncer without `sleep`.

use std::{collections::BTreeSet, path::PathBuf, sync::Arc};

use super::clock::Clock;

/// Quiet-window after the last event before [`Debouncer::ready_to_flush`]
/// reports `true`.
pub const QUIET_MS: u64 = 250;
/// Maximum total accumulation window, measured from the first event,
/// after which [`Debouncer::ready_to_flush`] also reports `true`.
pub const CAP_MS: u64 = 2_000;

/// Coalesces file paths and exposes a single drain point.
#[derive(Debug)]
pub struct Debouncer {
    /// Clock used for `quiet` and `cap` checks. `Arc<dyn Clock>` lets
    /// tests share a mock clock with assertions.
    clock: Arc<dyn Clock>,
    /// Pending file paths, deduplicated by `BTreeSet`.
    pending: BTreeSet<PathBuf>,
    /// Timestamp of the first pending event in the current window.
    first_event_ms: Option<u64>,
    /// Timestamp of the most recent pending event in the current window.
    last_event_ms: Option<u64>,
}

impl Debouncer {
    /// Constructs a debouncer reading time from `clock`.
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            pending: BTreeSet::new(),
            first_event_ms: None,
            last_event_ms: None,
        }
    }

    /// Records a path as pending. The debouncer keeps unique paths only.
    pub fn push(&mut self, path: PathBuf) {
        let now = self.clock.now_ms();
        if self.first_event_ms.is_none() {
            self.first_event_ms = Some(now);
        }
        self.last_event_ms = Some(now);
        let _inserted = self.pending.insert(path);
    }

    /// Returns `true` when either the quiet window has elapsed since
    /// the last event or the cap has elapsed since the first event.
    /// Returns `false` when no events are pending.
    #[must_use]
    pub fn ready_to_flush(&self) -> bool {
        let Some(last) = self.last_event_ms else {
            return false;
        };
        let now = self.clock.now_ms();
        let quiet_elapsed = now.saturating_sub(last) >= QUIET_MS;
        let cap_reached = self
            .first_event_ms
            .is_some_and(|first| now.saturating_sub(first) >= CAP_MS);
        quiet_elapsed || cap_reached
    }

    /// Returns whether at least one path is pending.
    #[must_use]
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Drains the pending set, resetting the window.
    pub fn flush(&mut self) -> Vec<PathBuf> {
        let drained: Vec<PathBuf> = self.pending.iter().cloned().collect();
        self.pending.clear();
        self.first_event_ms = None;
        self.last_event_ms = None;
        drained
    }
}
