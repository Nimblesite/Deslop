//! E2E proof for [METRICS-REPO] that the duplication metric is computed by
//! one language-agnostic code path. `compute_repo_metrics` projects clone
//! occurrences onto physical-line sets with no per-language branch, so the
//! same logical input must yield the same percentage in every language.
//!
//! Two byte-identical source files are a fully-duplicated repo. For C#,
//! Rust, Python, Dart, JavaScript, and TypeScript this asserts the metric
//! reports exactly 100% — the same reasonable value across all six — and
//! that the repo `duplicated_loc` equals the lines the visible clusters
//! cover, so a hidden / structural-only match could never inflate it.

mod common;

use serde_json::Value;

use crate::common::*;

/// One language's minimal fully-duplicated corpus: a file extension and a
/// source body written byte-identically into `a.<ext>` and `b.<ext>`.
struct LangCase {
    language: &'static str,
    extension: &'static str,
    source: &'static str,
}

const CASES: &[LangCase] = &[
    LangCase {
        language: "csharp",
        extension: "cs",
        source: "class Calc {\n    int Combine(int a, int b) {\n        int x = a + b;\n        int y = x * a;\n        return y - b + x;\n    }\n}\n",
    },
    LangCase {
        language: "rust",
        extension: "rs",
        source: "fn combine(a: i32, b: i32) -> i32 {\n    let x = a + b;\n    let y = x * a;\n    y - b + x\n}\n",
    },
    LangCase {
        language: "python",
        extension: "py",
        source: "def combine(a, b):\n    x = a + b\n    y = x * a\n    return y - b + x\n",
    },
    LangCase {
        language: "dart",
        extension: "dart",
        source: "int combine(int a, int b) {\n  var x = a + b;\n  var y = x * a;\n  return y - b + x;\n}\n",
    },
    LangCase {
        language: "javascript",
        extension: "js",
        source: "export function combine(a, b) {\n  const x = a + b;\n  const y = x * a;\n  return y - b + x;\n}\n",
    },
    LangCase {
        language: "typescript",
        extension: "ts",
        source: "export function combine(a: number, b: number): number {\n  const x = a + b;\n  const y = x * a;\n  return y - b + x;\n}\n",
    },
];

#[test]
fn duplication_metric_is_language_agnostic() -> Result<()> {
    let mut percentages = Vec::new();
    for case in CASES {
        let tmp = tempfile::tempdir()?;
        let scan_root = tmp.path().join(case.language);
        write_identical_pair(&scan_root, case.extension, case.source)?;
        let report = run_report(&scan_root, 8)?;
        assert_fully_duplicated(&report, case.language);
        percentages.push(
            metric_field(&report, "duplication_percent")
                .as_f64()
                .unwrap_or(-1.0),
        );
    }

    // The same logical input yields the same percentage in every language —
    // there is one calc, not four.
    let baseline = percentages.first().copied().unwrap_or(-1.0);
    for (case, percent) in CASES.iter().zip(&percentages) {
        assert!(
            (percent - baseline).abs() < 0.0001,
            "{}: every language must derive the same percentage from identical \
             input (baseline {baseline}), got {percent}",
            case.language
        );
    }
    Ok(())
}

/// Asserts a two-identical-file repo reports a fully-duplicated metric: every
/// analysed line is duplicated, the percentage is exactly 100, the clone is
/// real (not hidden noise), and the repo `duplicated_loc` equals the lines
/// the visible clusters cover.
fn assert_fully_duplicated(report: &Value, language: &str) {
    let analysed = metric_field(report, "analysed_loc").as_u64().unwrap_or(0);
    let duplicated = metric_field(report, "duplicated_loc").as_u64().unwrap_or(0);
    let percent = metric_field(report, "duplication_percent")
        .as_f64()
        .unwrap_or(-1.0);
    let hidden = field(report, "clusters_hidden")
        .as_u64()
        .unwrap_or(u64::MAX);
    assert!(
        analysed > 0,
        "{language}: fixture must analyse at least one line: {report:#}"
    );
    assert_eq!(
        duplicated, analysed,
        "{language}: every line of an identical pair is duplicated: {report:#}"
    );
    assert!(
        (percent - 100.0).abs() < 0.0001,
        "{language}: identical files must report 100% duplication, got {percent}: {report:#}"
    );
    assert_eq!(
        hidden, 0,
        "{language}: an exact identical clone is real duplication, not hidden noise: {report:#}"
    );
    assert_eq!(
        duplicated,
        visible_duplicated_loc(report),
        "{language}: metric must equal the visible-cluster line union: {report:#}"
    );
}
