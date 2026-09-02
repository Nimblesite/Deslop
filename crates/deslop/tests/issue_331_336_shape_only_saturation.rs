//! End-to-end regression coverage for #331 / #336 — shape-only
//! saturation ([FUSED-STRATEGY-BOUNDED-MAX], [RANK-STRUCTURAL-ONLY]).
//!
//! Both showstoppers share one mechanism: identifier/literal
//! normalisation makes `token_jaccard` a restatement of `structural`,
//! and sum-then-clamp fusion pins `fused` at 1.0 for every structural
//! match. Distinct-content, same-shape families (mandatory Flutter
//! widget declarations, unrelated numeric array literals) are then
//! reported as act-now duplication and outrank genuine clones.
//!
//! [RANK-STRUCTURAL-ONLY] retired the `structural_only` / `data_clone`
//! demotion modes — weight means mass and nothing else — so the
//! post-hoc "shape-only family outranks genuine clone" verdict is no
//! longer enforceable on the wire. What this suite still pins, at full
//! strength:
//! - a genuine copy-pasted clone is still found and byte-proven
//!   (recall guard);
//! - every cluster carries the mass-only surface — no `fused`, no
//!   bucket, no verdict ([FUSED-SCOPE], [PIPELINE-CLUSTER-CLOSURE]);
//! - a shape-only family is byte-distinct (never misread as a copy-paste
//!   of itself) and its mass is the wire formula.
//!
//! The ranking inversion #331 filed is tracked separately: it returned
//! when the demotion was retired, and if it is a real accuracy defect
//! the spec decision ([RANK-STRUCTURAL-ONLY]) — not this suite — is
//! what must move.

use serde_json::Value;

use crate::common::{
    corpora::*,
    signals::{
        assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
    },
    *,
};

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
/// reported and byte-proven — the recall half that keeps a precision
/// fix from silencing real clones — and returns its zero-based rank.
fn assert_genuine_clone_rank(
    scan_root: &std::path::Path,
    report: &Value,
    files: &[&str],
) -> Result<usize> {
    let clone = expect_cluster_spanning(report, files)?;
    assert!(
        has_verbatim_pair(scan_root, clone)?,
        "byte-identical clone across {files:?} must be byte-proven from the \
         source: {report:#}"
    );
    assert_structural_only_contract(clone, "#331/#336 genuine clone");
    assert_no_pair_surface_on_cluster(clone, "#331/#336 genuine clone");
    let rank = clusters(report)
        .iter()
        .position(|cluster| std::ptr::eq(cluster, clone))
        .ok_or_else(|| anyhow::anyhow!("cluster vanished while ranking: {report:#}"))?;
    Ok(rank)
}

/// Asserts one shape-only cluster is honest on the mass-only wire: no
/// pair-only surface, byte-distinct occurrences (a shape family, never
/// a copy-paste of itself), and the wire mass formula.
fn assert_shape_only_cluster(scan_root: &std::path::Path, cluster: &Value) -> Result<()> {
    let files = cluster_file_set(cluster);
    assert!(
        !has_verbatim_pair(scan_root, cluster)?,
        "shape-only family {files:?} must be byte-distinct — the same-shape \
         members differ in content, so a verbatim (byte-identical) reading \
         would be a fabrication: {cluster:#}"
    );
    assert_structural_only_contract(cluster, "#331/#336 shape-only family");
    assert_no_pair_surface_on_cluster(cluster, "#331/#336 shape-only family");
    Ok(())
}

/// Shared #331/#336 recall guard: the genuine clone survives, is
/// byte-proven, and every cluster touching a shape-only fixture file is
/// mass-honest and byte-distinct.
fn assert_shape_only_family_demoted(
    scan_root: &std::path::Path,
    report: &Value,
    genuine_files: &[&str],
    is_noise_file: impl Fn(&str) -> bool,
) -> Result<()> {
    let _genuine_rank = assert_genuine_clone_rank(scan_root, report, genuine_files)?;
    for cluster in clusters(report) {
        if cluster_file_set(cluster)
            .iter()
            .any(|name| is_noise_file(name))
        {
            assert_shape_only_cluster(scan_root, cluster)?;
        }
    }
    Ok(())
}

// [FUSED-STRATEGY-BOUNDED-MAX] / #331: six distinct StatefulWidget
// declarations share only the framework-mandated shape. They must not
// be reported as act-now duplication above a genuine copy-pasted clone.
#[test]
fn issue_331_distinct_widget_declarations_must_not_saturate_fused_confidence() -> Result<()> {
    let mut files: Vec<(String, String)> = WIDGETS
        .iter()
        .enumerate()
        .map(|(index, (name, body))| (format!("widget_{index}.dart"), dart_widget_file(name, body)))
        .collect();
    files.extend(genuine_pair(
        "metrics_a.dart",
        "metrics_b.dart",
        DART_GENUINE_CLONE,
    ));

    let (_workspace, root, report) = report_for_with_root(&files, 20)?;
    assert_shape_only_family_demoted(
        &root,
        &report,
        &["metrics_a.dart", "metrics_b.dart"],
        |name| name.starts_with("widget_"),
    )
}

// [CLONE-NOISE-DART-WIDGET-SCAFFOLD] / #331: template-stamped example
// apps share one class name and most content, so content agreement
// cannot demote them — the framework-scaffold filter must. The genuine
// clone keeps surfacing (recall guard).
#[test]
fn issue_331_template_stamped_widget_scaffolds_do_not_surface() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let root = tmp.path().join("src");
    std::fs::create_dir_all(&root)?;
    let bodies = [
        "Text(\"alpha\")",
        "Column(children: [Text(\"beta\")])",
        "Container(width: 4, color: Colors.red)",
        "ListView(shrinkWrap: true)",
    ];
    for (index, body) in bodies.iter().enumerate() {
        let source = format!(
            "class ExampleApp extends StatelessWidget {{\n\
             \x20 const ExampleApp({{super.key}});\n\
             \x20 @override\n\
             \x20 Widget build(BuildContext context) {{\n\
             \x20   return MaterialApp(home: {body});\n\
             \x20 }}\n\
             }}\n"
        );
        std::fs::write(root.join(format!("example_{index}.dart")), source)?;
    }
    for (name, source) in genuine_pair("metrics_a.dart", "metrics_b.dart", DART_GENUINE_CLONE) {
        std::fs::write(root.join(name), source)?;
    }
    let report = run_report(&root, 20)?;
    let scaffolds = summaries_where(&report, &root, |text| {
        text.contains("extends StatelessWidget")
    })?;
    assert_eq!(
        scaffolds,
        Vec::<String>::new(),
        "framework-mandated widget scaffolds must not surface as duplication: {report:#}"
    );
    let _rank = assert_genuine_clone_rank(&root, &report, &["metrics_a.dart", "metrics_b.dart"])?;
    Ok(())
}

// [FUSED-STRATEGY-BOUNDED-MAX] / #336: four numeric array literals share
// only their length and element kinds — every value differs. They must
// not be reported as act-now duplication above a genuine clone.
#[test]
fn issue_336_distinct_numeric_tables_must_not_saturate_fused_confidence() -> Result<()> {
    let (_workspace, root, report) = report_for_with_root(&fsharp_tables_corpus(), 20)?;
    assert_shape_only_family_demoted(&root, &report, &["parse_a.fs", "parse_b.fs"], |name| {
        name.starts_with("tables_")
    })
}
