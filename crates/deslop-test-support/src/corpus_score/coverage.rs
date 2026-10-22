//! [CORPUS-SCORE] The judged-line extent of a matched cluster.

use deslop_core::buckets::ClusterKind;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    clone_type::CloneType, clusters, has_distinct_matches, occurrence_key, overlaps, percent,
    visible, Range,
};

/// How much of a judged pair's source the matching cluster actually shows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeCoverage {
    /// Distinct judged lines overlapped by visible occurrences.
    pub covered_lines: u64,
    /// Inclusive lines across all judged ranges.
    pub judged_lines: u64,
    /// `100 * covered_lines / judged_lines`, absent for an empty entry.
    pub percent: Option<f64>,
}

impl RangeCoverage {
    /// Keep line coverage arithmetic in one place for pairs and aggregates.
    pub(super) fn new(covered_lines: u64, judged_lines: u64) -> Self {
        Self {
            covered_lines,
            judged_lines,
            percent: percent(
                usize::try_from(covered_lines).unwrap_or(usize::MAX),
                usize::try_from(judged_lines).unwrap_or(usize::MAX),
            ),
        }
    }
}

/// Both serialized report spellings of the cluster classification.
const TAXONOMY_FIELDS: &[&str] = &["kind", "bucket"];

/// Old reports call the cluster taxonomy `bucket`; current ones call it `kind`.
/// Either field can disprove a clone, and an absent or unknown taxonomy cannot
/// certify one ([CORPUS-SCORE]). A pinned type must also be one the kind
/// reports ([CORPUS-REGISTER-TYPE]).
fn is_clone_label(label: &Value, pinned: Option<CloneType>) -> bool {
    registered_kind(label)
        .is_some_and(|kind| kind.is_clone() && pinned.map_or(true, |pin| pin.admits(kind)))
}

/// The registered kind a classification label names, if it names one.
fn registered_kind(label: &Value) -> Option<ClusterKind> {
    label.as_str().and_then(|label| {
        ClusterKind::all()
            .into_iter()
            .find(|kind| kind.wire_label() == label)
    })
}

/// The classification labels a cluster carries, in either report schema.
fn labels(cluster: &Value) -> impl Iterator<Item = &Value> {
    TAXONOMY_FIELDS
        .iter()
        .filter_map(|field| cluster.get(*field))
}

/// [CORPUS-PRECISION-CURATED] Whether a cluster is positively informational:
/// it carries a classification, and every one it carries names a registered
/// kind that is not a clone. An absent or unrecognised classification proves
/// nothing, so it is never informational — a renamed label must fail a
/// non-duplicate claim, not silence it.
pub(crate) fn is_informational_finding(cluster: &Value) -> bool {
    labels(cluster).next().is_some()
        && labels(cluster).all(|label| registered_kind(label).is_some_and(|kind| !kind.is_clone()))
}

/// The first classification a cluster carries, as written; absent when it
/// carries none.
pub(crate) fn classification(cluster: &Value) -> Option<String> {
    labels(cluster).next().map(|label| {
        label
            .as_str()
            .map_or_else(|| label.to_string(), ToOwned::to_owned)
    })
}

/// Both report schemas must certify a clone when both fields are present.
pub(crate) fn is_clone_finding(cluster: &Value, pinned: Option<CloneType>) -> bool {
    let mut classified = false;
    for field in TAXONOMY_FIELDS {
        if let Some(label) = cluster.get(*field) {
            classified = true;
            if !is_clone_label(label, pinned) {
                return false;
            }
        }
    }
    classified
}

/// Of the matching clone clusters, keep the one showing the most judged lines.
fn matching_cluster<'a>(
    report: &'a Value,
    ranges: &[Range],
    pinned: Option<CloneType>,
) -> Option<(&'a Value, RangeCoverage)> {
    clusters(report)
        .iter()
        .filter(|cluster| {
            is_clone_finding(cluster, pinned)
                && cluster.get("id").and_then(Value::as_str).is_some()
                && has_distinct_matches(&visible(cluster), ranges)
        })
        .map(|cluster| (cluster, range_coverage(cluster, ranges)))
        .reduce(wider)
}

/// The match showing more judged lines; on a tie, the earlier in report order.
fn wider<'a>(
    best: (&'a Value, RangeCoverage),
    next: (&'a Value, RangeCoverage),
) -> (&'a Value, RangeCoverage) {
    if next.1.covered_lines > best.1.covered_lines {
        next
    } else {
        best
    }
}

/// What one judged entry found in a report.
pub(super) struct EntryMatch<'a> {
    /// The clone cluster that answered the entry, with its coverage.
    pub(super) cluster: Option<(&'a Value, RangeCoverage)>,
    /// For a typed entry nothing answered, the kind of the clone cluster that
    /// showed the pair anyway — what the engine called it instead.
    pub(super) reported_as: Option<String>,
}

/// Matches one entry's ranges, held to its pinned type when it has one.
pub(super) fn match_entry<'a>(
    report: &'a Value,
    ranges: &[Range],
    pinned: Option<CloneType>,
) -> EntryMatch<'a> {
    let cluster = matching_cluster(report, ranges, pinned);
    let reported_as = match (pinned, &cluster) {
        (Some(_), None) => reported_kind(report, ranges),
        _ => None,
    };
    EntryMatch {
        cluster,
        reported_as,
    }
}

/// The kind of the clone cluster that shows every range, whatever type it
/// reports.
fn reported_kind(report: &Value, ranges: &[Range]) -> Option<String> {
    let (cluster, _) = matching_cluster(report, ranges, None)?;
    TAXONOMY_FIELDS
        .iter()
        .find_map(|field| cluster.get(*field)?.as_str())
        .map(ToOwned::to_owned)
}

/// Clip one published occurrence to the lines the judge actually read.
fn clipped_overlap(occurrence: &Value, range: &Range) -> Option<(u64, u64)> {
    let (_, start, end, _, _) = occurrence_key(occurrence)?;
    overlaps(occurrence, range).then_some((
        start.max(u64::from(range.start)),
        end.min(u64::from(range.end)),
    ))
}

/// Union clipped spans so two overlapping occurrences never double-count.
fn covered_lines(shown: &[&Value], range: &Range) -> u64 {
    let mut spans = shown
        .iter()
        .filter_map(|occurrence| clipped_overlap(occurrence, range))
        .collect::<Vec<_>>();
    spans.sort_unstable();
    let (mut total, mut last_end) = (0_u64, 0_u64);
    for (start, end) in spans {
        let fresh_start = start.max(last_end.saturating_add(1));
        if fresh_start <= end {
            total = total.saturating_add(end.saturating_sub(fresh_start).saturating_add(1));
        }
        last_end = last_end.max(end);
    }
    total
}

/// Measure coverage after matching; it never changes a verdict or the gate.
fn range_coverage(cluster: &Value, ranges: &[Range]) -> RangeCoverage {
    let shown = visible(cluster);
    let judged_lines = ranges
        .iter()
        .map(|range| u64::from(range.end.saturating_sub(range.start)).saturating_add(1))
        .sum();
    let covered_lines = ranges
        .iter()
        .map(|range| covered_lines(&shown, range))
        .sum();
    RangeCoverage::new(covered_lines, judged_lines)
}
