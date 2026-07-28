//! [EXCLUSION-CONFIG] — a committed `vendor/` tree must never enter
//! discovery.
//!
//! `go mod vendor` writes every dependency's source into `vendor/` at the
//! module root, and unlike `node_modules` or `.cargo` that directory is
//! *conventionally committed*: GitHub's `Go.gitignore` ships the
//! `vendor/` rule commented out, and the Kubernetes / Docker / etcd
//! lineage of enterprise Go repositories all vendor by policy. It is also
//! not dot-prefixed, so the discovery walk's hidden-directory filter does
//! not skip it and the `ignore` crate's gitignore pass has no rule to
//! apply.
//!
//! Left in, those files are parsed, fingerprinted, ranked — and because
//! ranking is worst-offenders-first ([RANK-SCORE]), third-party
//! duplication the user cannot act on outranks every first-party finding.
//! On a real vendored repository that is the difference between a usable
//! report and an unusable one, which is why this sits alongside the #142
//! `.cargo` guard in `showstoppers.rs` rather than in a per-language
//! suite. Composer (PHP) and `cargo vendor` (Rust) write to the same
//! directory name and are covered by the same exclusion.
//!
//! Black-box: run the CLI over `go-vendored/`, whose first-party pair
//! sits at the root and whose three vendored files carry six blatantly
//! identical functions.

use anyhow::Result;
use serde_json::Value;

mod common;
use crate::common::*;

/// Path components of a reported occurrence path, split on both
/// separators so the assertion holds on Windows runners too.
fn path_components(path: &str) -> Vec<&str> {
    path.split(['/', '\\']).collect()
}

#[test]
fn committed_go_vendor_tree_is_excluded_from_discovery() -> Result<()> {
    let report = run_report(&fixture("go-vendored"), 8)?;

    assert_eq!(
        report.get("files_analysed").and_then(Value::as_u64),
        Some(2),
        "only the two first-party files (main.go, helper.go) may be analysed; the three \
         files under vendor/example.com/ are third-party source. report={report}",
    );

    let reported = clusters(&report);
    assert!(
        !reported.is_empty(),
        "the first-party pair is a genuine clone and must still be reported — an empty \
         report would satisfy the vendor guard below without proving anything. \
         report={report}",
    );

    for cluster in reported {
        for path in occurrence_paths(cluster) {
            assert!(
                !path_components(&path).contains(&"vendor"),
                "a vendored dependency leaked into a rendered cluster: {path}",
            );
            assert!(
                !path.contains("example.com"),
                "a vendored module path leaked into a rendered cluster: {path}",
            );
        }
    }

    // The surviving cluster is the first-party pair, so the user's worst
    // offender is code they own and can actually deduplicate.
    let first_party: Vec<String> = reported
        .iter()
        .flat_map(occurrence_files)
        .collect::<std::collections::BTreeSet<String>>()
        .into_iter()
        .collect();
    assert_eq!(
        first_party,
        vec!["helper.go".to_owned(), "main.go".to_owned()],
        "the ranked report must be exactly the first-party clone pair",
    );
    Ok(())
}
