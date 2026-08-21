//! [LIVE-DELTA] — every user-visible field of a cluster is part of
//! "this cluster changed".
//!
//! `ReportDelta::between` classifies a cluster whose id survived as
//! *updated* only when its payload differs, and the comparison used to
//! be a hand-written list of fields. The list had drifted: `bucket`,
//! `category`, `occurrences_total`, `occurrences_truncated`,
//! `intersects_diff` and `is_newly_introduced` were missing from it, so
//! were `agreement`, `rename_consistency` and `literal_fraction`, and
//! so were each occurrence's `start_line`, `end_line` and `in_diff`. A
//! cluster could change bucket, be re-categorised as data, gain a diff
//! tag or move to different lines and the delta would say nothing —
//! every live subscriber then renders the previous generation's answer
//! indefinitely, because a second identical generation produces no
//! delta either.
//!
//! This suite does not enumerate the fields either, and that is the
//! point: it reads the wire form of a rendered cluster, mutates **one
//! JSON leaf at a time**, and requires the delta to notice. A field
//! added to `docs/models/live-ipc.td` tomorrow is covered by this test
//! tomorrow, with nobody remembering to come back here.

use deslop_core::{
    report::{Report, ReportCluster},
    report_fixtures::{fixture_cluster, fixture_occurrence, fixture_report},
    ReportDelta,
};
use serde_json::{Map, Value};

/// The cluster id both generations carry, so every mutation is judged
/// as an *update* rather than an add plus a remove.
const CLUSTER_ID: &str = "c0ffee00c0ffee00";

/// The two occurrence paths of the fixture clone.
const OCCURRENCE_PATHS: [&str; 2] = ["src/alpha.rs", "src/beta.rs"];

/// The rendered cluster every mutation starts from. Its optional wire
/// fields are answered rather than omitted, so `intersects_diff`,
/// `is_newly_introduced` and each occurrence's `in_diff` are leaves the
/// walk below can actually reach.
fn baseline_cluster() -> ReportCluster {
    let mut cluster = fixture_cluster(
        CLUSTER_ID,
        OCCURRENCE_PATHS
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let mut occurrence = fixture_occurrence(path, 0, 64);
                occurrence.start_line = 1;
                occurrence.end_line = 8;
                occurrence.in_diff = Some(false);
                let _index = index;
                occurrence
            })
            .collect(),
    );
    cluster.intersects_diff = Some(false);
    cluster.is_newly_introduced = Some(false);
    cluster
}

/// A report carrying exactly `cluster`.
fn report_of(cluster: ReportCluster) -> Report {
    fixture_report(vec![cluster])
}

