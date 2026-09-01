//! End-to-end coverage for occurrence location rendering.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::common::{
    clone_corpus::{self, MIN_NODES},
    *,
};

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

/// The nested clone carriers, written the way every consumer names them.
const NESTED_CLONES: [&str; 2] = ["core/handlers/alpha.rs", "core/handlers/beta.rs"];

/// The separator a report path must never carry, on any platform.
const NATIVE_ONLY_SEPARATOR: char = '\\';

/// The separator every report path must carry, on every platform.
const WIRE_SEPARATOR: char = '/';

/// [OUTPUT-DIR] A report path is a wire value, read by the corpus
/// manifests, the VSIX, the MCP `path_contains` filter and every AI
/// consumer, all of which name a file with `/`. Serialising the platform
/// separator makes one tree render two different reports and makes every
/// consumer's comparison platform-conditional: `metrics.folders` already
/// joins with `/` everywhere, so on Windows one report shipped both
/// conventions (gh #439).
///
/// This case can only go red on a platform whose separator is not `/`.
/// The cross-platform half of the contract is pinned by the unit cases in
/// `deslop_core::paths::tests`, which supply the separator explicitly.
#[test]
fn every_reported_path_is_joined_with_the_wire_separator() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("tree");
    seed_nested_clones(&scan_root)?;
    let report = run_report(&scan_root, MIN_NODES)?;

    let reported = clone_corpus::all_report_paths(&report);
    assert!(
        !reported.is_empty(),
        "the scan must report the nested clone carriers, or this asserts nothing: {report}"
    );
    for path in &reported {
        assert!(
            !path.contains(NATIVE_ONLY_SEPARATOR),
            "reported path `{path}` carries the native separator; a report read on another \
             platform, or against a manifest, would miss this file on the separator alone"
        );
        assert!(
            path.contains(WIRE_SEPARATOR),
            "reported path `{path}` names a file nested two directories deep, so it must \
             carry `{WIRE_SEPARATOR}` separators"
        );
    }
    for carrier in NESTED_CLONES {
        assert!(
            reported.iter().any(|path| path == carrier),
            "the report must name `{carrier}` exactly as a manifest and a VSIX link do; \
             it reported {reported:?}"
        );
    }
    Ok(())
}

/// Writes both carriers of one duplicate function two directories deep.
fn seed_nested_clones(scan_root: &Path) -> Result<()> {
    for carrier in NESTED_CLONES {
        let file = scan_root.join(carrier);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file, clone_corpus::dup_source(carrier))?;
    }
    Ok(())
}
