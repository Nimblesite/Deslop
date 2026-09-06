//! The per-cluster `language` label contract over the `duplicates`
//! wire, shared by every language that regressed it.
//!
//! The MCP page summary once derived each cluster's language from a
//! hand-maintained extension → id map inside `deslop-mcp` — a drifted
//! copy of the renderer's mapping. Whatever it omitted surfaced as
//! `language: "unknown"`, breaking both the label and the `duplicates`
//! language filter: `.dart` (#164) and later `.fs` (#270), on a language
//! the analyzer was actually running.
//!
//! The recurrence is the point. The assertion is identical for every
//! language — only the fixture, the extension and the expected id
//! differ — so it lives here once, and adding the next language is a
//! one-line call rather than another copy of the map's blind spot.

use anyhow::{ensure, Result};
use serde_json::Value;

use super::{
    copied_fixture_named, initialized_mcp, request_duplicates_summary,
    spawn_lsp_and_wait_for_socket, structured_content,
};

/// Clusters requested per page: enough to hold every cluster the fixtures surface.
const PAGE_LIMIT: u64 = 50;

/// Drives the real `deslop-lsp` + `deslop-mcp` binaries against
/// `fixture`, and asserts every cluster whose representative occurrence
/// carries `extension` reports `expected_language`.
///
/// Fails when the fixture surfaces no such cluster: a page with nothing
/// to classify would satisfy a per-cluster assertion vacuously, which is
/// exactly the state the label bug produced.
pub fn assert_language_label_over_mcp(
    fixture: &str,
    extension: &str,
    expected_language: &str,
    issue: &str,
) -> Result<()> {
    let workspace = copied_fixture_named(fixture)?;
    let _lsp_guard = spawn_lsp_and_wait_for_socket(workspace.path())?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let page = structured_content(
        &request_duplicates_summary(&mut mcp, PAGE_LIMIT)?,
        "duplicates",
    )?;
    let languages = cluster_languages_for_extension(&page, extension);

    ensure!(
        !languages.is_empty(),
        "{fixture} must surface at least one hand-written .{extension} \
         cluster so the language label is exercised: {page}"
    );
    for language in &languages {
        ensure!(
            language == expected_language,
            "{issue}: a cluster whose representative occurrence is a \
             `.{extension}` file must report language={expected_language:?}, \
             not {language:?}; full page: {page}"
        );
    }
    Ok(())
}

/// The `language` label of every cluster whose representative
/// occurrence has the given file extension.
fn cluster_languages_for_extension(page: &Value, extension: &str) -> Vec<String> {
    page.get("clusters")
        .and_then(Value::as_array)
        .map(|clusters| {
            clusters
                .iter()
                .filter(|cluster| first_occurrence_has_extension(cluster, extension))
                .filter_map(|cluster| cluster.get("language").and_then(Value::as_str))
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// True when a cluster's `first_occurrence.path` carries `extension`.
fn first_occurrence_has_extension(cluster: &Value, extension: &str) -> bool {
    cluster
        .pointer("/first_occurrence/path")
        .and_then(Value::as_str)
        .and_then(|path| std::path::Path::new(path).extension()?.to_str())
        .is_some_and(|found| found.eq_ignore_ascii_case(extension))
}
