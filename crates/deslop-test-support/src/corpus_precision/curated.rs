//! [CORPUS-PRECISION-CURATED] Every visible cluster spanning a curated
//! non-duplicate breaches it, unless it is positively informational. A
//! cluster with no classification, or one the scorer does not recognise,
//! breaches: the check fails hard rather than pass without judging.

use serde_json::Value;

use super::{array, field_u64, Failure, MASS, OCCURRENCE_COUNT};
use crate::{
    corpus::{cluster_shows_span, visible_clusters},
    corpus_score::{classification, is_informational_finding},
};

/// A pair needs two distinct file paths in this manifest contract.
const MIN_PATHS: usize = 2;
/// Curated entry fields, named once for validation and diagnostics.
const FILES_FIELD: &str = "files";
/// Human evidence carried into a breach.
const WHY_FIELD: &str = "why";
/// Stable precision check identifier.
const PRECISION_CHECK: &str = "precision";
/// Stable cluster identifier field.
const ID_FIELD: &str = "id";
/// Visible fallback for a malformed identifier.
const UNLABELLED: &str = "<unlabelled>";
/// How a breach names a cluster that carries no classification.
const UNCLASSIFIED: &str = "unclassified";

/// Judges one entry; only a positively informational cluster is excused.
pub(super) fn check_one(entry: &Value, report: &Value, failures: &mut Vec<Failure>) {
    let Some(files) = non_duplicate_paths(entry, failures) else {
        return;
    };
    let breach = visible_clusters(report)
        .into_iter()
        .filter(|cluster| !is_informational_finding(cluster))
        .find(|cluster| cluster_shows_span(cluster, &files));
    if let Some(breach) = breach {
        failures.push(Failure::new(
            PRECISION_CHECK,
            breach_detail(breach, &files, reason(entry)),
        ));
    }
}

/// A malformed entry fails instead of quietly asserting nothing.
fn non_duplicate_paths(entry: &Value, failures: &mut Vec<Failure>) -> Option<Vec<String>> {
    let files: Vec<String> = array(entry, FILES_FIELD)
        .iter()
        .filter_map(|file| file.as_str().map(ToOwned::to_owned))
        .collect();
    if files.len() >= MIN_PATHS {
        return Some(files);
    }
    failures.push(Failure::new(
        PRECISION_CHECK,
        malformed_entry(entry, files.len()),
    ));
    None
}

/// Explain the invalid assertion with its human curation evidence.
fn malformed_entry(entry: &Value, count: usize) -> String {
    let why = reason(entry);
    format!("`must_not_cluster` entry names {count} file(s); a non-duplication claim needs at least two paths or it asserts nothing. Curated: {why}")
}

/// The breaching cluster, its classification and its visible extent.
fn breach_detail(breach: &Value, files: &[String], why: &str) -> String {
    let id = breach
        .get(ID_FIELD)
        .and_then(Value::as_str)
        .unwrap_or(UNLABELLED);
    let kind = classification(breach).unwrap_or_else(|| UNCLASSIFIED.to_owned());
    let mass = field_u64(breach, MASS);
    let size = field_u64(breach, OCCURRENCE_COUNT);
    format!("cluster {id} ({kind}, mass {mass}, {size} occurrences) is shown spanning {files:?}, which a human verified is not duplication. Curated: {why}")
}

/// The entry's human-readable reason, as supplied by the corpus manifest.
fn reason(entry: &Value) -> &str {
    entry
        .get(WHY_FIELD)
        .and_then(Value::as_str)
        .unwrap_or_default()
}
