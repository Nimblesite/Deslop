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
