//! `textDocument/codeLens` provider ([LSP-CODE-LENS]).
//!
//! Emits one code lens per occurrence in the requested file. The lens
//! title carries the cluster count and the shared confidence
//! explanation — fused score plus measured content evidence
//! ([FUSION-CONTENT-GATE]) — so a reader sees the full context without
//! opening the report view, and the attached command jumps to the next
//! occurrence.

use std::path::Path;

use deslop_core::live::FileReport;
use deslop_core::render::signals::plain_explanation;
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
/// the front, cluster count, then the confidence explanation.
///
/// [FUSION-CONTENT-GATE] The signal breakdown is the one shared
/// `render::signals` rendering, so the lens states the fused confidence
/// and the measured content evidence rather than a hand-rolled subset
/// that cannot separate a corroborated rename from a scaffolding family.
fn title_for(cluster: &ReportCluster) -> String {
    format!(
        "●● {count} copies — {explanation} — jump to next",
        count = cluster.size,
        explanation = plain_explanation(cluster.signals),
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

#[cfg(test)]
#[allow(clippy::missing_docs_in_private_items)]
mod tests {
    use super::*;
    use anyhow::{anyhow, Result};
    use deslop_core::report::{ReportOccurrence, ReportSignals};
    use std::path::PathBuf;

    fn make_cluster(id: &str, size: usize, occurrences: Vec<ReportOccurrence>) -> ReportCluster {
        ReportCluster {
            id: id.to_owned(),
            weight: 10.0,
            size,
            canonical_node_count: 20,
            signals: ReportSignals {
                structural: 0.87,
                token_jaccard: 0.72,
                embedding_cos: 0.55,
                fused: 0.88,
                agreement: 0.63,
                rename_consistency: 0.94,
                literal_fraction: 0.11,
            },
            bucket: "nearly_identical".into(),
            category: "logic".into(),
            occurrences_total: occurrences.len(),
            occurrences_truncated: false,
            occurrences,
            summary: "s".into(),
            interpretation: "i".into(),
        }
    }

    fn occurrence(path: &str, start: usize, end: usize) -> ReportOccurrence {
        ReportOccurrence {
            path: PathBuf::from(path),
            start_byte: start,
            end_byte: end,
            start_line: 1,
            end_line: 1,
            hidden: false,
        }
    }

    #[test]
    fn build_for_file_emits_one_lens_per_matching_occurrence_with_full_command() -> Result<()> {
        let cluster = make_cluster(
            "cluster-alpha",
            3,
            vec![
                occurrence("Alpha.cs", 0, 10),
                occurrence("Alpha.cs", 50, 80),
                occurrence("Other.cs", 10, 20),
            ],
        );
        let total_occurrences = cluster.occurrences.len();
        let report = FileReport {
            path: PathBuf::from("Alpha.cs"),
            clusters: vec![cluster],
            total_occurrences,
        };
        let lenses = build_for_file(&report);
        assert_eq!(
            lenses.len(),
            2,
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
            assert_eq!(arguments.len(), 2, "cluster id + occurrence index");
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
            assert!(
                command
                    .title
                    .contains("structural 0.87 · jaccard 0.72 · embedding 0.55"),
                "title carries three signals to 2dp: {}",
                command.title
            );
            // [FUSION-CONTENT-GATE] #344: every lens states the fused
            // confidence and the measured content evidence behind it.
            assert!(
                command.title.contains(
                    "structural 0.87 · jaccard 0.72 · embedding 0.55 · fused 0.88 · \
                     agreement 0.63 · rename 0.94 · literal 0.11"
                ),
                "title carries all seven signals via render::signals: {}",
                command.title
            );
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
        let cluster = make_cluster("c", 2, vec![occurrence("Other.cs", 0, 5)]);
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
    fn title_for_formats_all_signals_to_two_decimal_places() {
        let cluster = make_cluster("c", 7, vec![]);
        let title = title_for(&cluster);
        assert!(title.contains("●● 7 copies"), "{title}");
        assert!(title.contains("structural 0.87"), "{title}");
        assert!(title.contains("jaccard 0.72"), "{title}");
        assert!(title.contains("embedding 0.55"), "{title}");
        assert!(title.ends_with("jump to next"), "{title}");
    }

    // [FUSION-CONTENT-GATE] #344: structural and jaccard alone cannot tell a
    // corroborated Type-2 rename from an anchor-poor scaffolding family, so
    // the lens must also state the fused score and the measured evidence.
    #[test]
    fn title_for_states_fused_confidence_and_measured_content_evidence() {
        let cluster = make_cluster("c", 7, vec![]);
        let title = title_for(&cluster);
        assert!(title.contains("fused 0.88"), "fused confidence: {title}");
        assert!(title.contains("agreement 0.63"), "byte agreement: {title}");
        assert!(
            title.contains("rename 0.94"),
            "Baker rename corroboration: {title}"
        );
        assert!(
            title.contains("literal 0.11"),
            "literal share of the match: {title}"
        );
        assert_eq!(
            title,
            format!(
                "●● 7 copies — {} — jump to next",
                deslop_core::render::signals::plain_explanation(cluster.signals)
            ),
            "the lens title must be the shared render::signals rendering, never a \
             second hand-rolled formatter"
        );
    }

    // A lens whose evidence differs must render differently — pins that the
    // title reads the cluster's own signals rather than a constant.
    #[test]
    fn title_for_tracks_each_clusters_own_evidence() {
        let mut anchor_poor = make_cluster("scaffolding", 4, vec![]);
        anchor_poor.signals = ReportSignals {
            structural: 1.0,
            token_jaccard: 1.0,
            embedding_cos: 0.99,
            fused: 0.42,
            agreement: 0.07,
            rename_consistency: 0.03,
            literal_fraction: 0.81,
        };
        let title = title_for(&anchor_poor);
        assert!(
            title.contains("structural 1.00 · jaccard 1.00"),
            "identical shape: {title}"
        );
        assert!(
            title.contains("fused 0.42 · agreement 0.07 · rename 0.03 · literal 0.81"),
            "anchor-poor evidence separates it from a corroborated rename: {title}"
        );
        assert!(
            !title.contains("agreement 0.63"),
            "must not echo another cluster's evidence: {title}"
        );
    }
}
