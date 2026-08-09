//! End-to-end regression coverage for #331 / #336 — shape-only
//! saturation ([FUSION-STRATEGY-MAX-SUM], [RANK-STRUCTURAL-ONLY]).
//!
//! Both showstoppers share one mechanism: identifier/literal
//! normalisation makes `token_jaccard` a restatement of `structural`,
//! and sum-then-clamp fusion pins `fused` at 1.0 for every structural
//! match. Distinct-content, same-shape families (mandatory Flutter
//! widget declarations, unrelated numeric array literals) are then
//! reported as act-now duplication (`identical` / `nearly_identical`,
//! `fused = 1.0`) and outrank genuine clones.
//!
//! Correct behaviour pinned here, per issue #331/#336 acceptance:
//! - a genuine copy-pasted clone is still found at full confidence
//!   (recall guard), and ranks above every shape-only family;
//! - a shape-only family must not be reported at act-now confidence:
//!   `fused` stays below the [FUSED-THRESHOLD] act-now line and the
//!   bucket is an honest non-act-now one (or the cluster is suppressed
//!   outright). Complementary category labelling (`data`) is #336's
//!   follow-up ask and is deliberately not asserted here.

use serde_json::Value;

mod common;
use crate::common::*;

/// The agent-facing act-now line ([FUSED-THRESHOLD]): `find-similar`
/// consumers refuse to write code at or above this fused confidence.
const ACT_NOW_FUSED: f64 = 0.85;

/// Buckets that honestly describe a shape-only match without telling
/// the reader (human or agent) that the content is duplicated.
const HONEST_SHAPE_ONLY_BUCKETS: [&str; 2] = ["structural_only", "loosely_similar"];

/// Distinct Flutter widgets: same mandatory declaration shape, different
/// names and different `build` bodies — the #331 false-positive family.
const WIDGETS: [(&str, &str); 6] = [
    ("AlphaPanel", "Text(\"hello\")"),
    (
        "BetaBadge",
        "Column(children: [Text(\"a\"), Text(\"b\"), Icon(Icons.add)])",
    ),
    (
        "GammaTile",
        "Container(width: 12, height: 30, color: Colors.red)",
    ),
    (
        "DeltaCard",
        "ListView(padding: EdgeInsets.zero, shrinkWrap: true)",
    ),
    (
        "EpsilonRow",
        "Row(mainAxisSize: MainAxisSize.min, children: [Icon(Icons.close)])",
    ),
    (
        "ZetaChip",
        "Stack(fit: StackFit.expand, alignment: Alignment.center)",
    ),
];

/// A genuine copy-pasted Dart function — byte-identical across two files.
const DART_GENUINE_CLONE: &str = "int weightedTotal(List<int> values, int floor) {\n\
    \x20 if (values.isEmpty) {\n\
    \x20   return floor;\n\
    \x20 }\n\
    \x20 int total = 0;\n\
    \x20 for (final int value in values) {\n\
    \x20   if (value > floor) {\n\
    \x20     total = total + value * 2;\n\
    \x20   } else {\n\
    \x20     total = total - 1;\n\
    \x20   }\n\
    \x20 }\n\
    \x20 return total;\n\
    }\n";

/// One Flutter widget file: the framework-mandated `StatefulWidget`
/// declaration (the shared shape) plus a state class whose `build`
/// body is unique per widget so only the declarations align.
fn dart_widget_file(name: &str, body: &str) -> String {
    format!(
        "class {name} extends StatefulWidget {{\n\
         \x20 const {name}({{super.key}});\n\
         \x20 @override\n\
         \x20 State<{name}> createState() => _{name}State();\n\
         }}\n\n\
         class _{name}State extends State<{name}> {{\n\
         \x20 @override\n\
         \x20 Widget build(BuildContext context) {{\n\
         \x20   return {body};\n\
         \x20 }}\n\
         }}\n"
    )
}

