//! [METRICS-REPO-WEIGHTED] / [EXIT-CODES-WEIGHTED] There is one
//! duplication percentage and it is engine-computed.
//!
//! Structural, Jaccard, embedding and content evidence belong to pairs
//! and decide admission. Projecting any of them onto a closure component
//! invents a cluster score, so `bucket_weights`, `category_weights`,
//! `weighted_duplicated_loc`, `weighted_duplication_percent` and an
//! evidence-weighted `[metrics]` table are forbidden wire and
//! configuration. [EXIT-CODES] owns the one duplication-percentage gate;
//! there is no weighted companion to it.
//!
//! These are black-box CLI runs asserted against the rendered reports:
//! every surface must carry the same engine figure and no other.

use std::{fs, path::Path};

use anyhow::Result;
use serde_json::Value;

use crate::common::*;

/// A repository whose every line is duplicated, so a weighted figure —
/// were one computed — would be free to disagree with the honest one.
const DUPLICATED_SOURCE: &str = "export function reconcile(entries: number[]): number {\n\
    \x20   let balance = 0;\n\
    \x20   for (const entry of entries) {\n\
    \x20       balance += entry;\n\
    \x20   }\n\
    \x20   return balance;\n\
}\n";
/// The language the pair is written in.
const DUPLICATED_EXTENSION: &str = "ts";
/// The node floor the pair is scanned at: below the default, so the
/// seven-line function is in reach and the corpus is wholly duplicated.
const DUPLICATED_MIN_NODES: &str = "8";
/// Every metric key the wire may carry under `metrics`, and nothing else
/// may appear beside them.
const ALLOWED_METRIC_KEYS: [&str; 8] = [
    "analysed_loc",
    "clusters_total",
    "duplicated_files",
    "duplicated_loc",
    "duplication_percent",
    "folders",
    "per_file",
    "threshold",
];
/// The percentage two byte-identical files must produce.
const WHOLLY_DUPLICATED_PERCENT: f64 = 100.0;
/// How close a floating-point percentage must sit to it.
const PERCENT_TOLERANCE: f64 = 0.0001;
/// The three fields a threshold verdict carries ([EXIT-CODES]).
const THRESHOLD_FIELDS: [&str; 3] = ["breached", "percent", "source"];
/// The substring that betrays an evidence-weighted field anywhere in the
/// JSON wire, whatever the surface calls it.
const FORBIDDEN_KEY_MARKER: &str = "weight";
/// The word a rendered surface would have to use to offer a reader a
/// weighted figure. The bare stem would match the CSS `font-weight` the
/// HTML report styles itself with.
const FORBIDDEN_RENDER_MARKER: &str = "weighted";
/// Flags the spec says do not exist. Passing one must be a usage error,
/// never a silently ignored argument that changes no verdict.
const FORBIDDEN_FLAGS: [&str; 2] = ["--fail-over-weighted", "--max-weighted-duplication-percent"];
/// The exit code `clap` returns for an unknown argument.
const USAGE_EXIT_CODE: i32 = 2;

/// Every key in the report, at every depth, with its path — so a
/// forbidden field cannot hide inside a nested object.
fn key_paths(value: &Value, prefix: &str, found: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let path = format!("{prefix}.{key}");
                found.push(path.clone());
                key_paths(child, &path, found);
            }
        }
        Value::Array(items) => {
            for item in items {
                key_paths(item, prefix, found);
            }
        }
        _ => {}
    }
}

/// Scans a fully duplicated two-file repository and returns the JSON
/// report beside the text and HTML the same run rendered.
fn render_every_surface(tmp: &Path) -> Result<(Value, String, String)> {
    let scan_root = tmp.join("repo");
    write_identical_pair(&scan_root, DUPLICATED_EXTENSION, DUPLICATED_SOURCE)?;
    let prefix = tmp.join("report");
    let mut command = deslop_cmd(&scan_root, &prefix)?;
    let _assertion = command
        .args(["--min-nodes", DUPLICATED_MIN_NODES])
        .assert()
        .success();
    let json: Value = serde_json::from_str(&fs::read_to_string(tmp.join("report.json"))?)?;
    let text = fs::read_to_string(tmp.join("report.txt"))?;
    let html = fs::read_to_string(tmp.join("report.html"))?;
    Ok((json, text, html))
}

