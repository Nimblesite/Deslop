//! `textDocument/codeLens` provider ([LSP-CODE-LENS]).
//!
//! Emits one code lens per occurrence in the requested file. The lens
//! title is the shared cluster presentation — category, occurrence count,
//! and mass for actual clones — followed by the jump action; the attached
//! command jumps to the next occurrence. Pair evidence belongs exclusively
//! to explicit two-endpoint comparison ([FUSED-PAIR-SIGNALS]).

use std::path::Path;

use deslop_core::{
    buckets::ClusterKind,
    live::FileReport,
    report::{ReportCluster, ReportOccurrence},
    report_facts::clone_kind_left_out_of_total,
};
use serde_json::json;
use tower_lsp::lsp_types::{CodeLens, Command, Position, Range};

use crate::diagnostics::occurrence_matches_path;

/// Command id forwarded back to the client for "jump to next occurrence".
pub const JUMP_COMMAND: &str = "deslop.jumpToNextOccurrence";

/// Trailing action the lens title ends with, naming what a click does.
const JUMP_ACTION: &str = "— jump to next";

/// The lens anchors at the start of its occurrence's first line.
const FIRST_COLUMN: u32 = 0;

/// Builds the code lenses for one file report. A clone kind the total
/// excludes ([METRICS-TOTAL-INCLUDE]) gets no lens, as it gets no
/// diagnostic ([SEVERITY-DIAGNOSTICS-EXCLUDED]): the editor greys it out,
/// so nothing in the code may badge it as counted duplication.
#[must_use]
pub fn build_for_file(report: &FileReport, total_includes: &[ClusterKind]) -> Vec<CodeLens> {
    report
        .clusters
        .iter()
        .filter(|cluster| !clone_kind_left_out_of_total(cluster.kind, total_includes))
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
        .map(|(index, occurrence)| lens_for_occurrence(cluster, index, occurrence))
        .collect()
}

/// Builds a code lens at column zero of the first line of `occurrence`,
/// which sits at `occurrence_index` in the cluster ([LSP-CODE-LENS]).
fn lens_for_occurrence(
    cluster: &ReportCluster,
    occurrence_index: usize,
    occurrence: &ReportOccurrence,
) -> CodeLens {
    CodeLens {
        range: first_line_range(occurrence),
        command: Some(Command {
            title: title_for(cluster),
            command: JUMP_COMMAND.to_owned(),
            arguments: Some(vec![json!(cluster.id), json!(occurrence_index)]),
        }),
        data: None,
    }
}

/// Builds the lens title: the shared cluster presentation, then the jump
/// action ([LSP-CODE-LENS]).
///
/// The description comes from [`crate::presentation::diagnostic_message`],
/// the one renderer the diagnostic uses, so a lens and a diagnostic on the
/// same cluster never describe it differently. It names the category
/// ([CLONE-KIND-LABELS]), counts the occurrences, and publishes mass for
/// actual clones while identifying shape-only as the non-clone it is
/// ([CLONE-BUCKETS-DUAL-LABEL]).
///
/// [FUSED-PAIR-SIGNALS] The admission signals are pair measurements and
/// never touch the cluster; a code lens on one occurrence must not render
/// them.
fn title_for(cluster: &ReportCluster) -> String {
    format!(
        "{description} {JUMP_ACTION}",
        description = crate::presentation::diagnostic_message(cluster)
    )
}

/// Returns a zero-width range at column zero of the occurrence's first
/// line — the lens anchor ([LSP-CODE-LENS]). The wire reports one-based
/// lines; LSP positions are zero-based.
fn first_line_range(occurrence: &ReportOccurrence) -> Range {
    let line = u32::try_from(occurrence.start_line.saturating_sub(1).max(0)).unwrap_or(u32::MAX);
    let anchor = Position {
        line,
        character: FIRST_COLUMN,
    };
    Range {
        start: anchor,
        end: anchor,
    }
}

#[cfg(test)]
#[path = "code_lens/tests.rs"]
mod tests;
