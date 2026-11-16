//! Where a session is in the base-protocol lifecycle ([LSP-LIFECYCLE]).

use std::sync::atomic::{AtomicU8, Ordering};

/// Stored form of [`Phase::Uninitialized`].
const UNINITIALIZED: u8 = 0;
/// Stored form of [`Phase::Initializing`].
const INITIALIZING: u8 = 1;
/// Stored form of [`Phase::Initialized`].
const INITIALIZED: u8 = 2;
/// Stored form of [`Phase::ShutDown`].
const SHUT_DOWN: u8 = 3;

/// Lifecycle phase of one editor session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    /// No `initialize` request has arrived.
    Uninitialized,
    /// `initialize` is being answered.
    Initializing,
    /// `initialize` succeeded; the session does work.
    Initialized,
    /// `shutdown` arrived; only `exit` is still honoured.
    ShutDown,
}

impl Phase {
    /// Whether the server may send notifications and requests of its own.
    pub(crate) fn accepts_work(self) -> bool {
        matches!(self, Self::Initialized | Self::ShutDown)
    }

    /// The value stored for this phase.
    fn stored(self) -> u8 {
        match self {
            Self::Uninitialized => UNINITIALIZED,
            Self::Initializing => INITIALIZING,
            Self::Initialized => INITIALIZED,
            Self::ShutDown => SHUT_DOWN,
        }
    }

    /// The phase a stored value names.
    fn from_stored(stored: u8) -> Self {
        match stored {
            INITIALIZING => Self::Initializing,
            INITIALIZED => Self::Initialized,
            SHUT_DOWN => Self::ShutDown,
            _ => Self::Uninitialized,
        }
    }
}

/// The session's phase, shared between the session and its client handle.
#[derive(Debug, Default)]
pub(crate) struct PhaseCell(AtomicU8);

impl PhaseCell {
    /// Current phase.
    pub(crate) fn get(&self) -> Phase {
        Phase::from_stored(self.0.load(Ordering::SeqCst))
    }

    /// Moves to `phase`.
    pub(crate) fn set(&self, phase: Phase) {
        self.0.store(phase.stored(), Ordering::SeqCst);
    }

    /// Moves from `from` to `to`; false when the session was elsewhere.
    pub(crate) fn advance(&self, from: Phase, to: Phase) -> bool {
        self.0
            .compare_exchange(
                from.stored(),
                to.stored(),
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
    }
}
