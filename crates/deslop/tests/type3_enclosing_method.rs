//! Black-box regression for the five-language Type-3 recall hole
//! (#408, [PIPELINE-CLUSTER-SUBSUME], [REPAIR-SUBSUME-CONTENT-FIRST]): one
//! inserted statement must not hide a whole-method clone behind its own
//! fragments.
//!
//! Each fixture pairs two methods that are identical except for a
//! single inserted statement. The insertion rehashes every ancestor
//! Merkle node, so the enclosing pair carries `structural = 0.0` while
//! saturated fragments nested inside it carry `structural = 1.0` by
//! construction. Pre-#367, destructive subsumption elected on that raw
//! geometry and deleted the method pair before content was measured;
//! the report then showed only `structural_only` fragments — or, for
//! `ts-type3-stmt`, nothing at all.
//!
//! The enforceable statement: the *enclosing* method pair is the
//! visible cluster, and its occurrence set names the whole method in
//! both files. The occurrence set is asserted, not only the bucket,
//! because the nested fragments span the same two files and would
//! satisfy a bucket-only or file-set-only assertion.
//!
//! # How all five came to pass (GH #408)
//!
//! Four of the five once failed here, and the cause was **admission,
//! not subsumption**: no election order can elect a pair that was never
//! built. `bounded_fused()` is `max(structural, token_jaccard,
//! embedding_cos)`, the LSH path wrote a literal `structural = 0.0`,
//! embeddings are off, and the exact whole-method k-gram Jaccard falls
//! short of `FUSED_THRESHOLD` 0.85 in every language but C#:
//!
//! | fixture | method nodes | exact Jaccard | measured overlap |
//! |---|---|---|---|
//! | `csharp-type3` | 58 / 52 | 0.8519 | 0.898 |
//! | `dart-type3` | 56 / 49 | 0.8431 | 0.877 |
//! | `ts-type3-stmt` | 48 / 42 | 0.8067 | 0.875 |
//! | `go-type3` | 53 / 48 | 0.7755 | 0.906 |
//! | `python-type3` | 37 / 31 | 0.7429 | 0.842 |
//!
//! C# cleared the bar on tokens alone only because its
//! `namespace`/`class` scaffolding dilutes the one-statement delta. The
//! `MinHash` estimate was never the cause: it reads 0.80 against an
//! exact 0.8431 on Dart, and the exact value is still short.
//!
//! The discarded evidence was structural. `pair.rs` documented
//! `structural_sim` as "the best-achievable subtree overlap" while
//! writing `0.0` for every cross-bucket pair — even though the
//! unchanged statements inside these methods stay Merkle-identical,
//! which is exactly why the fragment views survived. `structural` is
//! now that overlap, measured by ordered tree alignment
//! ([FUSION-SHARED-SUBTREE]); [CLONE-BUCKETS-ROUTING] row 4b routes it
//! on the same two floors that admit the pair.
//!
//! A second defect sat behind the first and only became visible once
//! the pairs were admitted: [PIPELINE-CLUSTER-SUBSUME] nominated the
//! enclosing view in one direction only, so when the enclosing view was
//! also the heavier one a byte-identical fragment nested inside it
//! deleted it on a higher `structural`. `ts_type3_one_inserted_statement`
//! is the case that catches it — with only the admission fix, that
//! fixture's report is empty.
//!
//! Both halves are load-bearing. Reverting either takes the whole
//! enclosing pair out of the report in at least one language.

use anyhow::Result;
use serde_json::Value;

mod common;
use crate::common::*;

/// The whole-method span the surviving cluster must cover in one file.
struct MethodSpan {
    path: &'static str,
    first_line: u64,
    last_line: u64,
}

/// Reads a 1-based line field from a report occurrence.
fn occurrence_line(occurrence: &Value, key: &str) -> u64 {
    occurrence.get(key).and_then(Value::as_u64).unwrap_or(0)
}

