//! End-to-end coverage for occurrence location rendering.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use assert_cmd::Command;

mod common;
use crate::common::*;

#[test]
fn rendered_occurrence_locations_are_line_column_not_byte_ranges() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = Command::cargo_bin("deslop")?
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(&output)
        .assert()
        .success();
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
