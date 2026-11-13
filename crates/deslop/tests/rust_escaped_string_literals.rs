//! [FUSED-CONTENT-GATE-LITERAL-BOUND] [LANG-CAND-RUST-STRING-PARTS] Long Rust string literals
//! that share nothing but their escape sequences are not duplicates.
//!
//! On the Osprey repository Deslop grouped 16 unrelated strings as "Similar
//! code" — a CLI usage message, a JSON version line, an HTML sidebar and a
//! dozen embedded test programs — ranked #2 of 647 on its Rust crates (#626).
//! Their only common ground is a long run of `\n` and `\"` escapes. The
//! fixture keeps three of them verbatim: the usage text, a test's embedded
//! program and an HTML template.
//!
//! The positive control copies the usage text into a second constant, so the
//! suite also proves a genuinely copied string is still reported — and
//! reported on its own, without an unrelated string riding along.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde_json::Value;

use deslop_core::report::PairEndpoint;

use crate::common::{
    cluster_kind, cluster_spanning, cluster_touches, clusters, field, occurrences, run_report,
    seeded_fixture_root, signals::compare_endpoints, IDENTICAL_KIND, LOOSELY_SIMILAR_KIND,
    NEARLY_IDENTICAL_KIND,
};

const FIXTURE: &str = "rust-escaped-string-literals";
/// The subtree floor the live server used when it reported the incident.
const MIN_NODES: u32 = 30;
const USAGE_FILE: &str = "usage.rs";
const COPY_FILE: &str = "help.rs";
const UNRELATED_FILES: [&str; 2] = ["run_output.rs", "navigation.rs"];
const FIXTURE_FILES: u64 = 3;
const WITH_COPY_FILES: u64 = 4;
const COPY_OCCURRENCES: usize = 2;
const USAGE_NAME: &str = "const USAGE";
const COPY_NAME: &str = "const HELP";

/// The `let osp = "…"` statement and the usage constant's type and value:
/// the pair v0.42.0 reported, as `(file, start byte, end byte)`.
const EMBEDDED_PROGRAM: (&str, usize, usize) = ("run_output.rs", 270, 566);
const USAGE_TEXT: (&str, usize, usize) = (USAGE_FILE, 24, 1545);
const LITERAL_BOUND_REFUSAL: &str =
    "rejected: one string literal carries an endpoint and the texts differ";

/// The fixture contains excerpts from the Serde corpus pin 0db9b3b504bc
/// (MIT or Apache-2.0). These are the reported spans on the reduced fixture.
const SERDE_TOKEN_RUNS: &str = "rust-serde-token-runs";
type TokenRun = (&'static str, u64, u64);
const TAGGED_ENUM_RUNS: [TokenRun; 4] = [
    ("test_annotations.rs", 89, 120),
    ("test_annotations.rs", 130, 161),
    ("test_enum_internally_tagged.rs", 44, 77),
    ("test_enum_internally_tagged.rs", 122, 155),
];
const FLATTENED_ENUM_RUNS: [TokenRun; 3] = [
    ("issue2565.rs", 37, 46),
    ("test_annotations.rs", 34, 45),
    ("test_annotations.rs", 70, 81),
];

/// Every cluster the report holds, named by its files, for failure messages.
fn cluster_files(report: &Value) -> Vec<Vec<String>> {
    clusters(report)
        .iter()
        .map(|cluster| {
            occurrences(cluster)
                .iter()
                .filter_map(|occurrence| field(occurrence, "path").as_str().map(str::to_owned))
                .collect()
        })
        .collect()
}

/// Asserts no cluster pairs any of the unrelated strings with anything.
fn assert_unrelated_strings_stay_out(report: &Value, scenario: &str) {
    for file in UNRELATED_FILES {
        let offender = clusters(report)
            .iter()
            .find(|cluster| cluster_touches(cluster, file));
        assert!(
            offender.is_none(),
            "{scenario}: {file} holds a string unrelated to every other one, \
             yet it was grouped: {:?}",
            cluster_files(report),
        );
    }
}

/// Writes `help.rs`, a byte-identical copy of the usage string under a new name.
fn add_copied_string(scan_root: &Path) -> Result<()> {
    let source = scan_root.join(USAGE_FILE);
    let usage = fs::read_to_string(&source).context("read usage.rs")?;
    let copy = usage.replacen(USAGE_NAME, COPY_NAME, 1);
    assert_ne!(copy, usage, "the copy renames the constant");
    fs::write(scan_root.join(COPY_FILE), copy).context("write help.rs")
}

/// Three unrelated escape-heavy strings: nothing may be reported.
#[test]
fn unrelated_escaped_strings_are_never_grouped() -> Result<()> {
    let (_tmp, scan_root) = seeded_fixture_root(FIXTURE)?;
    let report = run_report(&scan_root, MIN_NODES)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FIXTURE_FILES)
    );
    assert_unrelated_strings_stay_out(&report, "unrelated strings only");
    assert!(
        clusters(&report).is_empty(),
        "no two of these strings share any text, so the report must be empty: {:?}",
        cluster_files(&report),
    );
    Ok(())
}