/// A value of the same JSON type as `value` that differs from it.
/// Returns `None` for a leaf with no distinguishable alternative, which
/// only `null` is — and the baseline answers every optional field so no
/// reachable leaf is `null`.
fn mutate_leaf(value: &Value) -> Option<Value> {
    match value {
        Value::Bool(flag) => Some(Value::Bool(!flag)),
        Value::Number(number) => number
            .as_f64()
            .map(|numeric| numeric + 1.0)
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number),
        Value::String(text) => Some(Value::String(format!("{text}-changed"))),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

/// Every JSON-pointer path to a scalar leaf inside `value`.
fn leaf_pointers(value: &Value, prefix: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(fields) => {
            for (name, child) in fields {
                leaf_pointers(child, &format!("{prefix}/{name}"), out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                leaf_pointers(child, &format!("{prefix}/{index}"), out);
            }
        }
        _ => out.push(prefix.to_owned()),
    }
}

/// Every `(pointer, whole document with that one leaf changed)` pair.
fn leaf_mutations(document: &Value) -> Vec<(String, Value)> {
    let mut pointers: Vec<String> = Vec::new();
    leaf_pointers(document, "", &mut pointers);
    pointers
        .into_iter()
        .filter_map(|pointer| replace_at(document, &pointer).map(|next| (pointer, next)))
        .collect()
}

/// `document` with the leaf at `pointer` replaced by a different value
/// of the same type.
fn replace_at(document: &Value, pointer: &str) -> Option<Value> {
    let mut next = document.clone();
    let slot = next.pointer_mut(pointer)?;
    *slot = mutate_leaf(slot)?;
    Some(next)
}

/// The cluster's wire form, as an object.
fn wire_form(cluster: &ReportCluster) -> Map<String, Value> {
    match serde_json::to_value(cluster) {
        Ok(Value::Object(fields)) => fields,
        _ => Map::new(),
    }
}

// Every scalar leaf of the rendered cluster — top level, inside
// `signals`, and inside each occurrence — makes the delta report the
// cluster updated. The pointer is named in every failure so the
// uncovered field is identified, not merely counted.
#[test]
fn changing_any_single_field_of_a_cluster_reports_it_updated() {
    let baseline = baseline_cluster();
    let previous = report_of(baseline.clone());
    let document = Value::Object(wire_form(&baseline));
    let mutations = leaf_mutations(&document);
    assert!(
        mutations.len() >= 20,
        "the walk must reach the whole rendered surface — top-level fields, \
         the eight signal axes and both occurrences. {count} leaves is too \
         few to be reading the real cluster: {document:#}",
        count = mutations.len(),
    );
    for (pointer, mutated) in mutations {
        let parsed = serde_json::from_value::<ReportCluster>(mutated.clone());
        assert!(
            parsed.is_ok(),
            "mutating {pointer} produced a cluster the wire cannot carry: {mutated:#}"
        );
        let Ok(changed) = parsed else { continue };
        let delta = ReportDelta::between(Some((1, &previous)), 2, &report_of(changed));
        assert_eq!(
            delta
                .clusters_updated
                .iter()
                .map(|cluster| cluster.id.as_str())
                .collect::<Vec<_>>(),
            vec![CLUSTER_ID],
            "{pointer} changed and the delta did not say so. A subscriber \
             told nothing keeps rendering the previous generation, and the \
             next identical generation produces no delta either, so the \
             stale view never heals: {delta:?}"
        );
        assert!(
            delta.clusters_added.is_empty() && delta.clusters_removed.is_empty(),
            "{pointer}: the id did not change, so this is an update and \
             neither an add nor a remove: {delta:?}"
        );
        assert!(
            !delta.is_empty(),
            "{pointer}: a non-empty delta must not report itself empty — \
             `is_empty` is what gates the `report/changed` notification"
        );
    }
}

// The other side of the contract: an unchanged generation is silent, so
// the comparison above is not simply answering "different" to
// everything.
#[test]
fn an_unchanged_cluster_produces_no_delta_at_all() {
    let previous = report_of(baseline_cluster());
    let delta = ReportDelta::between(Some((1, &previous)), 2, &report_of(baseline_cluster()));
    assert!(
        delta.is_empty(),
        "re-rendering the same cluster is not a change; firing \
         `report/changed` on it would wake every subscriber for nothing: \
         {delta:?}"
    );
    assert!(
        delta.clusters_updated.is_empty(),
        "an identical cluster is not updated: {delta:?}"
    );
}

// Dropping an occurrence is a user-visible change even when every
// remaining field matches — the array length is part of the payload,
// and a scalar-leaf walk cannot reach it.
#[test]
fn losing_an_occurrence_reports_the_cluster_updated() {
    let previous = report_of(baseline_cluster());
    let mut shrunk = baseline_cluster();
    let _removed = shrunk.occurrences.pop();
    let delta = ReportDelta::between(Some((1, &previous)), 2, &report_of(shrunk));
    assert_eq!(
        delta
            .clusters_updated
            .iter()
            .map(|cluster| cluster.id.as_str())
            .collect::<Vec<_>>(),
        vec![CLUSTER_ID],
        "a cluster that lost half its occurrences changed: {delta:?}"
    );
}
