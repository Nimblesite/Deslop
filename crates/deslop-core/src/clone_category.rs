//! Clone category — the "logic vs. data" axis of a cluster ([RANK-CATEGORY]).
//!
//! Orthogonal to the similarity bucket of [`crate::buckets::ClusterKind`]:
//! the bucket says *how similar* the copies are, the category says *whether
//! the repetition is extractable logic or un-refactorable data*. Detection of
//! the `data` category lives in [`crate::cluster_filters`]
//! ([CLONE-NOISE-DART-DATA-TABLE-LITERAL]); the ranking policy that consumes
//! it lives in [`crate::config::RankingPolicy`].
//!
//! This module is the single source of truth for the wire label, the
//! human-readable chip, and the category-specific action hint, so every
//! renderer (text, HTML, VSIX tree via JSON) agrees without re-deriving
//! strings.
//!
//! [CLONE-CATEGORY-REGISTRY] The shipped registry is `Logic` + `DataTable`; the
//! five literal-family categories and `CloneCategory::all()` in the spec table
//! ship with the planned literal feature ([LITERAL-CATEGORY], literals.md).

/// Whether a cluster's repetition is extractable logic or un-refactorable
/// data. Travels to the report on `ReportCluster.category` ([RANK-CATEGORY]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CloneCategory {
    /// Ordinary duplicated code. Full ranking weight; the default.
    Logic,
    /// A data-structure literal repeated across sibling elements — a
    /// `List`/`Set`/`Map` of near-identical constructor / record / map
    /// literals. Real repetition, but the constructor's purpose is to
    /// enumerate per-row fields, so it is demoted (or dropped) rather than
    /// presented as an egregious DRY violation
    /// ([CLONE-NOISE-DART-DATA-TABLE-LITERAL]).
    DataTable,
}

impl CloneCategory {
    /// Every shipped variant in canonical registry order. Facet surfaces
    /// iterate this instead of hand-listing values ([FACET-MODEL] rule 1);
    /// the literal-family variants join here when [LITERAL-CATEGORY] ships.
    #[must_use]
    pub const fn all() -> [Self; 2] {
        [Self::Logic, Self::DataTable]
    }

    /// Plain group title for facet and grouping surfaces: the shared
    /// [`Self::chip`] when one exists, or `"Code clones"` for the
    /// chip-less logic category ([FACET-GROUP-BY-TYPE]).
    #[must_use]
    pub fn group_title(self) -> &'static str {
        match self.chip() {
            Some(chip) => chip,
            None => "Code clones",
        }
    }

    /// Stable `snake_case` wire label carried on `ReportCluster.category`
    /// so agents can pattern-match without deserialising the enum. The
    /// `data` value reconstructs the human-readable chip and action hint
    /// for any downstream surface ([RANK-CATEGORY]).
    #[must_use]
    pub const fn wire_label(self) -> &'static str {
        match self {
            Self::Logic => "logic",
            Self::DataTable => "data",
        }
    }

    /// Parses the stable JSON `category` wire label, defaulting to
    /// [`Self::Logic`] for absent or unknown values so older reports and
    /// the common case both resolve cleanly.
    #[must_use]
    pub fn from_wire_label(label: &str) -> Self {
        match label {
            "data" => Self::DataTable,
            _ => Self::Logic,
        }
    }

    /// Short human chip shown on text / HTML surfaces next to the bucket
    /// title. `None` for [`Self::Logic`] — the absence of a chip already
    /// communicates "ordinary logic clone", so no surface adds noise for
    /// the common case.
    #[must_use]
    pub const fn chip(self) -> Option<&'static str> {
        match self {
            Self::Logic => None,
            Self::DataTable => Some("data table"),
        }
    }

    /// Category-specific refactoring recommendation. `data` tables are not
    /// "extract the duplicate" candidates — the fix is a builder with
    /// default arguments or moving the rows to an asset file.
    #[must_use]
    pub const fn action_sentence(self) -> &'static str {
        match self {
            Self::Logic => "Extract the duplicated logic into a shared function.",
            Self::DataTable => {
                "Repeated data rows, not duplicated logic. Consider a builder with default \
                 arguments, or move the rows to a JSON/CSV/asset file."
            }
        }
    }
}
