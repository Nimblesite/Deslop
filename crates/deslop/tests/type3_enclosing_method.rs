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
//! # Four of the five languages are red on purpose (GH #408)
//!
//! `csharp-type3` passes and gates the release. The other four do not,
//! and the reason is **admission, not subsumption**: no subsumption
//! order can elect a pair that was never built. Every one of the four
//! is dropped by `pair::survival_decision` at `DroppedBelowFused` —
//! `bounded_fused()` is `max(structural, token_jaccard, embedding_cos)`,
//! the LSH path writes a literal `structural = 0.0`, embeddings are off,
//! and the exact k-gram Jaccard between the two whole methods is short
//! of [`FUSED_THRESHOLD`] 0.85:
//!
//! | fixture | method nodes | exact Jaccard | admitted? |
//! |---|---|---|---|
//! | `csharp-type3` | 58 / 52 | 0.8519 | yes — renders at 0.92 |
//! | `dart-type3` | 56 / 49 | 0.8431 | no |
//! | `ts-type3-stmt` | 50 / 44 | 0.8067 | no |
//! | `go-type3` | 53 / 48 | 0.7755 | no |
//! | `python-type3` | 37 / 31 | 0.7429 | no |
//!
//! C# clears the bar only because its `namespace`/`class` scaffolding
//! dilutes the one-statement delta. The `MinHash` estimate is not the
//! cause: it reads 0.80 against an exact 0.8431 on Dart, and the exact
//! value is still short.
//!
//! The evidence the pipeline discards is structural. `pair.rs` documents
//! `structural_sim` as "the best-achievable subtree overlap", but
//! `candidates::add_lsh_pairs` writes a literal `0.0` for every
//! cross-bucket pair — while the unchanged statements inside these
//! methods are Merkle-identical, which is exactly why the fragment views
//! survive. Maximal shared-subtree coverage over the larger method:
//! dart 0.87, go 0.86, csharp 0.84, python 0.82, ts 0.81.
//!
//! Closing it means measuring that overlap at admission **and** at
//! render, plus a [CLONE-BUCKETS-ROUTING] row for "high structural
//! overlap, moderate token overlap". Rendered `structural` is binary
//! Merkle equality today and `buckets::is_lsh_only_nearmiss` requires
//! `structural <= 0.01`, so making it non-binary *without* that matching
//! row would demote `csharp-type3` to `loosely_similar` — which the
//! renderer hides — and take the one working language with it. That is a
//! signal-semantics change needing its own assertions and a corpus
//! re-measure, tracked on #408.
//!
//! Every assertion below is intact for all five languages. Run the four
//! with `cargo test -p deslop --test type3_enclosing_method -- --ignored`.

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