#[test]
fn no_surface_carries_an_evidence_weighted_duplication_figure() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let (report, text, html) = render_every_surface(tmp.path())?;

    // [METRICS-REPO-WEIGHTED] Nothing anywhere in the report is weighted.
    let mut paths = Vec::new();
    key_paths(&report, "", &mut paths);
    let weighted: Vec<&String> = paths
        .iter()
        .filter(|path| path.to_ascii_lowercase().contains(FORBIDDEN_KEY_MARKER))
        .collect();
    assert!(
        weighted.is_empty(),
        "no report field may be evidence-weighted, found {weighted:?}: {report:#}"
    );
    assert!(
        !paths.is_empty(),
        "the key scan must actually reach the report: {report:#}"
    );

    // The metrics object carries the engine's figures and no companion.
    let metrics = field(&report, "metrics");
    let mut present: Vec<&str> = metrics
        .as_object()
        .map(|map| map.keys().map(String::as_str).collect())
        .unwrap_or_default();
    present.sort_unstable();
    assert_eq!(
        present, ALLOWED_METRIC_KEYS,
        "the metrics wire is closed: {metrics:#}"
    );

    // [EXIT-CODES] One threshold verdict, three fields, engine-stamped.
    let threshold = field(metrics, "threshold");
    let mut verdict_fields: Vec<&str> = threshold
        .as_object()
        .map(|map| map.keys().map(String::as_str).collect())
        .unwrap_or_default();
    verdict_fields.sort_unstable();
    assert_eq!(
        verdict_fields, THRESHOLD_FIELDS,
        "a threshold verdict is percent, breached and source: {threshold:#}"
    );
    assert_eq!(
        field(threshold, "source"),
        "none",
        "no flag and no config means no threshold: {threshold:#}"
    );
    assert_eq!(
        field(threshold, "breached"),
        false,
        "an absent threshold cannot be breached: {threshold:#}"
    );

    assert_rendered_surfaces_agree(&report, &text, &html)
}

/// [METRICS-REPO-WEIGHTED] The text and HTML renderers state the figure
/// the engine computed and never a second one of their own.
fn assert_rendered_surfaces_agree(report: &Value, text: &str, html: &str) -> Result<()> {
    let metrics = field(report, "metrics");
    let percent = field(metrics, "duplication_percent")
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("duplication_percent missing: {metrics:#}"))?;
    assert!(
        (percent - WHOLLY_DUPLICATED_PERCENT).abs() < PERCENT_TOLERANCE,
        "two byte-identical files are wholly duplicated, got {percent}: {metrics:#}"
    );
    assert_eq!(
        field(metrics, "duplicated_loc"),
        field(metrics, "analysed_loc"),
        "every analysed line is duplicated in this corpus: {metrics:#}"
    );
    // The one line the text renderer prints, in the shape the spec
    // states, with every number taken from the engine's own metrics.
    let headline = format!(
        "repo: {percent:.1}% duplicated ({} / {} LOC, {} clusters across {} files)",
        field(metrics, "duplicated_loc"),
        field(metrics, "analysed_loc"),
        field(metrics, "clusters_total"),
        field(metrics, "duplicated_files"),
    );
    assert!(
        text.contains(&headline),
        "the text report must state `{headline}`: {text}"
    );
    assert_eq!(
        text.matches("% duplicated").count(),
        1,
        "one duplication percentage, not a companion beside it: {text}"
    );
    assert!(
        html.contains(&format!("{percent:.1}% duplicated")),
        "the HTML report must state the same figure"
    );
    assert_eq!(
        html.matches("% duplicated").count(),
        1,
        "the HTML surface carries the one figure too"
    );
    for surface in [text, html] {
        assert!(
            !surface.contains(FORBIDDEN_RENDER_MARKER),
            "no rendered surface may show a weighted companion figure: {surface}"
        );
    }
    Ok(())
}

#[test]
fn a_weighted_threshold_flag_is_a_usage_error() -> Result<()> {
    // [EXIT-CODES-WEIGHTED] A flag that does not exist must fail loudly.
    // Accepting and ignoring one would let a caller believe a weighted
    // gate ran when the only gate is the engine's own percentage.
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("repo");
    write_identical_pair(&scan_root, DUPLICATED_EXTENSION, DUPLICATED_SOURCE)?;
    for flag in FORBIDDEN_FLAGS {
        let mut command = deslop_cmd(&scan_root, &tmp.path().join("report"))?;
        let _assertion = command.arg(flag).arg("10.0").assert().code(USAGE_EXIT_CODE);
    }
    Ok(())
}
