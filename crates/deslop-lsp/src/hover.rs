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
    report::{ReportCluster, ReportOccurrence},
    report_location::format_occurrence,
};
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

use crate::presentation::{cluster_summary, signal_sentence};

/// Builds the hover response for `cluster`.
#[must_use]
pub fn build_for_cluster(cluster: &ReportCluster) -> Hover {
    hover_from_markdown(markdown_for(cluster))
}

/// Builds the hover response for the clusters under the cursor.
#[must_use]
pub fn build_for_clusters_with_root(
    clusters: &[ReportCluster],
    ranked_clusters: &[ReportCluster],
    workspace_root: &Path,
) -> Option<Hover> {
    let value = markdown_for_clusters_with_root(clusters, ranked_clusters, Some(workspace_root));
    (!value.is_empty()).then(|| hover_from_markdown(value))
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

/// Renders the hover markdown for multiple clusters.
#[must_use]
pub fn markdown_for_clusters(
    clusters: &[ReportCluster],
    ranked_clusters: &[ReportCluster],
) -> String {
    markdown_for_clusters_with_root(clusters, ranked_clusters, None)
}

/// Renders the hover markdown body with optional path resolution.
fn markdown_for_with_root(cluster: &ReportCluster, workspace_root: Option<&Path>) -> String {
    markdown_for_clusters_with_root(std::slice::from_ref(cluster), &[], workspace_root)
}

/// Renders all hovered clusters as one human-readable markdown list.
fn markdown_for_clusters_with_root(
    clusters: &[ReportCluster],
    ranked_clusters: &[ReportCluster],
    workspace_root: Option<&Path>,
) -> String {
    let mut cache: HashMap<PathBuf, Option<Vec<u8>>> = HashMap::new();
    let mut out = String::new();
    write_list_header(&mut out, clusters.len());
    for cluster in clusters {
        let rank = rank_for(cluster, ranked_clusters);
        write_cluster_block(&mut out, cluster, rank, workspace_root, &mut cache);
    }
    out
}

/// Writes a short list heading when multiple clusters overlap.
fn write_list_header(out: &mut String, count: usize) {
    if count > 1 {
        let _ = writeln!(out, "**Deslop clusters at this location ({count})**\n");
    }
}

/// Writes one cluster and its nested detail rows.
fn write_cluster_block(
    out: &mut String,
    cluster: &ReportCluster,
    rank: Option<usize>,
    workspace_root: Option<&Path>,
    cache: &mut HashMap<PathBuf, Option<Vec<u8>>>,
) {
    let _ = writeln!(out, "- **{}**", cluster_summary(cluster, rank));
    write_interpretation(out, cluster);
    let _ = writeln!(out, "  - {}", signal_sentence(cluster));
    write_occurrences(out, cluster, workspace_root, cache);
    let _ = writeln!(out);
}

/// Writes the interpretation row when the report carries one.
fn write_interpretation(out: &mut String, cluster: &ReportCluster) {
    if !cluster.interpretation.trim().is_empty() {
        let _ = writeln!(out, "  - {}", cluster.interpretation.trim());
    }
}

/// Writes the nested occurrence location list.
fn write_occurrences(
    out: &mut String,
    cluster: &ReportCluster,
    workspace_root: Option<&Path>,
    cache: &mut HashMap<PathBuf, Option<Vec<u8>>>,
) {
    let _ = writeln!(out, "  - Occurrences:");
    for occurrence in &cluster.occurrences {
        let location = occurrence_display_label(occurrence, workspace_root, cache);
        let _ = writeln!(out, "    - {location}");
    }
}

/// Finds the one-based global impact rank for a cluster.
fn rank_for(cluster: &ReportCluster, ranked_clusters: &[ReportCluster]) -> Option<usize> {
    ranked_clusters
        .iter()
        .position(|ranked| ranked.id == cluster.id)
        .map(|index| index.saturating_add(1))
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
        // Header is for humans: rank/title/count, not the raw stable hash.
        assert!(
            !body.contains("### Cluster abc123"),
            "hover must not lead with the raw cluster hash: {body}"
        );
        assert!(
            body.contains("Identical code [Type-1/2]"),
            "hover must use the shared bucket title: {body}"
        );
        assert!(
            body.contains("2 occurrences"),
            "hover must summarize the occurrence count in prose: {body}"
        );
        assert!(
            body.contains("Identical code. Safe to extract."),
            "interpretation: {body}"
        );
        assert!(
            !body.contains("| Signal | Value |"),
            "hover must not render a large signal table: {body}"
        );
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
