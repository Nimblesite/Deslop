//! Routes through pair-content admission ([FUSED-CONTENT-GATE]).

use super::EXACT_CONTENT_AGREEMENT;
use crate::content::ContentEvidence;

/// The route one candidate edge took through the content guard.
pub(super) enum GateVerdict {
    /// Refused: the endpoints play different embedding roles.
    RoleMismatch,
    /// Refused: one whole authored function against a multi-function run
    /// ([FUSED-CONTENT-GATE-AUTHORED-RUN]).
    AuthoredScopeMismatch,
    /// Refused: a token-only pair merely wraps an exact function clone.
    ContainerEcho,
    /// Admitted without measurement: no content route saturated.
    NotRequired,
    /// Admitted from ordered frontier identity.
    ExactFrontier {
        /// The applicable pair-content floor.
        floor: f64,
    },
    /// Measured against the scope-specific floor.
    Measured {
        /// The pair's content evidence.
        evidence: ContentEvidence,
        /// The floor the evidence had to clear.
        floor: f64,
        /// A strongly aligned core already passed with only Async-suffix edits.
        verified_async_core: bool,
    },
}

impl GateVerdict {
    /// Whether the edge survives the guard.
    pub(super) fn admitted(&self) -> bool {
        match self {
            Self::RoleMismatch | Self::AuthoredScopeMismatch | Self::ContainerEcho => false,
            Self::NotRequired => true,
            Self::ExactFrontier { floor } => EXACT_CONTENT_AGREEMENT >= *floor,
            Self::Measured {
                evidence,
                floor,
                verified_async_core,
            } => evidence.clears(*floor) || *verified_async_core,
        }
    }

    /// The route's name for the trace.
    pub(super) fn route(&self) -> &'static str {
        match self {
            Self::RoleMismatch => "role_mismatch",
            Self::AuthoredScopeMismatch => "authored_scope_mismatch",
            Self::ContainerEcho => "container_echo",
            Self::NotRequired => "not_required",
            Self::ExactFrontier { .. } => "exact_frontier",
            Self::Measured { .. } => "measured",
        }
    }
}
