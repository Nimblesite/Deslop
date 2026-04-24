//! Markdown renderer for cluster virtual-document views
//! ([LSP-EDITOR-SURFACES]).
//!
//! Editor-neutral output so any LSP client can open a cluster in a
//! readonly markdown buffer without having to reach into the report
//! structure itself. Snippets and `line:column` locations come from a
//! caller-supplied source lookup — the renderer stays pure; the LSP
//! wraps it in a filesystem reader.

use std::fmt::Write as _;

use crate::report::{ReportCluster, ReportOccurrence};

/// Renders `cluster` as markdown. `source_of(path)` returns the full
/// source text of an occurrence path; when it returns `None` the
/// renderer falls back to `bytes start..end` in the occurrence heading
/// and omits the snippet block.
#[must_use]
pub fn render_cluster_markdown<F>(cluster: &ReportCluster, source_of: F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let mut out = String::new();
    write_header(&mut out, cluster);
    write_signals(&mut out, cluster);
    let _ = writeln!(out, "## Occurrences");
    let _ = writeln!(out);
    for (rank, occurrence) in cluster.occurrences.iter().enumerate() {
        write_occurrence(&mut out, rank + 1, occurrence, &source_of);
    }
    out
}

fn write_header(out: &mut String, cluster: &ReportCluster) {
    let _ = writeln!(out, "# Deslop cluster `{}`", cluster.id);
    let _ = writeln!(out);
    if !cluster.summary.is_empty() {
        let _ = writeln!(out, "{}", cluster.summary);
        let _ = writeln!(out);
    }
    if !cluster.interpretation.is_empty() {
        let _ = writeln!(out, "_{}_", cluster.interpretation);
        let _ = writeln!(out);
    }
}

fn write_signals(out: &mut String, cluster: &ReportCluster) {
    let _ = writeln!(out, "- weight: `{:.2}`", cluster.weight);
    let _ = writeln!(
        out,
        "- size: `{}` nodes (canonical `{}`)",
        cluster.size, cluster.canonical_node_count,
    );
    let signals = cluster.signals;
    let _ = writeln!(
        out,
        "- signals: structural=`{:.2}` jaccard=`{:.2}` embedding=`{:.2}` fused=`{:.2}`",
        signals.structural, signals.token_jaccard, signals.embedding_cos, signals.fused,
    );
    let _ = writeln!(out);
}

fn write_occurrence<F>(
    out: &mut String,
    rank: usize,
    occurrence: &ReportOccurrence,
    source_of: &F,
) where
    F: Fn(&str) -> Option<String>,
{
    let path = occurrence.path.to_string_lossy();
    let Some(body) = source_of(&path) else {
        let _ = writeln!(
            out,
            "### {rank}. `{path}` _bytes {}..{}_",
            occurrence.start_byte, occurrence.end_byte,
        );
        let _ = writeln!(out);
        return;
    };
    let (start_line, start_col) = byte_position(&body, occurrence.start_byte);
    let (end_line, end_col) = byte_position(&body, occurrence.end_byte);
    let _ = writeln!(
        out,
        "### {rank}. `{path}:{start_line}:{start_col}` → `{end_line}:{end_col}`",
    );
    let _ = writeln!(out);
    let snippet = slice_bytes(&body, occurrence.start_byte, occurrence.end_byte);
    let _ = writeln!(out, "```");
    let _ = writeln!(out, "{}", snippet.trim_end_matches('\n'));
    let _ = writeln!(out, "```");
    let _ = writeln!(out);
}

/// Converts a byte offset into a 1-based `(line, column)`, clamping
/// anything past the end of `body` to the last valid position. UTF-8
/// safe — columns count bytes so callers that need grapheme columns
/// should post-process.
fn byte_position(body: &str, byte: usize) -> (usize, usize) {
    let capped = byte.min(body.len());
    let prefix = &body.as_bytes()[..capped];
    let line = prefix.iter().filter(|b| **b == b'\n').count() + 1;
    let col = match prefix.iter().rposition(|b| *b == b'\n') {
        Some(nl) => capped - nl,
        None => capped + 1,
    };
    (line, col)
}

fn slice_bytes(body: &str, start: usize, end: usize) -> String {
    let bytes = body.as_bytes();
    let start = start.min(bytes.len());
    let end = end.min(bytes.len()).max(start);
    String::from_utf8_lossy(&bytes[start..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::report::{ReportOccurrence, ReportSignals};

    use super::*;

    fn occurrence(path: &str, start: usize, end: usize) -> ReportOccurrence {
        ReportOccurrence {
            path: PathBuf::from(path),
            start_byte: start,
            end_byte: end,
            hidden: false,
        }
    }

    fn cluster() -> ReportCluster {
        ReportCluster {
            id: "c-md".to_owned(),
            weight: 1.25,
            size: 40,
            canonical_node_count: 10,
            signals: ReportSignals {
                structural: 0.99,
                token_jaccard: 0.98,
                embedding_cos: 0.0,
                fused: 0.98,
            },
            bucket: String::new(),
            occurrences: vec![occurrence("/tmp/A.cs", 0, 5)],
            occurrences_total: 1,
            occurrences_truncated: false,
            summary: "Summary line.".to_owned(),
            interpretation: "Interpretation line.".to_owned(),
        }
    }

    #[test]
    fn renders_cluster_id_signals_and_summary_in_markdown_header() {
        let c = cluster();
        let out = render_cluster_markdown(&c, |_| None);
        assert!(out.contains("Deslop cluster `c-md`"));
        assert!(out.contains("Summary line."));
        assert!(out.contains("_Interpretation line._"));
        assert!(out.contains("weight: `1.25`"));
        assert!(out.contains("structural=`0.99`"));
    }

    #[test]
    fn falls_back_to_byte_range_when_source_unavailable() {
        let c = cluster();
        let out = render_cluster_markdown(&c, |_| None);
        assert!(out.contains("bytes 0..5"));
        assert!(!out.contains("```\n"));
    }

    #[test]
    fn writes_line_column_and_fenced_snippet_when_source_is_known() {
        let c = cluster();
        let body = "alpha\nbeta\ngamma\n".to_owned();
        let out = render_cluster_markdown(&c, move |_| Some(body.clone()));
        assert!(out.contains("/tmp/A.cs:1:1"), "expected line:col in heading; got: {out}");
        assert!(out.contains("```\nalpha\n```"), "expected fenced snippet; got: {out}");
    }

    #[test]
    fn byte_position_handles_first_line_and_wrapped_line() {
        let body = "alpha\nbeta\ngamma";
        assert_eq!(byte_position(body, 0), (1, 1));
        assert_eq!(byte_position(body, 5), (1, 6));
        assert_eq!(byte_position(body, 6), (2, 1));
        assert_eq!(byte_position(body, 11), (3, 1));
        assert_eq!(byte_position(body, 9999), (3, 6));
    }
}
