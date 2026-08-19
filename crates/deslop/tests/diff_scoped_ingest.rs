//! Diff *ingest* over the committed `diff-scope` fixture
//! ([CLI-ARG-DIFF]): a diff that does not match the scanned tree, a
//! diff that is not valid diff text, a diff file that is not there, and
//! the stdin form.
//!
//! Split from `diff_scoped_reporting.rs`, which asserts what a *valid*
//! diff makes the report say. These four assert what the CLI does with
//! diff input it cannot honour — refusals are usage errors (exit 2),
//! never analysis failures, and never a silently empty scope.

mod common;

use std::fs;

use crate::common::{diff_scope::*, field, fixture, load_json, Result};

// [CLI-ARG-DIFF] verification: a diff that does not byte-match the
// scanned tree is refused as a usage error naming file and line.
#[test]
fn stale_diff_is_refused_with_file_and_line() -> Result<()> {
    let (_output, stderr, _tmp) = run_code(&["--diff", "patches/stale.patch"], 2)?;
    assert!(
        stderr.contains("caller.rs"),
        "refusal must name the mismatching file: {stderr}"
    );
    assert!(
        stderr.contains('6'),
        "refusal must name the mismatching new-side line: {stderr}"
    );
    Ok(())
}

// [CLI-ARG-DIFF] grammar: malformed diff text and a missing diff file
// are usage errors (exit 2), never analysis failures.
#[test]
fn malformed_or_missing_diff_is_a_usage_error() -> Result<()> {
    let (_output, stderr, _tmp) = run_code(&["--diff", "patches/malformed.patch"], 2)?;
    assert!(
        stderr.contains("diff"),
        "malformed refusal must say what failed: {stderr}"
    );
    let _missing = run_code(&["--diff", "patches/does-not-exist.patch"], 2)?;
    Ok(())
}

// [CLI-ARG-DIFF] `--diff -` reads the unified diff from stdin.
#[test]
fn diff_from_stdin_tags_identically() -> Result<()> {
    let patch = fs::read_to_string(fixture("diff-scope").join("patches/change.patch"))?;
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let mut cmd = diff_cmd(&output, &["--diff", "-"])?;
    let _assert = cmd.write_stdin(patch).assert().success();
    let report = load_json(&output.with_extension("json"))?;
    let fresh = cluster_with_paths(&report, &["src/fresh_a.rs", "src/fresh_b.rs"])?;
    assert_eq!(field(fresh, "is_newly_introduced"), true);
    assert_eq!(
        field(field(field(&report, "metrics"), "diff"), "added_loc"),
        ADDED_LOC
    );
    Ok(())
}
