//! `textDocument/hover` provider ([LSP-HOVER]).
//!
//! Returns a markdown card for the cluster containing the cursor. The
//! card lays out the four signals, interpretation line, and occurrence
//! list — enough context that the reader can decide whether to
//! investigate without leaving the file.

use std::{
    collections::HashMap,
    fmt::Write as _,
    path::{Path, PathBuf},
};

use deslop_core::{
    report::{occurrence_count, ReportCluster, ReportOccurrence},
    report_location::format_occurrence,
};
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

/// Builds the hover response for `cluster`.
#[must_use]
pub fn build_for_cluster(cluster: &ReportCluster) -> Hover {
    hover_from_markdown(markdown_for(cluster))
}

/// Builds the hover response for `cluster`, resolving relative
/// occurrence paths against `workspace_root`.
#[must_use]
pub fn build_for_cluster_with_root(cluster: &ReportCluster, workspace_root: &Path) -> Hover {
    hover_from_markdown(markdown_for_with_root(cluster, Some(workspace_root)))
}

/// Wraps rendered markdown in an LSP hover response.
fn hover_from_markdown(value: String) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: None,
    }
}

/// Renders the hover markdown body.
#[must_use]
pub fn markdown_for(cluster: &ReportCluster) -> String {
    markdown_for_with_root(cluster, None)
}

/// Renders the hover markdown body with optional path resolution.
fn markdown_for_with_root(cluster: &ReportCluster, workspace_root: Option<&Path>) -> String {
    let header = format!(
        "### Cluster {id}\n\n{interpretation}\n\n",
        id = cluster.id,
        interpretation = cluster.interpretation
    );
    let table = signals_table(cluster);
    let occurrences = occurrences_block(cluster, workspace_root);
    format!("{header}{table}\n{occurrences}")
}

/// Builds the signal table (markdown).
fn signals_table(cluster: &ReportCluster) -> String {
    format!(
        "| Signal | Value |\n|---|---|\n| structural | {structural:.2} |\n| token_jaccard | {jaccard:.2} |\n| embedding_cos | {embedding:.2} |\n| fused | {fused:.2} |\n",
        structural = cluster.signals.structural,
        jaccard = cluster.signals.token_jaccard,
        embedding = cluster.signals.embedding_cos,
        fused = cluster.signals.fused,
    )
}

/// Builds the occurrence bullet list.
fn occurrences_block(cluster: &ReportCluster, workspace_root: Option<&Path>) -> String {
    let mut cache: HashMap<PathBuf, Option<Vec<u8>>> = HashMap::new();
    let header = format!(
        "**Occurrences ({count})**:\n",
        count = occurrence_count(cluster)
    );
    let body = cluster
        .occurrences
        .iter()
        .fold(String::new(), |mut acc, occurrence| {
            let location = occurrence_display_label(occurrence, workspace_root, &mut cache);
            let _ = writeln!(acc, "- {location}");
            acc
        });
    format!("{header}{body}")
}

/// Formats one occurrence without exposing byte offsets.
fn occurrence_display_label(
    occurrence: &ReportOccurrence,
    workspace_root: Option<&Path>,
    cache: &mut HashMap<PathBuf, Option<Vec<u8>>>,
) -> String {
    let absolute = resolve_occurrence_path(&occurrence.path, workspace_root);
    let source = cached_source(&absolute, cache);
    format_occurrence(&occurrence.path, occurrence.start_byte, source)
}

/// Returns an absolute path for a report occurrence.
fn resolve_occurrence_path(path: &Path, workspace_root: Option<&Path>) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.map_or_else(|| path.to_path_buf(), |root| root.join(path))
    }
}

/// Reads source text once per occurrence path.
fn cached_source<'a>(
    path: &Path,
    cache: &'a mut HashMap<PathBuf, Option<Vec<u8>>>,
) -> Option<&'a [u8]> {
    let entry = cache
        .entry(path.to_path_buf())
        .or_insert_with(|| std::fs::read(path).ok());
    entry.as_deref()
}

#[cfg(test)]
#[allow(clippy::missing_docs_in_private_items)]
mod tests {
    use super::*;
    use anyhow::{anyhow, Result};
    use deslop_core::report::{ReportOccurrence, ReportSignals};
    use std::path::PathBuf;

    fn make_cluster() -> ReportCluster {
        ReportCluster {
            id: "abc123".into(),
            weight: 42.5,
            size: 2,
            canonical_node_count: 12,
            signals: ReportSignals {
                structural: 1.0,
                token_jaccard: 0.95,
                embedding_cos: 0.25,
                fused: 2.2,
            },
            bucket: "identical".into(),
            occurrences_total: 2,
            occurrences_truncated: false,
            occurrences: vec![
                ReportOccurrence {
                    path: PathBuf::from("Alpha.cs"),
                    start_byte: 10,
                    end_byte: 40,
                    hidden: false,
                },
                ReportOccurrence {
                    path: PathBuf::from("Beta.cs"),
                    start_byte: 5,
                    end_byte: 35,
                    hidden: false,
                },
            ],
            summary: "test summary".into(),
            interpretation: "Identical code. Safe to extract.".into(),
        }
    }

