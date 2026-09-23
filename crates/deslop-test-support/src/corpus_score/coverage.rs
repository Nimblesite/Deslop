//! [CORPUS-SCORE] The judged-line extent of a matched cluster.

use serde_json::Value;

use super::{
    clusters, has_distinct_matches, occurrence_key, overlaps, visible, Range, RangeCoverage,
};

/// Of the matching clusters, keep the one showing the most judged lines.
pub(super) fn matching_cluster<'a>(
    report: &'a Value,
    ranges: &[Range],
) -> Option<(&'a Value, RangeCoverage)> {
    clusters(report)
        .iter()
        .filter(|cluster| {
            cluster.get("id").and_then(Value::as_str).is_some()
                && has_distinct_matches(&visible(cluster), ranges)
        })
        .map(|cluster| (cluster, range_coverage(cluster, ranges)))
        .reduce(|best, next| {
            if next.1.covered_lines > best.1.covered_lines {
                next
            } else {
                best
            }
        })
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
