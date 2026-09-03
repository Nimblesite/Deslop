//! End-to-end coverage for occurrence location rendering.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde_json::Value;

use crate::common::{scan_dir::temp_scan_dir, *};

#[test]
fn rendered_occurrence_locations_are_line_column_not_byte_ranges() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let mut cmd = deslop_cmd(&fixture("csharp-small"), &output)?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    let text = fs::read_to_string(with_ext(&output, "txt"))?;
    let html = fs::read_to_string(with_ext(&output, "html"))?;
    assert_human_locations(&text, "text report");
    assert_human_locations(visible_html_body(&html), "HTML report");
    Ok(())
}

fn assert_human_locations(rendered: &str, label: &str) {
    assert!(
        rendered.contains(".cs:"),
        "{label} must include C# occurrence locations: {rendered}"
    );
    assert!(
        !has_compact_range(rendered),
        "{label} must not render occurrence locations as path:start-end byte ranges: {rendered}"
    );
    assert!(
        !rendered.contains("bytes "),
        "{label} must not expose byte markers in occurrence display text: {rendered}"
    );
}

fn has_compact_range(rendered: &str) -> bool {
    rendered.split_whitespace().any(is_compact_range_token)
}

fn visible_html_body(html: &str) -> &str {
    html.split_once("<details class=\"run-details\"")
        .map_or(html, |(body, _details)| body)
}

fn is_compact_range_token(token: &str) -> bool {
    let Some((_, tail)) = token.split_once(".cs:") else {
        return false;
    };
    tail.chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-')
        .any(|c| c == '-')
}

fn with_ext(base: &Path, ext: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    let _changed = path.set_extension(ext);
    path
}

// ---------------------------------------------------------------------------
// [OUTPUT-SCHEMA-PATH-SEPARATOR] Rendered path spelling.
// ---------------------------------------------------------------------------

/// The one separator a rendered report puts between path segments, on
/// every platform. A consumer that reads a report produced on Windows
/// and one produced on Linux must get the same string for the same
/// file, or every path comparison it makes across the two is wrong.
const REPORT_SEPARATOR: char = '/';

/// The separator the host's own [`std::path`] joins with. It is `\` on
/// Windows, and finding it in a rendered report is the defect this
/// section exists to catch.
const HOST_SEPARATOR: char = std::path::MAIN_SEPARATOR;

/// Directory the clone pair is staged into: two levels deep, so a fix
/// that only respells the first separator still fails here.
const NESTED_DIR: &str = "src/billing";

/// The same directory spelled the way a Windows host would join it.
/// No rendered surface may contain this.
const NESTED_DIR_HOST_SPELLING: &str = "src\\billing";

/// The character no rendered path may contain.
const BACKSLASH: char = '\\';

/// How one backslash appears once JSON has escaped it, so the raw
/// document can be swept without parsing it back.
const ESCAPED_BACKSLASH: &str = "\\\\";

/// The two staged clones, spelled the way every surface must render
/// them.
const ALPHA_PATH: &str = "src/billing/Alpha.cs";
const BETA_PATH: &str = "src/billing/Beta.cs";

/// Both folder rows the nested corpus must roll up into.
const FOLDER_ROWS: [&str; 2] = ["src", "src/billing"];

/// Subtree floor that publishes the staged C# pair as one cluster.
const NESTED_MIN_NODES: &str = "8";

/// The object key every rendered workspace-relative path sits under.
const PATH_KEY: &str = "path";

/// Stages `csharp-small` two directories deep and renders all three
/// formats over it.
fn nested_corpus_reports() -> Result<(tempfile::TempDir, PathBuf)> {
    let (tmp, root) = temp_scan_dir("repo")?;
    seed(&fixture("csharp-small"), &root.join(NESTED_DIR))?;
    let output = tmp.path().join("report");
    let mut cmd = deslop_cmd(&root, &output)?;
    let _assertion = cmd
        .args(["--min-nodes", NESTED_MIN_NODES, "--embeddings", "off"])
        .assert()
        .success();
    Ok((tmp, output))
}

/// Every `path` string in `value`, paired with the JSON pointer it was
/// found at so a failure names the offending field.
fn rendered_paths(value: &Value) -> Vec<(String, String)> {
    let mut found = Vec::new();
    collect_paths(value, "", &mut found);
    found
}