/// Finds the occurrence in `cluster` that covers the whole method span.
fn covering_occurrence<'a>(cluster: &'a Value, span: &MethodSpan) -> Option<&'a Value> {
    occurrences(cluster).iter().find(|occurrence| {
        occurrence.get("path").and_then(Value::as_str) == Some(span.path)
            && occurrence_line(occurrence, "start_line") <= span.first_line
            && occurrence_line(occurrence, "end_line") >= span.last_line
    })
}

/// Finds the visible cluster whose occurrences cover both method spans.
fn enclosing_pair_cluster<'a>(
    report: &'a Value,
    left: &MethodSpan,
    right: &MethodSpan,
) -> Option<&'a Value> {
    clusters(report).iter().find(|cluster| {
        covering_occurrence(cluster, left).is_some()
            && covering_occurrence(cluster, right).is_some()
    })
}

/// No other visible cluster may name either file: a fragment published
/// beside the method pair re-describes bytes the pair already reports.
fn assert_fragments_absorbed(report: &Value, survivor: &Value, files: [&str; 2]) {
    for other in clusters(report) {
        if cluster_id(other) == cluster_id(survivor) {
            continue;
        }
        let named = occurrence_paths(other);
        assert!(
            !files
                .iter()
                .any(|file| named.iter().any(|path| path == file)),
            "a fragment view is still visible beside the enclosing method pair: {other:#}"
        );
    }
}

/// The full #408 contract for one language fixture.
fn assert_enclosing_pair_visible(name: &str, left: &MethodSpan, right: &MethodSpan) -> Result<()> {
    let report = run_report(&fixture(name), 8)?;
    let Some(cluster) = enclosing_pair_cluster(&report, left, right) else {
        anyhow::bail!(
            "#408: the enclosing method pair {}:{}-{} / {}:{}-{} is not a visible \
             cluster; only fragment views survived subsumption: {report:#}",
            left.path,
            left.first_line,
            left.last_line,
            right.path,
            right.first_line,
            right.last_line,
        );
    };
    assert_eq!(
        cluster_size(cluster),
        2,
        "the method pair must span exactly two occurrences: {cluster:#}"
    );
    assert_eq!(
        cluster_bucket(cluster),
        "nearly_identical",
        "a one-statement Type-3 near-miss must render as a credible near-identical \
         clone, not a demoted shape match: {cluster:#}"
    );
    assert!(
        signal(cluster, "fused") >= 0.6,
        "the pair's confidence must reach the reuse band ([FUSED-THRESHOLD]): {cluster:#}"
    );
    assert_fragments_absorbed(&report, cluster, [left.path, right.path]);
    Ok(())
}

/// Shorthand for the span table below.
const fn span(path: &'static str, first_line: u64, last_line: u64) -> MethodSpan {
    MethodSpan {
        path,
        first_line,
        last_line,
    }
}

#[test]
fn csharp_type3_reports_the_enclosing_method_pair() -> Result<()> {
    assert_enclosing_pair_visible(
        "csharp-type3",
        &span("Delta.cs", 5, 18),
        &span("Epsilon.cs", 5, 17),
    )
}

#[test]

fn dart_type3_reports_the_enclosing_method_pair() -> Result<()> {
    assert_enclosing_pair_visible(
        "dart-type3",
        &span("delta.dart", 1, 11),
        &span("epsilon.dart", 1, 10),
    )
}

#[test]

fn go_type3_reports_the_enclosing_method_pair() -> Result<()> {
    assert_enclosing_pair_visible(
        "go-type3",
        &span("delta.go", 3, 13),
        &span("epsilon.go", 3, 12),
    )
}

#[test]

fn python_type3_reports_the_enclosing_method_pair() -> Result<()> {
    assert_enclosing_pair_visible(
        "python-type3",
        &span("alpha.py", 1, 8),
        &span("beta.py", 1, 7),
    )
}

#[test]

fn ts_type3_one_inserted_statement_must_not_erase_the_method_pair() -> Result<()> {
    assert_enclosing_pair_visible(
        "ts-type3-stmt",
        &span("pointBoard.ts", 1, 12),
        &span("scoreBoard.ts", 1, 11),
    )
}