    #[test]
    fn markdown_for_cluster_covers_header_signals_and_occurrences() {
        let cluster = make_cluster();
        let body = markdown_for(&cluster);
        // Header carries the cluster id and interpretation.
        assert!(body.contains("### Cluster abc123"), "header: {body}");
        assert!(
            body.contains("Identical code. Safe to extract."),
            "interpretation: {body}"
        );
        // Signals table: each row present with 2dp formatting.
        assert!(body.contains("| structural | 1.00 |"), "structural: {body}");
        assert!(body.contains("| token_jaccard | 0.95 |"), "jaccard: {body}");
        assert!(
            body.contains("| embedding_cos | 0.25 |"),
            "embedding: {body}"
        );
        assert!(body.contains("| fused | 2.20 |"), "fused: {body}");
        // Occurrence bullet list carries both occurrences without byte ranges.
        assert!(body.contains("**Occurrences (2)**"), "occ header: {body}");
        assert!(
            body.contains("- Alpha.cs:line unavailable"),
            "alpha occ: {body}"
        );
        assert!(
            body.contains("- Beta.cs:line unavailable"),
            "beta occ: {body}"
        );
        assert!(!body.contains(":10-40"), "no byte labels: {body}");
    }

    #[test]
    fn markdown_for_cluster_uses_total_count_and_human_locations() -> Result<()> {
        // GH #26/#27: hover copy must not drift from the authoritative
        // cluster occurrence count, and it must not render raw byte
        // markers as the user-facing location.
        let mut cluster = make_cluster();
        let dir = tempfile::tempdir()?;
        let source_path = dir.path().join("Alpha.cs");
        let source = "first\n  duplicated();\n";
        std::fs::write(&source_path, source)?;
        let start = source.find("duplicated").ok_or_else(|| anyhow!("token"))?;
        let occurrence = cluster
            .occurrences
            .get_mut(0)
            .ok_or_else(|| anyhow!("first occurrence"))?;
        occurrence.path = source_path;
        occurrence.start_byte = start;
        occurrence.end_byte = start.saturating_add("duplicated".len());
        cluster.occurrences_total = 35;
        let body = markdown_for(&cluster);
        assert!(
            body.contains("**Occurrences (35)**"),
            "hover must report occurrences_total when present: {body}"
        );
        assert!(
            !body.contains("Alpha.cs:10-40"),
            "hover must not expose raw byte ranges as occurrence labels: {body}"
        );
        assert!(
            body.contains(":2:3"),
            "hover must display a human line/column label: {body}"
        );
        Ok(())
    }

    #[test]
    fn build_for_cluster_wraps_markdown_in_hover_content() -> Result<()> {
        let cluster = make_cluster();
        let hover = build_for_cluster(&cluster);
        let HoverContents::Markup(markup) = hover.contents else {
            return Err(anyhow!("hover contents should be MarkupContent"));
        };
        assert_eq!(markup.kind, MarkupKind::Markdown);
        assert!(
            markup.value.contains("### Cluster abc123"),
            "value: {}",
            markup.value
        );
        assert!(
            hover.range.is_none(),
            "hover range must stay None so the client highlights the full cursor range"
        );
        Ok(())
    }

    #[test]
    fn markdown_for_cluster_falls_back_to_size_when_total_is_missing() {
        let mut cluster = make_cluster();
        cluster.size = 35;
        cluster.occurrences_total = 0;
        let body = markdown_for(&cluster);
        assert!(
            body.contains("**Occurrences (35)**"),
            "hover must fall back to cluster size for older reports: {body}"
        );
    }

    #[test]
    fn signals_table_formats_each_signal_row() {
        let cluster = make_cluster();
        let table = signals_table(&cluster);
        for row in [
            "| Signal | Value |",
            "| structural | 1.00 |",
            "| token_jaccard | 0.95 |",
            "| embedding_cos | 0.25 |",
            "| fused | 2.20 |",
        ] {
            assert!(table.contains(row), "row {row} missing: {table}");
        }
    }

    #[test]
    fn occurrences_block_with_empty_list_still_renders_header() {
        let mut cluster = make_cluster();
        cluster.size = 0;
        cluster.occurrences.clear();
        cluster.occurrences_total = 0;
        let block = occurrences_block(&cluster, None);
        assert!(
            block.contains("**Occurrences (0)**"),
            "header required even when empty: {block}"
        );
        assert!(
            !block.contains("- "),
            "no bullets should render for an empty list: {block}"
        );
    }
}
