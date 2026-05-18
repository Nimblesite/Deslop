//! `textDocument/hover` provider ([LSP-HOVER]).
//!
//! Returns a markdown card for the cluster containing the cursor. The
//! card lays out the signal sentence and occurrence
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

/// Which audience a rendered hover card targets. The human LSP hover
/// in an editor hides raw numeric signal scores; the agent-facing
/// `markdown_for*` API keeps them so callers scraping hovers through
/// the LSP protocol still see the full signal breakdown.
#[derive(Copy, Clone, Eq, PartialEq)]
enum Audience {
    /// Visible-to-human hover shown in VS Code / any LSP editor.
    Human,
    /// Machine-readable text rendered by the public `markdown_for*`
    /// API used by agent-facing report scrapers.
    Agent,
}

/// Builds the hover response for `cluster`.
#[must_use]
pub fn build_for_cluster(cluster: &ReportCluster) -> Hover {
    hover_from_markdown(human_markdown(std::slice::from_ref(cluster), None))
}

/// Builds the hover response for the clusters under the cursor.
#[must_use]
pub fn build_for_clusters_with_root(
    clusters: &[ReportCluster],
    workspace_root: &Path,
) -> Option<Hover> {
    let value = human_markdown(clusters, Some(workspace_root));
    (!value.is_empty()).then(|| hover_from_markdown(value))
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

/// Renders the agent-facing hover markdown body for a single cluster.
/// Keeps the signal breakdown so agents scraping the hover via the
/// LSP protocol see the same evidence as the JSON report.
#[must_use]
pub fn markdown_for(cluster: &ReportCluster) -> String {
    render_clusters(std::slice::from_ref(cluster), None, Audience::Agent)
}

/// Renders the agent-facing hover markdown for multiple clusters.
#[must_use]
pub fn markdown_for_clusters(clusters: &[ReportCluster]) -> String {
    render_clusters(clusters, None, Audience::Agent)
}

/// Renders human-visible hover markdown without raw signal details.
fn human_markdown(clusters: &[ReportCluster], workspace_root: Option<&Path>) -> String {
    render_clusters(clusters, workspace_root, Audience::Human)
}

/// Core rendering entry point; branches on `audience` to keep or drop
/// the signal sentence.
fn render_clusters(
    clusters: &[ReportCluster],
    workspace_root: Option<&Path>,
    audience: Audience,
) -> String {
    let mut cache: HashMap<PathBuf, Option<Vec<u8>>> = HashMap::new();
    let mut out = String::new();
    write_list_header(&mut out, clusters.len());
    for cluster in clusters {
        write_cluster_block(&mut out, cluster, workspace_root, &mut cache, audience);
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
    workspace_root: Option<&Path>,
    cache: &mut HashMap<PathBuf, Option<Vec<u8>>>,
    audience: Audience,
) {
    let _ = writeln!(out, "- **{}**", cluster_summary(cluster));
    if audience == Audience::Agent {
        let _ = writeln!(out, "  - {}", signal_sentence(cluster));
        write_occurrences(out, cluster, workspace_root, cache);
    }
    let _ = writeln!(out);
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
                token_jaccard: 1.0,
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
                    start_line: 1,
                    end_line: 1,
                    hidden: false,
                },
                ReportOccurrence {
                    path: PathBuf::from("Beta.cs"),
                    start_byte: 5,
                    end_byte: 35,
                    start_line: 1,
                    end_line: 1,
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
            body.contains("2 occurrences"),
            "hover must summarize the occurrence count in prose: {body}"
        );
        assert!(
            body.contains("Identical code"),
            "hover must use plain human labels: {body}"
        );
        assert!(
            !body.contains("Type-"),
            "hover must not expose clone taxonomy: {body}"
        );
        assert!(
            !body.contains("| Signal | Value |"),
            "hover must not render a large signal table: {body}"
        );
        // Occurrence bullet list carries both occurrences without byte ranges.
        assert!(body.contains("Occurrences:"), "occ header: {body}");
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
            body.contains("35 occurrences"),
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
            markup.value.contains("Identical code"),
            "value: {}",
            markup.value
        );
        assert!(
            !markup.value.contains("Type-"),
            "hover markup must stay human-readable: {}",
            markup.value
        );
        assert!(
            hover.range.is_none(),
            "hover range must stay None so the client highlights the full cursor range"
        );
        Ok(())
    }

    #[test]
    fn human_hover_omits_occurrence_list() -> Result<()> {
        // [LSP-HOVER] Human audience: compact summary only — the giant
        // occurrence list belongs in agent-facing markdown, not in the
        // card a human sees while coding.
        let cluster = make_cluster();
        let hover = build_for_cluster(&cluster);
        let HoverContents::Markup(markup) = hover.contents else {
            return Err(anyhow!(
                "expected HoverContents::Markup, got a different variant"
            ));
        };
        assert!(
            !markup.value.contains("Occurrences:"),
            "human hover must not dump the occurrence list: {}",
            markup.value
        );
        assert!(
            !markup.value.contains("Alpha.cs"),
            "human hover must not list individual occurrence paths: {}",
            markup.value
        );
        // Summary still carries the count phrase.
        assert!(
            markup.value.contains("occurrences"),
            "human hover must still state the total count: {}",
            markup.value
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
            body.contains("35 occurrences"),
            "hover must fall back to cluster size for older reports: {body}"
        );
    }

    #[test]
    fn markdown_for_cluster_includes_compact_signal_sentence() {
        let cluster = make_cluster();
        let body = markdown_for(&cluster);
        assert!(
            body.contains("signals: structural 1.00, jaccard 1.00, embedding 0.25, fused 2.20."),
            "compact signal sentence required: {body}"
        );
    }

    #[test]
    fn markdown_for_cluster_with_empty_occurrence_list_keeps_header() {
        let mut cluster = make_cluster();
        cluster.size = 0;
        cluster.occurrences.clear();
        cluster.occurrences_total = 0;
        let block = markdown_for(&cluster);
        assert!(
            block.contains("Occurrences:"),
            "header required even when empty: {block}"
        );
        assert!(
            !block.contains("    - "),
            "no bullets should render for an empty list: {block}"
        );
    }

    #[test]
    fn markdown_for_clusters_lists_every_cluster_with_slug() {
        let mut first = make_cluster();
        first.id = "abcdef0123456789".into();
        let mut second = make_cluster();
        second.id = "fedcba9876543210".into();
        second.bucket = "nearly_identical".into();
        second.signals.structural = 0.33;
        second.signals.token_jaccard = 0.96;
        second.interpretation = "Nearly identical code. Review both.".into();
        let body = markdown_for_clusters(&[first, second]);
        assert!(
            body.contains("Deslop clusters at this location (2)"),
            "multi-cluster hover must include a list heading: {body}"
        );
        assert!(
            body.contains("- **abcdef0 Identical code"),
            "first cluster headline must lead with its slug: {body}"
        );
        assert!(
            body.contains("- **fedcba9 Nearly identical code"),
            "second cluster headline must lead with its slug: {body}"
        );
        assert!(
            !body.contains("- **#"),
            "headlines must not lead with rank-as-id: {body}"
        );
        assert!(
            !body.contains("Type-"),
            "multi-cluster hover must not expose taxonomy labels: {body}"
        );
    }
}
