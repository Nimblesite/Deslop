//! [CORPUS-SCOPE] Did the scan happen at all?
//!
//! Every other corpus check reads the clusters a report contains. None of
//! them can see the report that contains *nothing*: a scan that analysed
//! zero files renders cleanly, exits 0, and satisfies recall, precision,
//! confidence and ceilings at once, because each of those iterates a set
//! that is empty.
//!
//! That is not hypothetical. gh #342 shipped exactly it — a repository
//! under any folder named `dist`, `build` or `target` analysed as zero
//! files — and the corpus gate, the one instrument built to catch a total
//! false negative, watched it go past. The two checks here are the cheapest
//! assertions in the suite and they guard the most severe failure it has.
//!
//! Both curated inputs are **required**. A manifest that omits one fails the
//! gate: an absent bound is not a repository with no opinion about its own
//! size, it is a check that cannot fire, and [CORPUS-BASELINE] would read
//! that silence as evidence the defect is absent.

use serde_json::Value;

use crate::corpus::Failure;

/// [CORPUS-SCOPE] Asserts the scan reached the repository and produced a
/// cluster population inside its curated band.
pub fn check_scan_scope(manifest: &Value, report: &Value, failures: &mut Vec<Failure>) {
    check_files_analysed(manifest, report, failures);
    check_cluster_count_band(manifest, report, failures);
}

/// [CORPUS-SCOPE] `files_analysed` — the scan parsed a plausible number of
/// files, and never zero.
fn check_files_analysed(manifest: &Value, report: &Value, failures: &mut Vec<Failure>) {
    let Some(minimum) = manifest.get("expect_files_min").and_then(Value::as_u64) else {
        failures.push(Failure::new(
            "files_analysed",
            "manifest carries no `expect_files_min`, so nothing asserts the scan reached \
             the repository — a run that analysed zero files would pass every other check \
             in this suite (gh #342)",
        ));
        return;
    };
    let Some(analysed) = report.get("files_analysed").and_then(Value::as_u64) else {
        failures.push(Failure::new(
            "files_analysed",
            "the report has no `files_analysed` field, so the scan's reach cannot be judged",
        ));
        return;
    };
    if analysed < minimum {
        failures.push(Failure::new(
            "files_analysed",
            format!(
                "scan analysed {analysed} files, under the curated floor of {minimum}. \
                 Discovery lost part of the repository — an exclusion pattern, an \
                 extension map, or the whole tree (gh #342)"
            ),
        ));
    }
}

/// [CORPUS-SCOPE] `cluster_count_band` — the cluster population sits inside
/// its curated band.
///
/// A collapse means detection stopped finding duplicates; an explosion means
/// a filter or a threshold started manufacturing them. Both are repository-
/// wide swings no per-cluster check can see, because each of those judges
/// only the clusters that *are* there.
fn check_cluster_count_band(manifest: &Value, report: &Value, failures: &mut Vec<Failure>) {
    let Some(band) = manifest.get("expect_clusters") else {
        failures.push(Failure::new(
            "cluster_count_band",
            "manifest carries no `expect_clusters` band, so a repository-wide swing in \
             either direction would be printed rather than refused",
        ));
        return;
    };
    let (Some(min), Some(max)) = (
        band.get("min").and_then(Value::as_u64),
        band.get("max").and_then(Value::as_u64),
    ) else {
        failures.push(Failure::new(
            "cluster_count_band",
            "`expect_clusters` must carry numeric `min` and `max`",
        ));
        return;
    };
    let count = report
        .get("clusters")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let count = u64::try_from(count).unwrap_or(u64::MAX);
    if count < min || count > max {
        failures.push(Failure::new(
            "cluster_count_band",
            format!(
                "report renders {count} clusters, outside the curated band {min}..={max}. \
                 Below it, detection stopped finding duplicates; above it, something \
                 started manufacturing them"
            ),
        ));
    }
}

#[cfg(test)]
mod tests;
