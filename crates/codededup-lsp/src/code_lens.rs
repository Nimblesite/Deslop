//! `textDocument/codeLens` provider ([LSP-CODE-LENS]).
//!
//! Emits one code lens per occurrence in the requested file. The lens
//! title carries cluster count + signals so a reader sees the full
//! context without opening the report view, and the attached command
//! jumps to the next occurrence.

use std::path::Path;

use codededup_core::live::FileReport;
use codededup_core::report::{ReportCluster, ReportOccurrence};
use serde_json::json;
use tower_lsp::lsp_types::{CodeLens, Command, Position, Range};

/// Command id forwarded back to the client for "jump to next occurrence".
pub const JUMP_COMMAND: &str = "codededup.jumpToNextOccurrence";

/// Builds the code lenses for one file report.
#[must_use]
pub fn build_for_file(report: &FileReport) -> Vec<CodeLens> {
    report
        .clusters
        .iter()
        .flat_map(|cluster| lenses_for_cluster(cluster, &report.path))
        .collect()
}

/// Builds one code lens per occurrence of `cluster` that lives in
/// `path`.
fn lenses_for_cluster(cluster: &ReportCluster, path: &Path) -> Vec<CodeLens> {
    cluster
        .occurrences
        .iter()
        .enumerate()
        .filter(|(_, occurrence)| occurrence_matches_path(occurrence, path))
        .map(|(index, _occurrence)| lens_for_occurrence(cluster, index))
        .collect()
}

/// Matches occurrence paths against the report path with the usual
/// relative/absolute skew tolerance.
fn occurrence_matches_path(occurrence: &ReportOccurrence, path: &Path) -> bool {
    occurrence.path == path || occurrence.path.ends_with(path) || path.ends_with(&occurrence.path)
}

/// Builds a code lens at column zero of the cluster's first line for
/// the occurrence at `occurrence_index`.
fn lens_for_occurrence(cluster: &ReportCluster, occurrence_index: usize) -> CodeLens {
    CodeLens {
        range: zero_range(),
        command: Some(Command {
            title: title_for(cluster),
            command: JUMP_COMMAND.to_owned(),
            arguments: Some(vec![json!(cluster.id), json!(occurrence_index)]),
        }),
        data: None,
    }
}

/// Builds the lens title. Spec-compliant two-dot severity glyph at
/// the front, cluster count, then the three signals.
fn title_for(cluster: &ReportCluster) -> String {
    format!(
        "●● {count} copies — structural {structural:.2} · jaccard {jaccard:.2} · embedding {embedding:.2} — jump to next",
        count = cluster.size,
        structural = cluster.signals.structural,
        jaccard = cluster.signals.token_jaccard,
        embedding = cluster.signals.embedding_cos,
    )
}

/// Returns a zero-width range at position `(0, 0)` — the lens anchor.
fn zero_range() -> Range {
    Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 0,
            character: 0,
        },
    }
}
