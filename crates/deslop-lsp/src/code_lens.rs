//! `textDocument/codeLens` provider ([LSP-CODE-LENS]).
//!
//! Emits one code lens per occurrence in the requested file. The lens
//! title carries the cluster count and the elected pair's measured
//! axes plus content evidence
//! ([FUSED-CONTENT-GATE]) — so a reader sees the full context without
//! opening the report view, and the attached command jumps to the next
//! occurrence.

use std::path::Path;

use deslop_core::live::FileReport;
use deslop_core::report::ReportCluster;
use serde_json::json;
use tower_lsp::lsp_types::{CodeLens, Command, Position, Range};

use crate::diagnostics::occurrence_matches_path;

/// Command id forwarded back to the client for "jump to next occurrence".
pub const JUMP_COMMAND: &str = "deslop.jumpToNextOccurrence";

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
/// the front, cluster count, then the jump action.
///
/// [FUSED-PAIR-SIGNALS] The admission signals are pair measurements and
/// never touch the cluster; a code lens on one occurrence must not render
/// them. The title states the cluster's copy count only.
fn title_for(cluster: &ReportCluster) -> String {
    format!("●● {} copies — jump to next", cluster.occurrence_count)
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

#[cfg(test)]
#[allow(clippy::missing_docs_in_private_items)]
mod tests {
    use super::*;
    use anyhow::{anyhow, Result};
    use deslop_core::report::{ReportOccurrence, ReportSignalSource, ReportSignals};
    use std::path::PathBuf;

    const ALPHA_FILE: &str = "Alpha.cs";
    const PAIR_SIZE: usize = 2;

    fn make_cluster(id: &str, size: usize, occurrences: Vec<ReportOccurrence>) -> ReportCluster {
        let signals = ReportSignals {
            structural: 0.87,
            token_jaccard: 0.72,
            shape: 0.87,
            embedding_cos: 0.55,
            pair_agreement: 0.63,
            pair_rename_consistency: 0.94,
            literal_fraction: 0.11,
        };
        let mut cluster = deslop_core::report_fixtures::fixture_cluster(id, occurrences);
        cluster.weight = 10.0;
        cluster.size = size;
        cluster.canonical_node_count = 20;
        cluster.signals = signals;
        "nearly_identical".clone_into(&mut cluster.bucket);
        "s".clone_into(&mut cluster.summary);
        "i".clone_into(&mut cluster.interpretation);
        if cluster.occurrences.len() >= PAIR_SIZE {
            cluster.signal_source = Some(ReportSignalSource { left: 0, right: 1 });
        }
        deslop_core::report_fixtures::restamp_fixture(&mut cluster);
        cluster
    }

    fn occurrence(path: &str, start: usize, end: usize) -> ReportOccurrence {
        ReportOccurrence {
            path: PathBuf::from(path),
            start_byte: start,
            end_byte: end,
            start_line: 1,
            end_line: 1,
            hidden: false,
            in_diff: None,
        }
    }

    #[test]
    fn build_for_file_emits_one_lens_per_matching_occurrence_with_full_command() -> Result<()> {
        let cluster = make_cluster(
            "cluster-alpha",
            3,
            vec![
                occurrence(ALPHA_FILE, 0, 10),
                occurrence(ALPHA_FILE, 50, 80),
                occurrence("Other.cs", 10, 20),
            ],
        );
        let total_occurrences = cluster.occurrences.len();
        let report = FileReport {
            path: PathBuf::from(ALPHA_FILE),
            clusters: vec![cluster],
            total_occurrences,
        };
        let lenses = build_for_file(&report);
        assert_eq!(
            lenses.len(),
            PAIR_SIZE,
            "two matching occurrences → two lenses: {lenses:?}"
        );
        for (expected_index, lens) in lenses.iter().enumerate() {
            assert_eq!(lens.range, zero_range(), "lens range must be zero anchor");
            assert!(lens.data.is_none(), "no data payload on emitted lenses");
            let command = lens
                .command
                .as_ref()
                .ok_or_else(|| anyhow!("command populated"))?;
            assert_eq!(command.command, JUMP_COMMAND);
            assert_eq!(command.command, "deslop.jumpToNextOccurrence");
            let arguments = command
                .arguments
                .as_ref()
                .ok_or_else(|| anyhow!("command arguments populated"))?;
            assert_eq!(arguments.len(), PAIR_SIZE, "cluster id + occurrence index");
            let first_arg = arguments.first().ok_or_else(|| anyhow!("first argument"))?;
            let second_arg = arguments.get(1).ok_or_else(|| anyhow!("second argument"))?;
            assert_eq!(*first_arg, serde_json::json!("cluster-alpha"));
            let expected_occurrence_index = usize::from(expected_index != 0);
            assert_eq!(
                *second_arg,
                serde_json::json!(expected_occurrence_index),
                "occurrence index is the absolute position in cluster.occurrences"
            );
            assert!(
                command.title.starts_with("●● 3 copies — "),
                "title starts with severity glyph + count: {}",
                command.title
            );
            // [FUSED-PAIR-SIGNALS] The lens is a cluster surface and
            // renders no pair signals.
            for (gone, axis) in [
                ("structural 0.87", "shape"),
                ("agreement 0.63", "content"),
                ("rename 0.94", "rename"),
            ] {
                assert!(
                    !command.title.contains(gone),
                    "{axis} must not reach the lens title: {}",
                    command.title
                );
            }
            assert!(
                command.title.ends_with(" — jump to next"),
                "title ends with action: {}",
                command.title
            );
        }
        Ok(())
    }

    #[test]
    fn occurrence_matches_path_handles_relative_and_absolute_skew() {
        let absolute = Path::new("/ws/src/Alpha.cs");
        let relative_occ = occurrence("src/Alpha.cs", 0, 1);
        assert!(
            occurrence_matches_path(&relative_occ, absolute),
            "absolute report path ends_with relative occurrence path"
        );
        let absolute_occ = occurrence("/ws/src/Alpha.cs", 0, 1);
        let relative = Path::new("src/Alpha.cs");
        assert!(
            occurrence_matches_path(&absolute_occ, relative),
            "absolute occurrence path ends_with relative report path"
        );
        let equal_occ = occurrence("Beta.cs", 0, 1);
        assert!(
            occurrence_matches_path(&equal_occ, Path::new("Beta.cs")),
            "exact equality still matches"
        );
        let no_match = occurrence("Gamma.cs", 0, 1);
        assert!(
            !occurrence_matches_path(&no_match, Path::new("Delta.cs")),
            "unrelated paths do not match"
        );
    }

    #[test]
    fn build_for_file_with_no_matching_cluster_returns_empty_vec() {
        let cluster = make_cluster("c", PAIR_SIZE, vec![occurrence("Other.cs", 0, 5)]);
        let total_occurrences = cluster.occurrences.len();
        let report = FileReport {
            path: PathBuf::from("Alpha.cs"),
            clusters: vec![cluster],
            total_occurrences,
        };
        let lenses = build_for_file(&report);
        assert!(lenses.is_empty(), "nothing from a non-matching cluster");
    }

    #[test]
    fn lens_for_occurrence_preserves_cluster_id_and_index() -> Result<()> {
        let cluster = make_cluster("xyz-789", 5, vec![occurrence("A.cs", 0, 1)]);
        let lens = lens_for_occurrence(&cluster, 4);
        let command = lens.command.ok_or_else(|| anyhow!("command populated"))?;
        let arguments = command
            .arguments
            .ok_or_else(|| anyhow!("arguments populated"))?;
        let first_arg = arguments.first().ok_or_else(|| anyhow!("first argument"))?;
        let second_arg = arguments.get(1).ok_or_else(|| anyhow!("second argument"))?;
        assert_eq!(*first_arg, serde_json::json!("xyz-789"));
        assert_eq!(*second_arg, serde_json::json!(4_usize));
        assert!(command.title.contains("●● 5 copies"), "{}", command.title);
        Ok(())
    }

    #[test]
    fn zero_range_is_a_zero_width_anchor_at_origin() {
        let range = zero_range();
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 0);
        assert_eq!(range.start, range.end, "range is zero-width");
    }

    #[test]
    fn title_for_renders_copy_count_only() {
        let cluster = make_cluster(
            "c",
            7,
            vec![occurrence("A.cs", 0, 1), occurrence("B.cs", 0, 1)],
        );
        let title = title_for(&cluster);
        assert_eq!(title, "●● 7 copies — jump to next");
        assert!(title.ends_with("jump to next"), "{}", title);
    }

    // [FUSED-PAIR-SIGNALS] The admission signals are pair measurements and
    // never touch the cluster; a code lens on one occurrence must not render
    // them. The title states the copy count only.
    #[test]
    fn title_for_renders_no_pair_evidence() {
        let cluster = make_cluster(
            "c",
            7,
            vec![occurrence("A.cs", 0, 1), occurrence("B.cs", 0, 1)],
        );
        let title = title_for(&cluster);
        assert_eq!(title, "●● 7 copies — jump to next");
        for (gone, axis) in [
            ("structural", "shape"),
            ("jaccard", "token"),
            ("embedding", "embedding"),
            ("agreement", "content"),
            ("rename", "rename"),
            ("literal", "literal"),
            ("measured pair", "pair attribution"),
        ] {
            assert!(
                !title.contains(gone),
                "{axis} must not reach the lens title: {title}"
            );
        }
        assert!(
            !title.contains("fused"),
            "no cluster fused score on any surface: {title}"
        );
    }

    #[test]
    fn title_without_an_elected_pair_omits_every_pair_score() {
        let cluster = make_cluster("unsourced", 7, vec![]);
        let title = title_for(&cluster);
        assert_eq!(title, "●● 7 copies — jump to next");
        assert!(!title.contains("structural"));
        assert!(!title.contains("agreement"));
    }
}