/// Asserts the genuine byte-identical clone spanning `files` is still
/// reported at full confidence — the recall half that keeps a precision
/// fix from silencing real clones — and returns its zero-based rank.
fn assert_genuine_clone_rank(report: &Value, files: &[&str]) -> Result<usize> {
    let clone = expect_cluster_spanning(report, files)?;
    assert_eq!(
        cluster_bucket(clone),
        "identical",
        "byte-identical clone across {files:?} must stay bucketed identical: {report:#}"
    );
    assert!(
        signal(clone, "fused") >= ACT_NOW_FUSED,
        "byte-identical clone across {files:?} must keep act-now confidence: {report:#}"
    );
    assert!(
        approx(signal(clone, "token_jaccard"), 1.0),
        "byte-identical clone across {files:?} must keep a full token signal: {report:#}"
    );
    let rank = clusters(report)
        .iter()
        .position(|cluster| std::ptr::eq(cluster, clone))
        .ok_or_else(|| anyhow::anyhow!("cluster vanished while ranking: {report:#}"))?;
    Ok(rank)
}

/// Asserts one shape-only cluster is reported honestly: below the
/// act-now fused line, in a non-act-now bucket, and ranked strictly
/// below the genuine clone.
fn assert_shape_only_cluster(cluster: &Value, rank: usize, genuine_rank: usize) {
    let bucket = cluster_bucket(cluster);
    let fused = signal(cluster, "fused");
    let files = cluster_file_set(cluster);
    assert!(
        fused < ACT_NOW_FUSED,
        "shape-only family {files:?} must not reach the act-now fused line: \
         bucket={bucket}, fused={fused}"
    );
    assert!(
        HONEST_SHAPE_ONLY_BUCKETS.contains(&bucket),
        "shape-only family {files:?} must be routed to an honest bucket: \
         bucket={bucket}, fused={fused}"
    );
    assert!(
        genuine_rank < rank,
        "the genuine clone (rank #{genuine_rank}) must outrank the shape-only \
         family {files:?} (rank #{rank})"
    );
}

/// Shared #331/#336 verdict: the genuine clone survives at full
/// confidence and every cluster touching a shape-only fixture file is
/// demoted, honestly bucketed, and outranked (or suppressed outright).
fn assert_shape_only_family_demoted(
    report: &Value,
    genuine_files: &[&str],
    is_noise_file: impl Fn(&str) -> bool,
) -> Result<()> {
    let genuine_rank = assert_genuine_clone_rank(report, genuine_files)?;
    for (rank, cluster) in clusters(report).iter().enumerate() {
        if cluster_file_set(cluster).iter().any(|name| is_noise_file(name)) {
            assert_shape_only_cluster(cluster, rank, genuine_rank);
        }
    }
    Ok(())
}

// [FUSION-STRATEGY-MAX-SUM] / #331: six distinct StatefulWidget
// declarations share only the framework-mandated shape. They must not
// be reported as act-now duplication above a genuine copy-pasted clone.
#[test]
fn issue_331_distinct_widget_declarations_must_not_saturate_fused_confidence() -> Result<()> {
    let mut files: Vec<(String, String)> = WIDGETS
        .iter()
        .enumerate()
        .map(|(index, (name, body))| {
            (format!("widget_{index}.dart"), dart_widget_file(name, body))
        })
        .collect();
    files.extend(genuine_pair(
        "metrics_a.dart",
        "metrics_b.dart",
        DART_GENUINE_CLONE,
    ));

    let report = report_for(&files, 20)?;
    assert_shape_only_family_demoted(
        &report,
        &["metrics_a.dart", "metrics_b.dart"],
        |name| name.starts_with("widget_"),
    )
}

// [FUSION-STRATEGY-MAX-SUM] / #336: four numeric array literals share
// only their length and element kinds — every value differs. They must
// not be reported as act-now duplication above a genuine clone.
#[test]
fn issue_336_distinct_numeric_tables_must_not_saturate_fused_confidence() -> Result<()> {
    let report = report_for(&fsharp_tables_corpus(), 20)?;
    assert_shape_only_family_demoted(&report, &["parse_a.fs", "parse_b.fs"], |name| {
        name.starts_with("tables_")
    })
}
