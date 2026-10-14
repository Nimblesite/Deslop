//! [CORPUS-REGISTER-TYPE] The clone type a judge may pin on a CLEARLY IN.
//!
//! A judge who is certain a pair is, say, renamed code (Type II) can say so.
//! The pair is then found only by a cluster whose kind reports that type; an
//! engine that publishes renamed code as byte-identical has answered a
//! different question. A blank type pins nothing, so any clone kind finds it.

use anyhow::{anyhow, Result};
use deslop_core::buckets::ClusterKind;
use serde_json::Value;

use super::CLEARLY_IN;

/// The register field a judge pins the type in.
pub const TYPE_FIELD: &str = "type";

/// The research clone types ([CLONE-TYPE-TAXONOMY]) a judge can pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneType {
    /// Identical apart from whitespace and comments.
    One,
    /// The same structure with renamed identifiers, types or literals.
    Two,
    /// A copy with added, removed or modified statements.
    Three,
    /// Equivalent behaviour written as different code.
    Four,
}

impl CloneType {
    /// Every type, in taxonomy order.
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [Self::One, Self::Two, Self::Three, Self::Four]
    }

    /// The spelling a judge writes and a register stores.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::One => "Type I",
            Self::Two => "Type II",
            Self::Three => "Type III",
            Self::Four => "Type IV",
        }
    }

    /// Whether a cluster of `kind` reports this type. The engine splits Type
    /// III between nearly identical and similar code ([CLONE-KIND-LABELS]),
    /// and shape-only findings report no type at all.
    #[must_use]
    pub fn admits(self, kind: ClusterKind) -> bool {
        let reported: &[Self] = match kind {
            ClusterKind::Identical => &[Self::One],
            ClusterKind::NearlyIdentical => &[Self::Two, Self::Three],
            ClusterKind::LooselySimilar => &[Self::Three],
            ClusterKind::SameBehavior => &[Self::Four],
            ClusterKind::StructuralOnly => &[],
        };
        reported.contains(&self)
    }

    /// The type an entry under `verdict` pins, or `None` when it is blank.
    ///
    /// # Errors
    ///
    /// Refuses a label outside the four, and any type on a verdict other than
    /// CLEARLY IN: a pair that is not a clone has no clone type, and scoring
    /// one anyway would quietly narrow what counts as reporting it.
    pub fn pinned(entry: &Value, verdict: &str) -> Result<Option<Self>> {
        let label = match entry.get(TYPE_FIELD) {
            None | Some(Value::Null) => return Ok(None),
            Some(Value::String(label)) => label.trim(),
            Some(other) => return Err(anyhow!("register type is not text: {other}")),
        };
        if label.is_empty() {
            return Ok(None);
        }
        if verdict != CLEARLY_IN {
            return Err(anyhow!("a {verdict} entry names no clone, so it has no type"));
        }
        Self::all()
            .into_iter()
            .find(|clone_type| clone_type.label() == label)
            .map(Some)
            .ok_or_else(|| anyhow!("register type `{label}` is not Type I, II, III or IV"))
    }
}
