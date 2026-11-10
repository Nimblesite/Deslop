//! [CORPUS-CHECK-FIELDS] Every field of a rendered JSON report that a corpus
//! check reads, named once.
//!
//! A check reads a report only through [`ReportField`], so it cannot ask for
//! a field this list does not hold, and `crates/deslop/tests/corpus_report_fields.rs`
//! renders a real report and asserts it emits every one, in its shape. Reading
//! a field the renderer never writes is silent: the lookup comes back empty,
//! the check sees zero, and a breach reads as an empty cluster. `size` and
//! `category` were read that way, in two checks, for as long as nothing
//! asserted they existed (gh #540).

use serde_json::Value;

/// The report block that holds the [`FieldLevel::Metrics`] fields.
const METRICS: &str = "metrics";

/// Where in a rendered report a field sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldLevel {
    /// On the report itself.
    Report,
    /// On the report's `metrics` block.
    Metrics,
    /// On every entry of the report's `clusters`.
    Cluster,
    /// On every entry of a cluster's `occurrences`.
    Occurrence,
}

/// The JSON shape a check expects a field to hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldShape {
    /// An unsigned integer.
    Count,
    /// Any number.
    Number,
    /// An array.
    List,
    /// A string.
    Text,
    /// A boolean.
    Flag,
}

/// A rendered-report field a corpus check reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportField {
    /// How many files the scan analysed.
    FilesAnalysed,
    /// The published clusters.
    Clusters,
    /// Lines of code the scan analysed.
    AnalysedLoc,
    /// The repository's duplication percentage.
    DuplicationPercent,
    /// A cluster's stable id.
    Id,
    /// A cluster's one-based rank.
    Rank,
    /// A cluster's mass.
    Mass,
    /// How many occurrences a cluster shows.
    OccurrenceCount,
    /// How many occurrences a cluster carries, shown or not.
    OccurrencesTotal,
    /// The node count of a cluster's canonical occurrence.
    CanonicalNodeCount,
    /// A cluster's occurrences.
    Occurrences,
    /// A cluster's classification.
    Kind,
    /// An occurrence's file, relative to the scan root.
    Path,
    /// An occurrence's first line.
    StartLine,
    /// An occurrence's last line.
    EndLine,
    /// An occurrence's first byte.
    StartByte,
    /// An occurrence's end byte.
    EndByte,
    /// Whether an occurrence is hidden from the user.
    Hidden,
}

impl ReportField {
    /// Every field, so the enumeration test cannot miss one.
    pub const ALL: [Self; 18] = [
        Self::FilesAnalysed,
        Self::Clusters,
        Self::AnalysedLoc,
        Self::DuplicationPercent,
        Self::Id,
        Self::Rank,
        Self::Mass,
        Self::OccurrenceCount,
        Self::OccurrencesTotal,
        Self::CanonicalNodeCount,
        Self::Occurrences,
        Self::Kind,
        Self::Path,
        Self::StartLine,
        Self::EndLine,
        Self::StartByte,
        Self::EndByte,
        Self::Hidden,
    ];

    /// The field's name on the wire.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::FilesAnalysed => "files_analysed",
            Self::Clusters => "clusters",
            Self::AnalysedLoc => "analysed_loc",
            Self::DuplicationPercent => "duplication_percent",
            Self::Id => "id",
            Self::Rank => "rank",
            Self::Mass => "mass",
            Self::OccurrenceCount => "occurrence_count",
            Self::OccurrencesTotal => "occurrences_total",
            Self::CanonicalNodeCount => "canonical_node_count",
            Self::Occurrences => "occurrences",
            Self::Kind => "kind",
            Self::Path => "path",
            Self::StartLine => "start_line",
            Self::EndLine => "end_line",
            Self::StartByte => "start_byte",
            Self::EndByte => "end_byte",
            Self::Hidden => "hidden",
        }
    }

    /// Where the field sits.
    #[must_use]
    pub const fn level(self) -> FieldLevel {
        match self {
            Self::FilesAnalysed | Self::Clusters => FieldLevel::Report,
            Self::AnalysedLoc | Self::DuplicationPercent => FieldLevel::Metrics,
            Self::Id
            | Self::Rank
            | Self::Mass
            | Self::OccurrenceCount
            | Self::OccurrencesTotal
            | Self::CanonicalNodeCount
            | Self::Occurrences
            | Self::Kind => FieldLevel::Cluster,
            Self::Path
            | Self::StartLine
            | Self::EndLine
            | Self::StartByte
            | Self::EndByte
            | Self::Hidden => FieldLevel::Occurrence,
        }
    }

    /// The shape the checks read the field as.
    #[must_use]
    pub const fn shape(self) -> FieldShape {
        match self {
            Self::Clusters | Self::Occurrences => FieldShape::List,
            Self::Id | Self::Kind | Self::Path => FieldShape::Text,
            Self::Hidden => FieldShape::Flag,
            Self::DuplicationPercent => FieldShape::Number,
            Self::FilesAnalysed
            | Self::AnalysedLoc
            | Self::Rank
            | Self::Mass
            | Self::OccurrenceCount
            | Self::OccurrencesTotal
            | Self::CanonicalNodeCount
            | Self::StartLine
            | Self::EndLine
            | Self::StartByte
            | Self::EndByte => FieldShape::Count,
        }
    }

    /// The field's value on `value`, if present. A metrics field is read
    /// off the report, through its `metrics` block.
    #[must_use]
    pub fn of(self, value: &Value) -> Option<&Value> {
        match self.level() {
            FieldLevel::Metrics => value.get(METRICS)?.get(self.name()),
            FieldLevel::Report | FieldLevel::Cluster | FieldLevel::Occurrence => {
                value.get(self.name())
            }
        }
    }

    /// The field as a count, or `0` when absent.
    #[must_use]
    pub fn count(self, value: &Value) -> u64 {
        self.of(value).and_then(Value::as_u64).unwrap_or_default()
    }

    /// The field as a count, or `None` when absent.
    #[must_use]
    pub fn count_of(self, value: &Value) -> Option<u64> {
        self.of(value).and_then(Value::as_u64)
    }

    /// The field as a number, or `0.0` when absent.
    #[must_use]
    pub fn number(self, value: &Value) -> f64 {
        self.of(value).and_then(Value::as_f64).unwrap_or_default()
    }

    /// The field as text, if present.
    #[must_use]
    pub fn text(self, value: &Value) -> Option<&str> {
        self.of(value).and_then(Value::as_str)
    }

    /// The field's entries, or none when absent.
    #[must_use]
    pub fn items(self, value: &Value) -> &[Value] {
        crate::corpus::array(value, self.name())
    }

    /// True only when the field is present and `true`.
    #[must_use]
    pub fn is_set(self, value: &Value) -> bool {
        self.of(value).and_then(Value::as_bool) == Some(true)
    }

    /// True when the field is present in the shape the checks read it as.
    #[must_use]
    pub fn is_well_formed(self, value: &Value) -> bool {
        self.of(value).is_some_and(|field| match self.shape() {
            FieldShape::Count => field.is_u64(),
            FieldShape::Number => field.is_number(),
            FieldShape::List => field.is_array(),
            FieldShape::Text => field.is_string(),
            FieldShape::Flag => field.is_boolean(),
        })
    }
}
