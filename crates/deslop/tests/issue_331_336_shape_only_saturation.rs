//! [CLONE-BUCKETS-STRUCTURAL-ONLY] Shape matches are information, not duplication.
//! Genuine copies keep their mass and rank; unrelated tables and widget shapes
//! have zero weight, no diagnostic by default, and appear after clones.

use deslop_core::lang::{dart::DartParser, LanguageParser};
use serde_json::Value;

use crate::common::{
    corpora::*,
    scan_dir::temp_scan_dir,
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

const BODY_COPY_FILES: [&str; 2] = ["body_left.dart", "body_right.dart"];
const BODY_COPY_WIDGET_NAMES: [&str; 2] = ["CopyLeftPanel", "CopyRightPanel"];
const BODY_COPY_MIN_NODES: u32 = 20;
const FIRST_FINDING: usize = 0;
const COPIED_WIDGET_NAME: &str = "CopiedPanel";
const COPIED_WIDGET_BODY: &str = "Text(\"same layout\")";
const NEARLY_IDENTICAL_KIND: &str = "nearly_identical";
const IDENTICAL_KIND: &str = "identical";
const DART_BODY_KIND: &str = "function_body";
const COPY_RANK: u64 = 1;
const COPY_DIAGNOSTIC: &str = "warning";
const COPY_GROUP_COUNT: u64 = 1;
const COPIED_WIDGET_LOC: u64 = 24;

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
    assert!(findings::is_clone_finding(clone), "{clone:#}");
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

/// Check zero weight, default silence, and byte-distinct shape members.
fn assert_shape_only_cluster(scan_root: &std::path::Path, cluster: &Value) -> Result<()> {
    let files = cluster_file_set(cluster);
    assert!(
        !has_verbatim_pair(scan_root, cluster)?,
        "shape-only family {files:?} must be byte-distinct: {cluster:#}"
    );
    assert!(
        !findings::is_clone_finding(cluster),
        "shape family is informational: {cluster:#}"
    );
    assert_eq!(
        field(cluster, "severity").as_str(),
        Some("none"),
        "{cluster:#}"
    );
    assert_no_pair_surface_on_cluster(cluster, "shape-only family");
    Ok(())
}

/// Genuine copies lead every informational shape family in the report.
fn assert_shape_only_family_demoted(
    scan_root: &std::path::Path,
    report: &Value,
    genuine_files: &[&str],
    is_noise_file: impl Fn(&str) -> bool,
) -> Result<()> {
    let genuine_rank = assert_genuine_clone_rank(scan_root, report, genuine_files)?;
    for (position, cluster) in clusters(report).iter().enumerate() {
        if cluster_file_set(cluster)
            .iter()
            .any(|name| is_noise_file(name))
        {
            assert_shape_only_cluster(scan_root, cluster)?;
            assert!(
                position > genuine_rank,
                "information follows the genuine clone: {report:#}"
            );
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

/// [CLONE-NOISE-DART-WIDGET-SCAFFOLD] Copied logic inside widget bodies still surfaces.
#[test]
fn copied_logic_inside_widget_bodies_remains_a_ranked_clone() -> Result<()> {
    let files = widget_body_copies();
    let (_workspace, root, report) = report_for_with_root(&files, BODY_COPY_MIN_NODES)?;
    let clone = expect_cluster_spanning(&report, &BODY_COPY_FILES)?;
    assert_widget_copy(&report, clone, NEARLY_IDENTICAL_KIND);
    assert_copied_body_bytes(&root, clone, DART_GENUINE_CLONE)?;
    Ok(())
}

/// [CLONE-NOISE-DART-WIDGET-SCAFFOLD] Identical layouts are real copies.
#[test]
fn copied_widget_construction_is_an_identical_clone() -> Result<()> {
    let source = dart_widget_file(COPIED_WIDGET_NAME, COPIED_WIDGET_BODY);
    let files: Vec<_> = BODY_COPY_FILES
        .map(|path| (path.to_owned(), source.clone()))
        .into();
    let (_workspace, root, report) = report_for_with_root(&files, BODY_COPY_MIN_NODES)?;
    assert_eq!(
        assert_genuine_clone_rank(&root, &report, &BODY_COPY_FILES)?,
        FIRST_FINDING
    );
    let clone = expect_cluster_spanning(&report, &BODY_COPY_FILES)?;
    assert_widget_copy(&report, clone, IDENTICAL_KIND);
    Ok(())
}

/// [CLONE-NOISE-DART-WIDGET-SCAFFOLD] Renaming a widget does not erase its copied layout.
#[test]
fn renamed_widget_construction_is_a_nearly_identical_clone() -> Result<()> {
    let files = widget_copies_with_body(COPIED_WIDGET_BODY);
    let (_workspace, root, report) = report_for_with_root(&files, BODY_COPY_MIN_NODES)?;
    let clone = expect_cluster_spanning(&report, &BODY_COPY_FILES)?;
    assert_widget_copy(&report, clone, NEARLY_IDENTICAL_KIND);
    let expected = dart_widget_file(COPIED_WIDGET_NAME, COPIED_WIDGET_BODY);
    assert_copied_body_bytes(&root, clone, &expected)?;
    Ok(())
}

/// [CLONE-NOISE-DART-WIDGET-SCAFFOLD] A copied layout cannot turn unrelated widgets into copies.
#[test]
fn copied_widget_construction_stays_separate_from_unrelated_layouts() -> Result<()> {
    let files = mixed_widget_layouts();
    let (_workspace, root, report) = report_for_with_root(&files, BODY_COPY_MIN_NODES)?;
    let clone = expect_cluster_spanning(&report, &BODY_COPY_FILES)?;
    assert_widget_copy(&report, clone, NEARLY_IDENTICAL_KIND);
    let expected = dart_widget_file(COPIED_WIDGET_NAME, COPIED_WIDGET_BODY);
    assert_copied_body_bytes(&root, clone, &expected)?;
    assert_widget_copy_totals(&report)?;
    for candidate in clone_findings(&report) {
        assert_eq!(
            cluster_file_set(&candidate),
            cluster_file_set(clone),
            "unrelated widget bodies are not clones: {candidate:#}"
        );
    }
    Ok(())
}

fn assert_widget_copy_totals(report: &Value) -> Result<()> {
    assert_eq!(
        clone_findings(report).len(),
        usize::try_from(COPY_GROUP_COUNT)?,
        "one copied group, no duplicate views: {report:#}"
    );
    let metrics = field(report, "metrics");
    let expected = [
        ("clusters_total", COPY_GROUP_COUNT),
        ("duplicated_files", u64::try_from(BODY_COPY_FILES.len())?),
        ("duplicated_loc", COPIED_WIDGET_LOC),
    ];
    for (name, value) in expected {
        assert_eq!(field(metrics, name).as_u64(), Some(value), "{name}");
    }
    Ok(())
}

fn mixed_widget_layouts() -> Vec<(String, String)> {
    let mut files = widget_copies_with_body(COPIED_WIDGET_BODY);
    files.extend(
        WIDGETS
            .into_iter()
            .enumerate()
            .map(|(index, (name, body))| {
                (format!("widget_{index}.dart"), dart_widget_file(name, body))
            }),
    );
    files
}

fn assert_widget_copy(report: &Value, clone: &Value, kind: &str) {
    assert_eq!(field(clone, "kind").as_str(), Some(kind), "{clone:#}");
    assert_eq!(clusters(report).first(), Some(clone), "{report:#}");
    assert_eq!(field(clone, "rank").as_u64(), Some(COPY_RANK), "{clone:#}");
    let severity = field(clone, "severity").as_str();
    assert_eq!(severity, Some(COPY_DIAGNOSTIC), "{clone:#}");
    assert_eq!(
        occurrences(clone).len(),
        BODY_COPY_FILES.len(),
        "one body per file: {clone:#}"
    );
    assert_structural_only_contract(clone, "copied widget");
    assert_no_pair_surface_on_cluster(clone, "copied widget");
}

fn assert_copied_body_bytes(root: &std::path::Path, clone: &Value, expected: &str) -> Result<()> {
    let expected_bodies = dart_body_sources(expected)?;
    assert!(
        !expected_bodies.is_empty(),
        "the copy witness has a parsed body"
    );
    for occurrence in occurrences(clone) {
        let source = occurrence_text(root, occurrence)?;
        let bodies = dart_body_sources(&source)?;
        assert!(
            expected_bodies.iter().any(|body| bodies.contains(body)),
            "the reported range contains exact copied body bytes: {occurrence:#}"
        );
    }
    Ok(())
}

fn dart_body_sources(source: &str) -> Result<Vec<Vec<u8>>> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&DartParser::new().grammar())?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("Dart parser returned no tree"))?;
    assert!(!tree.root_node().has_error(), "Dart copy witness parses");
    dart_body_nodes(tree.root_node())
        .into_iter()
        .map(|node| {
            source
                .as_bytes()
                .get(node.byte_range())
                .map(<[u8]>::to_vec)
                .ok_or_else(|| anyhow::anyhow!("body range outside source"))
        })
        .collect()
}

fn dart_body_nodes(root: tree_sitter::Node<'_>) -> Vec<tree_sitter::Node<'_>> {
    let mut bodies = Vec::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if node.kind() == DART_BODY_KIND {
            bodies.push(node);
        }
        pending.extend(node.named_children(&mut node.walk()));
    }
    bodies
}

fn widget_body_copies() -> Vec<(String, String)> {
    let body = format!(
        "(() {{ {DART_GENUINE_CLONE} return Text(weightedTotal([1, 2], 3).toString()); }})()"
    );
    widget_copies_with_body(&body)
}

fn widget_copies_with_body(body: &str) -> Vec<(String, String)> {
    BODY_COPY_FILES
        .into_iter()
        .zip(BODY_COPY_WIDGET_NAMES)
        .map(|(path, name)| (path.to_owned(), dart_widget_file(name, body)))
        .collect()
}

// [CLONE-NOISE-DART-WIDGET-SCAFFOLD] / #331: template-stamped example
// apps share one class name and most content, so content agreement
// cannot demote them — the framework-scaffold filter must. The genuine
// clone keeps surfacing (recall guard).
#[test]
fn issue_331_template_stamped_widget_scaffolds_do_not_surface() -> Result<()> {
    let (_tmp, root) = temp_scan_dir("src")?;
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
