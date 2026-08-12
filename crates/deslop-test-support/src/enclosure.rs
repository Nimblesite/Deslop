//! Physical-enclosure predicates over rendered occurrences
//! ([PIPELINE-CLUSTER-SUBSUME]).
//!
//! One physical duplication can be described by more than one cluster: a
//! duplicated method, and the run of single-statement clones nested
//! inside it. Both cover the same bytes, so overlap cannot say which is
//! the duplicate — enclosure can. When every occurrence of one cluster
//! sits inside an occurrence of another, the enclosing view *is* the
//! duplication and the nested view is a redundant re-description that
//! must not be published.
//!
//! Every suite that asserts this needs the same predicate, over reports
//! it holds in different shapes — rendered JSON in the CLI suites,
//! typed `ReportCluster` values in the refactor suites. The predicate
//! lives here once and both shapes convert into [`Span`].

/// One rendered occurrence reduced to the fields enclosure depends on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// Rendered occurrence path, compared verbatim.
    pub path: String,
    /// Inclusive start byte of the occurrence.
    pub start: u64,
    /// Exclusive end byte of the occurrence.
    pub end: u64,
}

impl Span {
    /// A span over `[start, end)` of `path`.
    #[must_use]
    pub fn new(path: impl Into<String>, start: u64, end: u64) -> Self {
        Self {
            path: path.into(),
            start,
            end,
        }
    }

    /// True when `inner` lies wholly inside `self` in the same file.
    #[must_use]
    pub fn contains(&self, inner: &Self) -> bool {
        self.path == inner.path && self.start <= inner.start && inner.end <= self.end
    }
}

/// Reads the enclosure-relevant fields off a rendered occurrence object.
#[must_use]
pub fn span_of(occurrence: &serde_json::Value) -> Option<Span> {
    Some(Span::new(
        occurrence.get("path")?.as_str()?,
        occurrence.get("start_byte")?.as_u64()?,
        occurrence.get("end_byte")?.as_u64()?,
    ))
}

/// Every occurrence span of a rendered cluster.
#[must_use]
pub fn spans_of(cluster: &serde_json::Value) -> Vec<Span> {
    cluster
        .get("occurrences")
        .and_then(serde_json::Value::as_array)
        .map(|occurrences| occurrences.iter().filter_map(span_of).collect())
        .unwrap_or_default()
}

/// True when every span of `nested` sits inside some span of
/// `enclosing`, and `enclosing` reaches beyond `nested` — the
/// redundant-view relation. Equal span sets are not enclosure.
#[must_use]
pub fn strictly_encloses(enclosing: &[Span], nested: &[Span]) -> bool {
    if enclosing.is_empty() || nested.is_empty() {
        return false;
    }
    let every_nested_inside = nested
        .iter()
        .all(|candidate| enclosing.iter().any(|outer| outer.contains(candidate)));
    let some_enclosing_outside = enclosing
        .iter()
        .any(|outer| !nested.iter().any(|inner| inner.contains(outer)));
    every_nested_inside && some_enclosing_outside
}

/// Describes the first pair of clusters where one is a nested
/// re-description of the other, or `None` when no pair is.
#[must_use]
pub fn first_nested_view(clusters: &[(String, Vec<Span>)]) -> Option<String> {
    clusters
        .iter()
        .enumerate()
        .find_map(|(index, (left_id, left))| {
            clusters
                .iter()
                .skip(index.saturating_add(1))
                .find_map(|(right_id, right)| nested_pair(left_id, left, right_id, right))
        })
}

/// The nested-view message for one ordered pair, in either direction.
fn nested_pair(left_id: &str, left: &[Span], right_id: &str, right: &[Span]) -> Option<String> {
    match (
        strictly_encloses(left, right),
        strictly_encloses(right, left),
    ) {
        (true, _) => Some(nested_message(right_id, left_id)),
        (_, true) => Some(nested_message(left_id, right_id)),
        _ => None,
    }
}

/// The single phrasing of the nested-view violation.
fn nested_message(nested_id: &str, enclosing_id: &str) -> String {
    format!(
        "cluster {nested_id} is nested inside {enclosing_id}; only the \
         enclosing view may be published"
    )
}
