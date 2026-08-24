//! Negative-control E2E tests for JavaScript / TypeScript
//! ([PIPELINE-BOILERPLATE-FILTER], [CLONE-TYPE-TAXONOMY]).
//!
//! Detection is only trustworthy if it stays quiet on code that is not
//! duplicated. Genuinely different functions must not cluster, and an
//! identical import prologue shared across files must be suppressed as
//! module boilerplate rather than reported as duplicate logic — the same
//! treatment C#, Rust, Python, and Dart imports already receive.

use anyhow::Result;

use crate::common::*;

#[test]
fn javascript_distinct_functions_produce_no_clusters() -> Result<()> {
    let report = run_report(&fixture("js-distinct-functions"), 15)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(4),
        "all four distinct-function files must be analysed: {report:#}"
    );
    assert!(
        clusters(&report).is_empty(),
        "unrelated functions must not be reported as clones: {report:#}"
    );
    Ok(())
}

#[test]
fn javascript_import_prologue_is_suppressed_not_clustered() -> Result<()> {
    let report = run_report(&fixture("js-import-boilerplate"), 12)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(3),
        "all three route files must be analysed: {report:#}"
    );
    // The three route files share an identical six-line `import` prologue
    // and `const router = express.Router();`. Their actual route-handler
    // bodies (.get vs .post, 404 vs 422) genuinely differ. With JS import
    // boilerplate suppressed, the shared prologue no longer surfaces and the
    // divergent bodies never cluster, so the report is clean.
    assert!(
        clusters(&report).is_empty(),
        "the shared import prologue must be suppressed, not reported as a clone: {report:#}"
    );
    // Belt and braces: no route-handler body marker ever appears in a clone.
    assert!(
        summaries_where(&report, &fixture("js-import-boilerplate"), |text| {
            text.contains("db.orders.create") || text.contains("db.users.findById")
        })?
        .is_empty(),
        "divergent route-handler logic must not be reported as duplication: {report:#}"
    );
    Ok(())
}
