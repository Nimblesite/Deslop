//! `textDocument/hover` provider ([LSP-HOVER]).
//!
//! Returns a markdown card for the cluster containing the cursor. The
//! card lays out the four signals, interpretation line, and occurrence
//! list — enough context that the reader can decide whether to
//! investigate without leaving the file.

use std::fmt::Write as _;

use deslop_core::report::ReportCluster;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

/// Builds the hover response for `cluster`.
#[must_use]
pub fn build_for_cluster(cluster: &ReportCluster) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown_for(cluster),
        }),
        range: None,
    }
}

/// Renders the hover markdown body.
#[must_use]
pub fn markdown_for(cluster: &ReportCluster) -> String {
    let header = format!(
        "### Cluster {id}\n\n{interpretation}\n\n",
        id = cluster.id,
        interpretation = cluster.interpretation
    );
    let table = signals_table(cluster);
    let occurrences = occurrences_block(cluster);
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
fn occurrences_block(cluster: &ReportCluster) -> String {
    let header = format!(
        "**Occurrences ({count})**:\n",
        count = cluster.occurrences.len()
    );
    let body = cluster
        .occurrences
        .iter()
        .fold(String::new(), |mut acc, occurrence| {
            let _ = writeln!(
                acc,
                "- {path}:{start}-{end}",
                path = occurrence.path.display(),
                start = occurrence.start_byte,
                end = occurrence.end_byte,
            );
            acc
        });
    format!("{header}{body}")
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
        // Occurrence bullet list carries both occurrences with byte ranges.
        assert!(body.contains("**Occurrences (2)**"), "occ header: {body}");
        assert!(body.contains("- Alpha.cs:10-40"), "alpha occ: {body}");
        assert!(body.contains("- Beta.cs:5-35"), "beta occ: {body}");
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
        cluster.occurrences.clear();
        let block = occurrences_block(&cluster);
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
