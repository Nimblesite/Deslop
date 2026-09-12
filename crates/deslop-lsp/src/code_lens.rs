//! `textDocument/codeLens` provider ([LSP-CODE-LENS]).
//!
//! Emits one code lens per occurrence in the requested file. The lens
//! title is the shared cluster presentation — category, occurrence count,
//! and mass for actual clones — followed by the jump action; the attached
//! command jumps to the next occurrence. Pair evidence belongs exclusively
//! to explicit two-endpoint comparison ([FUSED-PAIR-SIGNALS]).

use std::path::Path;

use deslop_core::{live::FileReport, report::ReportCluster};
use serde_json::json;
use tower_lsp::lsp_types::{CodeLens, Command, Position, Range};

use crate::diagnostics::occurrence_matches_path;

/// Command id forwarded back to the client for "jump to next occurrence".
pub const JUMP_COMMAND: &str = "deslop.jumpToNextOccurrence";

/// Trailing action the lens title ends with, naming what a click does.
const JUMP_ACTION: &str = "— jump to next";

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
    use std::path::PathBuf;

    use anyhow::{anyhow, Result};
    use deslop_core::{
        buckets::ClusterKind,
        report::{ReportCluster, ReportOccurrence},
    };

    use super::*;

    const ALPHA_FILE: &str = "Alpha.cs";
    const PAIR_SIZE: usize = 2;

    fn make_cluster(id: &str, occurrences: Vec<ReportOccurrence>) -> ReportCluster {
        deslop_core::report_fixtures::fixture_cluster(id, occurrences)
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
                command
                    .title
                    .starts_with("Nearly identical code × 3 — mass "),
                "[LSP-CODE-LENS] title leads with the category and the count: {}",
                command.title
            );
            assert!(
                !command.title.contains(BAND_GLYPH),
                "the retired band glyph is gone: {}",
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
        let cluster = make_cluster("c", vec![occurrence("Other.cs", 0, 5)]);
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
        let cluster = make_cluster("xyz-789", vec![occurrence("A.cs", 0, 1)]);
        let lens = lens_for_occurrence(&cluster, 4);
        let command = lens.command.ok_or_else(|| anyhow!("command populated"))?;
        let arguments = command
            .arguments
            .ok_or_else(|| anyhow!("arguments populated"))?;
        let first_arg = arguments.first().ok_or_else(|| anyhow!("first argument"))?;
        let second_arg = arguments.get(1).ok_or_else(|| anyhow!("second argument"))?;
        assert_eq!(*first_arg, serde_json::json!("xyz-789"));
        assert_eq!(*second_arg, serde_json::json!(4_usize));
        assert!(
            command
                .title
                .starts_with("Nearly identical code × 1 — mass "),
            "a single occurrence is counted without a plural defect: {}",
            command.title
        );
        assert!(!command.title.contains(BAND_GLYPH), "{}", command.title);
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
    fn title_for_renders_the_category_count_and_mass() {
        let cluster = make_cluster(
            "c",
            vec![occurrence("A.cs", 0, 1), occurrence("B.cs", 0, 1)],
        );
        let title = title_for(&cluster);
        assert_eq!(title, "Nearly identical code × 2 — mass 4 — jump to next");
        assert!(title.ends_with("jump to next"), "{}", title);
        assert!(!title.contains(BAND_GLYPH), "{title}");
    }

    // [FUSED-PAIR-SIGNALS] The admission signals are pair measurements and
    // never touch the cluster; a code lens on one occurrence must not render
    // them. The title states the copy count only.
    #[test]
    fn title_for_renders_no_pair_evidence() {
        let cluster = make_cluster(
            "c",
            vec![occurrence("A.cs", 0, 1), occurrence("B.cs", 0, 1)],
        );
        let title = title_for(&cluster);
        assert_eq!(title, "Nearly identical code × 2 — mass 4 — jump to next");
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
    fn title_never_renders_pair_scores() {
        let cluster = make_cluster("unsourced", vec![]);
        let title = title_for(&cluster);
        assert_eq!(title, "Nearly identical code × 0 — mass 0 — jump to next");
        assert!(!title.contains("structural"));
        assert!(!title.contains("agreement"));
    }

    /// [LSP-CODE-LENS] The category a clone lens must name, and the mass
    /// and copy count it must carry, as the shared presentation renders
    /// them.
    const CLONE_LENS_TITLE: &str =
        "Identical code \u{d7} 4 \u{2014} mass 142 \u{2014} jump to next";
    /// [LSP-CODE-LENS] A shape-only lens names the informational
    /// category and claims neither copies nor mass.
    const SHAPE_ONLY_LENS_TITLE: &str =
        "Same shape, different content \u{2014} informational, not a clone \u{2014} jump to next";
    /// The retired mass-percentile band glyph; [LSP-CODE-LENS] requires
    /// plain text.
    const BAND_GLYPH: char = '\u{25cf}';
    /// Mass of the clone the lens must publish.
    const CLONE_MASS: u64 = 142;
    /// Visible copies of the clone the lens must count.
    const CLONE_COPIES: usize = 4;
    /// Byte distance between fixture occurrences.
    const OCCURRENCE_STRIDE: usize = 10;
    /// Byte width of each fixture occurrence.
    const OCCURRENCE_WIDTH: usize = 5;

    /// A cluster of `kind` carrying `mass`, occurring `copies` times in
    /// [`ALPHA_FILE`].
    fn cluster_of_kind(kind: ClusterKind, mass: u64, copies: usize) -> ReportCluster {
        let occurrences = (0..copies)
            .map(|index| {
                let start = index.saturating_mul(OCCURRENCE_STRIDE);
                occurrence(ALPHA_FILE, start, start.saturating_add(OCCURRENCE_WIDTH))
            })
            .collect();
        let mut cluster = make_cluster("kinded", occurrences);
        cluster.kind = kind;
        cluster.mass = mass;
        cluster.severity = kind.default_diagnostic_severity().to_owned();
        cluster
    }

    // [LSP-CODE-LENS] The lens states the category, the occurrence count
    // and the mass, in plain text. The retired band glyph quantised mass
    // rank into dots; mass is now published as the number it is.
    #[test]
    fn a_clone_lens_states_its_category_count_and_mass_in_plain_text() {
        let cluster = cluster_of_kind(ClusterKind::Identical, CLONE_MASS, CLONE_COPIES);
        let title = title_for(&cluster);
        assert_eq!(
            title, CLONE_LENS_TITLE,
            "lens title follows [LSP-CODE-LENS]"
        );
        assert!(
            title.starts_with(ClusterKind::Identical.labels().title),
            "the lens leads with the category from the one kind registry \
             ([CLONE-KIND-LABELS]): {title}"
        );
        assert!(
            title.contains(&CLONE_MASS.to_string()),
            "a clone lens publishes its mass: {title}"
        );
        assert!(
            title.contains(&CLONE_COPIES.to_string()),
            "a clone lens counts its occurrences: {title}"
        );
        assert!(
            !title.contains(BAND_GLYPH),
            "[LSP-CODE-LENS] requires plain text, not a band glyph: {title}"
        );
        assert!(
            title.ends_with(" \u{2014} jump to next"),
            "the jump action stays last: {title}"
        );
    }

    // [CLONE-BUCKETS-DUAL-LABEL] / [CLONE-BUCKETS-STRUCTURAL-ONLY] Shape-only
    // is not a clone: its lens must say so and must claim neither copies nor
    // mass, which are duplicate quantities an informational record never owns.
    #[test]
    fn a_shape_only_lens_is_named_a_non_clone_and_claims_no_copies_or_mass() {
        let cluster = cluster_of_kind(ClusterKind::StructuralOnly, 0, CLONE_COPIES);
        let title = title_for(&cluster);
        assert_eq!(
            title, SHAPE_ONLY_LENS_TITLE,
            "shape-only lens follows [LSP-CODE-LENS]"
        );
        assert!(
            title.starts_with(ClusterKind::StructuralOnly.labels().title),
            "the informational category is named: {title}"
        );
        assert!(
            title.contains("not a clone"),
            "[CLONE-BUCKETS-STRUCTURAL-ONLY] shape-only is identified as a \
             non-clone: {title}"
        );
        assert!(
            !title.contains("copies"),
            "an informational finding has no copies to count: {title}"
        );
        assert!(
            !title.contains("mass"),
            "an informational record never claims duplicate mass: {title}"
        );
        assert!(
            !title.contains(BAND_GLYPH),
            "[LSP-CODE-LENS] requires plain text: {title}"
        );
    }
}
