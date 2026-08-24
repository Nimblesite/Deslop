//! [PIPELINE-DETERMINISM] Did two scans of one corpus state agree?
//!
//! The gate used to compare the two runs' **ordered cluster id vectors**
//! and nothing else. Ids are derived from the smallest member's hash
//! ([PIPELINE-CLUSTER-EXACT]), so they survive almost everything a
//! non-deterministic pipeline can do to a report: two runs could agree on
//! every id while disagreeing on occurrence ranges, buckets, signals,
//! rank order, hidden flags, `duplication_percent` and `files_analysed`,
//! and the gate would call that reproducible. It also dropped any cluster
//! whose `id` would not read as a string, so a malformed report compared
//! equal to a healthy one by having fewer entries on both sides.
//!
//! What a user gets is the whole report, so the whole report is what has
//! to be reproducible. This module compares the full rendered payload,
//! removing only the fields that describe the *run* rather than the
//! corpus, and names the first field that moved so a failure is
//! diagnosable without dumping two whole-repository reports.

use serde_json::Value;

use crate::corpus::Failure;

/// The check id a disagreement is recorded under, matching the key in
/// `corpus/known-failures.json`.
const DETERMINISM_CHECK: &str = "determinism";

/// Report fields two scans of one corpus state are allowed to disagree
/// on. `cache_stats` counts what the second run read from the first
/// run's warm fingerprint cache: a property of run history, not of the
/// corpus. Nothing else on the wire is exempt.
const RUN_HISTORY_FIELDS: &[&str] = &["cache_stats"];

/// How deep a disagreement is described before the path is reported as
/// it stands. Deep enough to name a signal inside an occurrence inside a
/// cluster, which is the finest thing a reader needs pointed at.
const MAX_DISAGREEMENT_DEPTH: usize = 8;

/// [PIPELINE-DETERMINISM] Records a failure unless the two reports are
/// the same report, field for field, once run-history fields are set
/// aside.
pub fn check_reports_agree(first: &Value, second: &Value, failures: &mut Vec<Failure>) {
    for report in [first, second] {
        if let Some(defect) = malformed(report) {
            failures.push(Failure::new(DETERMINISM_CHECK, defect));
            return;
        }
    }
    let (left, right) = (comparable(first), comparable(second));
    if let Some(where_) = first_disagreement(&left, &right, "", 0) {
        failures.push(Failure::new(
            DETERMINISM_CHECK,
            format!(
                "two identical scans disagreed at {where_}. No report, no ranking and no \
                 --fail-over gate is reproducible while this is true (#301)"
            ),
        ));
    }
}

/// Why a report cannot be compared at all, if it cannot. A cluster
/// without a string `id` is malformed input rather than a cluster to
/// skip: skipping it is what let two disagreeing reports compare equal.
fn malformed(report: &Value) -> Option<String> {
    let clusters = report.get("clusters").and_then(Value::as_array)?;
    clusters.iter().enumerate().find_map(|(rank, cluster)| {
        cluster
            .get("id")
            .and_then(Value::as_str)
            .is_none()
            .then(|| format!("the cluster at rank {rank} carries no string `id`: {cluster}"))
    })
}

/// The report without its run-history fields — the part two scans of one
/// corpus state must reproduce exactly.
fn comparable(report: &Value) -> Value {
    let mut comparable = report.clone();
    if let Some(object) = comparable.as_object_mut() {
        for field in RUN_HISTORY_FIELDS {
            let _removed = object.remove(*field);
        }
    }
    comparable
}

/// The first place the two values differ, as a readable path plus both
/// sides' values. `None` when they are equal.
fn first_disagreement(left: &Value, right: &Value, path: &str, depth: usize) -> Option<String> {
    if left == right {
        return None;
    }
    if depth >= MAX_DISAGREEMENT_DEPTH {
        return Some(describe(path, left, right));
    }
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            object_disagreement(left, right, path, depth)
        }
        (Value::Array(left), Value::Array(right)) => array_disagreement(left, right, path, depth),
        _ => Some(describe(path, left, right)),
    }
}

/// The first key two objects disagree on, including a key one of them
/// does not have at all.
fn object_disagreement(
    left: &serde_json::Map<String, Value>,
    right: &serde_json::Map<String, Value>,
    path: &str,
    depth: usize,
) -> Option<String> {
    let mut keys: Vec<&String> = left.keys().chain(right.keys()).collect();
    keys.sort_unstable();
    keys.dedup();
    keys.into_iter().find_map(|key| {
        let child = format!("{path}.{key}");
        match (left.get(key), right.get(key)) {
            (Some(left), Some(right)) => {
                first_disagreement(left, right, &child, depth.saturating_add(1))
            }
            (left, right) => Some(format!(
                "{child}: present on one side only ({left:?} vs {right:?})"
            )),
        }
    })
}

/// The first index two arrays disagree on, or their differing lengths.
fn array_disagreement(left: &[Value], right: &[Value], path: &str, depth: usize) -> Option<String> {
    if left.len() != right.len() {
        return Some(format!("{path}: {} entries vs {}", left.len(), right.len()));
    }
    left.iter()
        .zip(right.iter())
        .enumerate()
        .find_map(|(index, (left, right))| {
            first_disagreement(
                left,
                right,
                &format!("{path}[{index}]"),
                depth.saturating_add(1),
            )
        })
}

/// One disagreement rendered for a human: where, and both sides.
fn describe(path: &str, left: &Value, right: &Value) -> String {
    let at = if path.is_empty() { "the report" } else { path };
    format!("{at}: {left} vs {right}")
}

#[cfg(test)]
mod tests;
