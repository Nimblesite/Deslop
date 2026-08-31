//! Markdown rendering for one mass-only cluster.

use std::fmt::Write as _;

use crate::report::{ReportCluster, ReportOccurrence};

/// Renders a cluster without inventing or selecting pair evidence.
#[must_use]
pub fn render_cluster_markdown<F>(cluster: &ReportCluster, source_of: F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let mut out = String::new();
    write_header(&mut out, cluster);
    let _ = writeln!(out, "## Occurrences\n");
    for (index, occurrence) in cluster.occurrences.iter().enumerate() {
        write_occurrence(&mut out, index.saturating_add(1), occurrence, &source_of);
    }
    out
}

/// Writes neutral cluster facts.
fn write_header(out: &mut String, cluster: &ReportCluster) {
    let _ = writeln!(out, "# Deslop cluster `{}`\n", cluster.id);
    let _ = writeln!(out, "- mass: `{}`", cluster.mass);
    let _ = writeln!(out, "- occurrences: `{}`", cluster.occurrence_count);
    let _ = writeln!(
        out,
        "- canonical nodes: `{}`\n",
        cluster.canonical_node_count
    );
}

/// Writes one occurrence heading and optional source snippet.
fn write_occurrence<F>(out: &mut String, rank: usize, occurrence: &ReportOccurrence, source_of: &F)
where
    F: Fn(&str) -> Option<String>,
{
    let path = occurrence.path.to_string_lossy();
    let Some(body) = source_of(&path) else {
        let _ = writeln!(out, "### {rank}. `{path}` _line unavailable_\n");
        return;
    };
    let (start_line, start_col) = byte_position(&body, occurrence.start_byte);
    let (end_line, end_col) = byte_position(&body, occurrence.end_byte);
    let snippet = slice_bytes(&body, occurrence.start_byte, occurrence.end_byte);
    let _ = writeln!(
        out,
        "### {rank}. `{path}:{start_line}:{start_col}` → `{end_line}:{end_col}`\n"
    );
    let _ = writeln!(out, "```\n{}\n```\n", snippet.trim_end_matches('\n'));
}

/// Converts a byte offset into a one-based line and byte column.
fn byte_position(body: &str, byte: usize) -> (usize, usize) {
    let capped = byte.min(body.len());
    let prefix = body.as_bytes().get(..capped).unwrap_or(&[]);
    let line = prefix.split(|candidate| *candidate == b'\n').count();
    let column = prefix
        .iter()
        .rposition(|candidate| *candidate == b'\n')
        .map_or_else(
            || capped.saturating_add(1),
            |newline| capped.saturating_sub(newline),
        );
    (line, column)
}

/// Copies a clamped byte range from `body`.
fn slice_bytes(body: &str, start: usize, end: usize) -> String {
    let bytes = body.as_bytes();
    let start = start.min(bytes.len());
    let end = end.min(bytes.len()).max(start);
    String::from_utf8_lossy(bytes.get(start..end).unwrap_or(&[])).into_owned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    const CLUSTER_ID: &str = "c-md";
    const LEFT_PATH: &str = "/tmp/A.cs";
    const RIGHT_PATH: &str = "/tmp/B.cs";
    const MASS: u64 = 10;
    const CANONICAL_NODES: usize = 10;
    const OCCURRENCE_COUNT: usize = 2;

    #[test]
    fn cluster_header_contains_mass_and_no_pair_evidence() {
        let out = render_cluster_markdown(&cluster(), |_| None);
        assert!(out.contains("Deslop cluster `c-md`"));
        assert!(out.contains("mass: `10`"));
        assert!(out.contains("occurrences: `2`"));
        assert!(out.contains("canonical nodes: `10`"));
        for forbidden in [
            "structural",
            "jaccard",
            "embedding",
            "agreement",
            "rename",
            "elected",
        ] {
            assert!(
                !out.to_lowercase().contains(forbidden),
                "pair evidence leaked through {forbidden}: {out}"
            );
        }
    }

    #[test]
    fn absent_source_uses_a_human_path_without_byte_offsets() {
        let out = render_cluster_markdown(&cluster(), |_| None);
        assert!(out.contains(LEFT_PATH));
        assert!(out.contains("_line unavailable_"));
        assert!(!out.contains("bytes"));
        assert!(!out.contains("```\n"));
    }

    #[test]
    fn known_source_renders_line_column_and_snippet() {
        let body = "alpha\nbeta\ngamma\n".to_owned();
        let out = render_cluster_markdown(&cluster(), move |_| Some(body.clone()));
        assert!(out.contains("/tmp/A.cs:1:1"));
        assert!(out.contains("```\nalpha\n```"));
    }

    #[test]
    fn byte_positions_are_clamped() {
        const BODY: &str = "alpha\nbeta\ngamma";
        assert_eq!(byte_position(BODY, 0), (1, 1));
        assert_eq!(byte_position(BODY, 6), (2, 1));
        assert_eq!(byte_position(BODY, usize::MAX), (3, 6));
    }

    fn cluster() -> ReportCluster {
        ReportCluster {
            id: CLUSTER_ID.to_owned(),
            rank: 1,
            rank_band: "worst".to_owned(),
            mass: MASS,
            canonical_node_count: CANONICAL_NODES,
            occurrences: vec![occurrence(LEFT_PATH), occurrence(RIGHT_PATH)],
            occurrences_total: OCCURRENCE_COUNT,
            occurrence_count: OCCURRENCE_COUNT,
            occurrences_truncated: false,
            intersects_diff: None,
            is_newly_introduced: None,
        }
    }

    fn occurrence(path: &str) -> ReportOccurrence {
        ReportOccurrence {
            path: PathBuf::from(path),
            start_byte: 0,
            end_byte: 5,
            start_line: 1,
            end_line: 1,
            hidden: false,
            in_diff: None,
        }
    }
}