fn collect_paths(value: &Value, at: &str, found: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let here = format!("{at}/{key}");
                if key == PATH_KEY {
                    if let Some(text) = child.as_str() {
                        found.push((here.clone(), text.to_owned()));
                    }
                }
                collect_paths(child, &here, found);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_paths(child, &format!("{at}/{index}"), found);
            }
        }
        _ => {}
    }
}

/// The `path` values of one named metrics array, in wire order.
fn metric_paths(report: &Value, array: &str) -> Vec<String> {
    field(field(report, "metrics"), array)
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get(PATH_KEY).and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

// [OUTPUT-SCHEMA-PATH-SEPARATOR] Every workspace-relative path the JSON
// report publishes — occurrence, per-file metric, folder rollup — is
// spelled with `/`, whatever the host joins its own paths with. The
// sweep is exhaustive rather than field-by-field: a new path-bearing
// field added later is covered the day it ships, with no edit here.
#[test]
fn every_rendered_json_path_is_spelled_with_forward_slashes() -> Result<()> {
    let (_tmp, output) = nested_corpus_reports()?;
    let report = load_json(&with_ext(&output, "json"))?;

    let rendered = rendered_paths(&report);
    assert!(
        rendered.len() >= 5,
        "a two-file nested corpus must publish at least two occurrence, \
         two per-file and two folder paths, found {rendered:?}: {report:#}",
    );
    for (pointer, path) in &rendered {
        assert!(
            !path.contains(HOST_SEPARATOR) || HOST_SEPARATOR == REPORT_SEPARATOR,
            "{pointer} renders the host separator {HOST_SEPARATOR:?} in {path:?}: \
             a report is read on platforms other than the one that wrote it",
        );
        assert!(
            !path.contains(BACKSLASH),
            "{pointer} renders a backslash in {path:?}: every rendered path \
             segment is joined with {REPORT_SEPARATOR:?}",
        );
        assert_eq!(
            path,
            &path
                .split(REPORT_SEPARATOR)
                .collect::<Vec<_>>()
                .join(&REPORT_SEPARATOR.to_string()),
            "{pointer} must survive a split/join on {REPORT_SEPARATOR:?} unchanged",
        );
    }

    let occurrence_paths: Vec<String> = clusters(&report)
        .iter()
        .flat_map(|cluster| occurrences(cluster).iter())
        .filter_map(|occurrence| occurrence.get(PATH_KEY).and_then(Value::as_str))
        .map(str::to_owned)
        .collect();
    assert_eq!(
        occurrence_paths,
        vec![ALPHA_PATH.to_owned(), BETA_PATH.to_owned()],
        "the staged clone pair must be named by its nested path: {report:#}",
    );

    let mut per_file = metric_paths(&report, "per_file");
    per_file.sort();
    assert_eq!(
        per_file,
        vec![ALPHA_PATH.to_owned(), BETA_PATH.to_owned()],
        "per-file metric rows carry the same spelling as occurrences: {report:#}",
    );

    let mut folders = metric_paths(&report, "folders");
    folders.sort();
    assert_eq!(
        folders,
        FOLDER_ROWS.map(str::to_owned).to_vec(),
        "folder rollups group the segments a client splits a file row into: {report:#}",
    );
    Ok(())
}

// [OUTPUT-SCHEMA-PATH-SEPARATOR] The derived text and HTML renderings
// carry the JSON spelling through unchanged — a human reading either
// one, and a tool matching a location out of either one, sees the same
// path the canonical report published.
#[test]
fn derived_text_and_html_carry_the_json_path_spelling() -> Result<()> {
    let (_tmp, output) = nested_corpus_reports()?;
    let json = fs::read_to_string(with_ext(&output, "json"))?;
    let text = fs::read_to_string(with_ext(&output, "txt"))?;
    let html = fs::read_to_string(with_ext(&output, "html"))?;

    for (label, rendered) in [("JSON", &json), ("text", &text), ("HTML", &html)] {
        for expected in [ALPHA_PATH, BETA_PATH] {
            assert!(
                rendered.contains(expected),
                "{label} report must name {expected}: {rendered}",
            );
        }
        assert!(
            !rendered.contains(NESTED_DIR_HOST_SPELLING),
            "{label} report must not spell a nested path with a backslash: {rendered}",
        );
    }
    assert!(
        !json.contains(ESCAPED_BACKSLASH),
        "no JSON string value may carry an escaped backslash: {json}",
    );
    Ok(())
}