/// [FUSED-CONTENT-GATE-LITERAL-BOUND] Asked directly, the engine refuses the
/// pair for the reason the scan dropped it, and still measures every axis.
#[test]
fn explicit_comparison_refuses_the_unrelated_strings_by_name() -> Result<()> {
    let (_tmp, scan_root) = seeded_fixture_root(FIXTURE)?;
    let [left, right] =
        [EMBEDDED_PROGRAM, USAGE_TEXT].map(|(file, start_byte, end_byte)| PairEndpoint {
            path: scan_root.join(file),
            start_byte,
            end_byte,
        });
    let evidence = compare_endpoints(&scan_root, MIN_NODES, left, right)?.evidence;
    assert!(!evidence.admitted, "unrelated strings: {evidence:#?}");
    assert_eq!(evidence.explanation, LITERAL_BOUND_REFUSAL);
    assert_eq!(evidence.classification, None);
    assert!(
        evidence.agreement.abs() < f64::EPSILON,
        "the two statements share no authored text: {evidence:#?}"
    );
    assert!(
        evidence.structural > evidence.agreement,
        "shape alone made them look alike: {evidence:#?}"
    );
    Ok(())
}

/// A real copy of the usage string is reported, alone, as a copy.
#[test]
fn a_copied_string_is_reported_without_the_unrelated_ones() -> Result<()> {
    let (_tmp, scan_root) = seeded_fixture_root(FIXTURE)?;
    add_copied_string(&scan_root)?;
    let report = run_report(&scan_root, MIN_NODES)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(WITH_COPY_FILES)
    );
    let copy = cluster_spanning(&report, &[USAGE_FILE, COPY_FILE]).with_context(|| {
        format!(
            "the copied usage string must be reported: {:?}",
            cluster_files(&report)
        )
    })?;
    assert_eq!(
        occurrences(copy).len(),
        COPY_OCCURRENCES,
        "the copy group holds exactly the original and its copy: {:?}",
        cluster_files(&report),
    );
    assert!(
        [IDENTICAL_KIND, NEARLY_IDENTICAL_KIND].contains(&cluster_kind(copy)),
        "a byte-identical string under a renamed constant is a near-verbatim copy, got {}",
        cluster_kind(copy),
    );
    assert_unrelated_strings_stay_out(&report, "with a real copy present");
    Ok(())
}

/// [LANG-CAND-RUST-STRING-PARTS] String normalization must preserve token-run clones.
#[test]
fn serde_token_runs_keep_their_clone_membership_and_rank() -> Result<()> {
    const TAGGED_RANK: u64 = 1;
    const FLATTENED_RANK: u64 = 8;
    let (_tmp, root) = seeded_fixture_root(SERDE_TOKEN_RUNS)?;
    let report = run_report(&root, MIN_NODES)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FIXTURE_FILES)
    );
    assert_token_run_cluster(&report, &TAGGED_ENUM_RUNS, TAGGED_RANK)?;
    assert_token_run_cluster(&report, &FLATTENED_ENUM_RUNS, FLATTENED_RANK)
}

fn assert_token_run_cluster(report: &Value, expected: &[TokenRun], rank: u64) -> Result<()> {
    let cluster = clusters(report)
        .iter()
        .find(|cluster| expected.iter().all(|run| contains_token_run(cluster, *run)))
        .with_context(|| format!("missing Serde token-run cluster with {expected:?}"))?;
    assert_eq!(occurrences(cluster).len(), expected.len());
    assert_eq!(
        field(cluster, "occurrence_count").as_u64(),
        Some(expected.len() as u64)
    );
    assert_eq!(cluster_kind(cluster), LOOSELY_SIMILAR_KIND);
    assert_eq!(field(cluster, "rank").as_u64(), Some(rank));
    Ok(())
}

fn contains_token_run(cluster: &Value, (path, start, end): TokenRun) -> bool {
    occurrences(cluster).iter().any(|occurrence| {
        field(occurrence, "path").as_str() == Some(path)
            && field(occurrence, "start_line").as_u64() == Some(start)
            && field(occurrence, "end_line").as_u64() == Some(end)
    })
}
