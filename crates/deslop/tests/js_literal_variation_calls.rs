//! E2E pins for [CLONE-NOISE-LITERAL-VARIATION-CALLS] on JavaScript test
//! suites: a call whose argument carries statements is a body holder,
//! and a family of test cases is judged by what its bodies do, never by
//! their names.
//!
//! The fixture mirrors the `axios` adapter suites: two files each hold a
//! `describe('progress')` block wrapping an `upload` and a `download`
//! case, and the four `it(...)` bodies are one duplication under a
//! consistent rename. The same-file collapse publishes the widest view
//! of each region — the `describe` statement around each case — so the
//! filter meets wrappers whose only literal is a name. Read as
//! "`describe` varying one string", the family was hidden as
//! scaffolding and five real duplicates went with it.

use anyhow::Result;

use crate::common::*;

/// The fixture and its two suite files.
const FIXTURE: &str = "js-test-bodies-under-describe";
const FETCH_SUITE: &str = "fetch.test.js";
const HTTP_SUITE: &str = "http.test.js";

/// The four test bodies, as the line spans a reader would judge, in both
/// files alike: the fixture files differ only in the adapter name.
const UPLOAD_BODY: (u64, u64) = (7, 21);
const DOWNLOAD_BODY: (u64, u64) = (26, 40);

/// Nothing in this fixture is scaffolding, so nothing may be hidden.
const HIDDEN_CLUSTERS: u64 = 0;

/// The four bodies publish as one visible cross-file clone whose
/// occurrences cover every judged body, and no cluster is hidden.
#[test]
fn test_bodies_under_describe_wrappers_publish_as_one_clone() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), 30)?;
    let clone = expect_cluster_spanning(&report, &[FETCH_SUITE, HTTP_SUITE])?;
    for occurrence in occurrences(clone) {
        assert!(
            !occurrence_is_hidden(occurrence),
            "every occurrence of the test-body clone must be visible: {clone:#}"
        );
    }
    for file in [FETCH_SUITE, HTTP_SUITE] {
        for body in [UPLOAD_BODY, DOWNLOAD_BODY] {
            assert!(
                visible_occurrence_covers(&report, file, body),
                "a visible occurrence must cover {file}:{}-{} — the body is a \
                 real duplicate whatever its `it(...)` name says: {report:#}",
                body.0,
                body.1
            );
        }
    }
    assert_eq!(
        clusters_hidden(&report),
        HIDDEN_CLUSTERS,
        "no view of this fixture is literal-variation scaffolding: {report:#}"
    );
    Ok(())
}

/// Whether some visible occurrence in `file` overlaps the judged line
/// span — the same question the corpus scorecard asks of a report.
fn visible_occurrence_covers(report: &serde_json::Value, file: &str, span: (u64, u64)) -> bool {
    clusters(report).iter().any(|cluster| {
        occurrences(cluster).iter().any(|occurrence| {
            let (start, end) = occurrence_line_span(occurrence);
            !occurrence_is_hidden(occurrence)
                && occurrence_files_match(occurrence, file)
                && start <= span.1
                && span.0 <= end
        })
    })
}

/// Whether the occurrence's path names `file`.
fn occurrence_files_match(occurrence: &serde_json::Value, file: &str) -> bool {
    occurrence_path(occurrence).is_ok_and(|path| path.ends_with(file))
}
