//! E2E pins for [FUSED-CONTENT-GATE-RENAME]: a pair that is one code
//! written twice under a consistent renaming is admitted whatever its
//! literals do. Three shapes the content gate refused on the corpus, each
//! a real duplicate a reader judged as such:
//!
//! - `guzzle` `SetCookie`: sibling accessor pairs whose literal keys
//!   rename with the accessor (`'Name'` beside `getName`, `'Value'`
//!   beside `getValue`), so no literal is preserved and the pooled
//!   rename axis read `0.0`;
//! - `guzzle` `TlsVersion` against `CurlFactory`: one `if` chain mapping
//!   the same constants to a renamed family of return constants, each
//!   substitution seen once and so uncorroborated;
//! - `bloc` `edit_todo_event_test.dart`: two test groups renamed
//!   consistently, with the renamed name echoed in a literal.
//!
//! Each measured under the pooled support floor and was dropped before
//! closure. The identifier bijection is contradiction-free in all
//! three, which is the Type-2 definition, so all three must publish.

use anyhow::Result;

use crate::common::*;

/// The fixture and its files.
const FIXTURE: &str = "type2-rename-literal-drift";
const COOKIE: &str = "cookie.php";
const TLS_VERSION: &str = "tls_version.php";
const CURL_FACTORY: &str = "curl_factory.php";
const EVENT_TEST: &str = "edit_todo_event_test.dart";

/// The shipped node floor the corpus scans run at.
const MIN_NODES: u32 = 30;

/// The two accessor pairs of `cookie.php`, as line spans.
const NAME_ACCESSORS: (u64, u64) = (13, 26);
const VALUE_ACCESSORS: (u64, u64) = (31, 44);
/// The `if` chain in each handler file.
const IF_CHAIN: (u64, u64) = (9, 17);
/// The two renamed test groups.
const TITLE_GROUP: (u64, u64) = (6, 22);
const DESCRIPTION_GROUP: (u64, u64) = (24, 40);

/// A same-file rename whose literals drifted with it stays a clone.
#[test]
fn renamed_accessor_pairs_with_renamed_literal_keys_publish() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), MIN_NODES)?;
    assert_visible_clone_covering(&report, COOKIE, &[NAME_ACCESSORS, VALUE_ACCESSORS]);
    Ok(())
}

/// A cross-file rename made of one-shot substitutions stays a clone.
#[test]
fn renamed_constant_family_across_files_publishes() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), MIN_NODES)?;
    let clone = expect_cluster_spanning(&report, &[TLS_VERSION, CURL_FACTORY])?;
    for occurrence in occurrences(clone) {
        assert!(
            !occurrence_is_hidden(occurrence),
            "the renamed `if` chain is a visible clone: {clone:#}"
        );
    }
    for file in [TLS_VERSION, CURL_FACTORY] {
        assert_visible_clone_covering(&report, file, &[IF_CHAIN]);
    }
    Ok(())
}

/// Two consistently renamed test groups in one file stay a clone.
#[test]
fn renamed_test_groups_in_one_file_publish() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), MIN_NODES)?;
    assert_visible_clone_covering(&report, EVENT_TEST, &[TITLE_GROUP, DESCRIPTION_GROUP]);
    Ok(())
}

/// Asserts one visible cluster has an occurrence in `file` overlapping
/// every span in `spans` — the corpus scorecard's own match rule.
fn assert_visible_clone_covering(report: &serde_json::Value, file: &str, spans: &[(u64, u64)]) {
    let covered = clusters(report).iter().any(|cluster| {
        spans.iter().all(|span| {
            occurrences(cluster).iter().any(|occurrence| {
                let (start, end) = occurrence_line_span(occurrence);
                !occurrence_is_hidden(occurrence)
                    && occurrence_path(occurrence).is_ok_and(|path| path.ends_with(file))
                    && start <= span.1
                    && span.0 <= end
            })
        })
    });
    assert!(
        covered,
        "one visible cluster must cover every span {spans:?} of {file}: {report:#}"
    );
}
