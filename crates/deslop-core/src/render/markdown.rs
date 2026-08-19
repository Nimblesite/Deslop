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
/// renderer prints the path alone and omits the snippet block so human
/// readers never see raw byte offsets ([LIVE-REPORT-DISPLAY]).
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
        write_occurrence(&mut out, rank.saturating_add(1), occurrence, &source_of);
    }
    out
}

/// Writes the cluster title and optional narrative text.
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

/// Writes the compact numeric signal summary.
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

/// Writes one occurrence heading and optional source snippet.
fn write_occurrence<F>(out: &mut String, rank: usize, occurrence: &ReportOccurrence, source_of: &F)
where
    F: Fn(&str) -> Option<String>,
{
    let path = occurrence.path.to_string_lossy();
    let Some(body) = source_of(&path) else {
        let _ = writeln!(out, "### {rank}. `{path}` _line unavailable_");
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
    let prefix = body.as_bytes().get(..capped).unwrap_or(&[]);
    let line = count_newlines(prefix).saturating_add(1);
    let col = match prefix.iter().rposition(|b| *b == b'\n') {
        Some(nl) => capped.saturating_sub(nl),
        None => capped.saturating_add(1),
    };
    (line, col)
}

/// Copies a clamped byte range from `body`.
fn slice_bytes(body: &str, start: usize, end: usize) -> String {
    let bytes = body.as_bytes();
    let start = start.min(bytes.len());
    let end = end.min(bytes.len()).max(start);
    String::from_utf8_lossy(bytes.get(start..end).unwrap_or(&[])).into_owned()
}

/// Counts newline bytes without pulling in a dependency for one renderer.
fn count_newlines(bytes: &[u8]) -> usize {
    let mut count = 0_usize;
    for byte in bytes {
        if *byte == b'\n' {
            count = count.saturating_add(1);
        }
    }
    count
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
            start_line: 0_i64,
            end_line: 0_i64,
            hidden: false,
            in_diff: None,
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
            category: String::new(),
            occurrences: vec![occurrence("/tmp/A.cs", 0, 5)],
            occurrences_total: 1,
            occurrences_truncated: false,
            summary: "Summary line.".to_owned(),
            interpretation: "Interpretation line.".to_owned(),
            intersects_diff: None,
            is_newly_introduced: None,
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
    fn no_source_fallback_omits_bytes_and_snippet_for_humans() {
        let c = cluster();
        let out = render_cluster_markdown(&c, |_| None);
        assert!(
            out.contains("/tmp/A.cs"),
            "fallback heading must name the file; got: {out}"
        );
        assert!(
            out.contains("_line unavailable_"),
            "fallback heading must read human-friendly, not byte offsets; got: {out}"
        );
        assert!(
            !out.contains("bytes"),
            "raw byte offsets must not leak into human markdown; got: {out}"
        );
        assert!(
            !out.contains("```\n"),
            "no snippet block when source is absent; got: {out}"
        );
    }

    #[test]
    fn writes_line_column_and_fenced_snippet_when_source_is_known() {
        let c = cluster();
        let body = "alpha\nbeta\ngamma\n".to_owned();
        let out = render_cluster_markdown(&c, move |_| Some(body.clone()));
        assert!(
            out.contains("/tmp/A.cs:1:1"),
            "expected line:col in heading; got: {out}"
        );
        assert!(
            out.contains("```\nalpha\n```"),
            "expected fenced snippet; got: {out}"
        );
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
