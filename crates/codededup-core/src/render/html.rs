//! Single-file HTML renderer.
//!
//! Human-readable view of the canonical report. Inline CSS, no JS, no
//! external fonts — a report opened from disk on a fresh machine works
//! offline. Per [OUTPUT-HUMAN-HTML], each occurrence is a collapsible
//! `<details>` panel containing the source snippet (loaded from
//! `scan_root + occurrence.path` at render time) with line numbers and
//! tree-sitter-driven syntax highlighting.

use std::{
    collections::HashMap,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    render::highlight::highlight_snippet,
    report::{Report, ReportCluster, ReportOccurrence},
};

/// First N occurrences of each cluster open by default; the rest stay
/// collapsed so a 256-occurrence cluster doesn't blow up the page.
const OPEN_OCCURRENCES_PER_CLUSTER: usize = 1;

/// Renders `report` as a single-file HTML document. `scan_root` is the
/// directory occurrence paths are relative to; pass `None` (e.g. for
/// `--from-report` when the source is no longer available) to get the
/// terse byte-offset-only view per occurrence.
#[must_use]
pub fn render_html(report: &Report, scan_root: Option<&Path>) -> String {
    let mut out = String::new();
    let _ = write!(out, "<!doctype html><html lang=\"en\"><head>");
    write_head(&mut out, report);
    let _ = write!(out, "</head><body>");
    write_header(&mut out, report);
    write_action_hints(&mut out, report);
    write_schema_doc(&mut out, report);
    let mut snippets = SnippetLoader::new(scan_root);
    write_clusters(&mut out, report, &mut snippets);
    let _ = write!(out, "</body></html>");
    out
}

/// Reads source files lazily and caches them so a cluster with many
/// occurrences in the same file does only one disk read. `None`
/// `scan_root` makes every load a miss — snippet panels degrade to the
/// "source unavailable" placeholder per [OUTPUT-HUMAN-HTML].
struct SnippetLoader<'a> {
    /// Directory occurrence paths resolve against. `None` disables disk
    /// reads entirely.
    scan_root: Option<&'a Path>,
    /// Cache keyed by relative path.
    cache: HashMap<PathBuf, Option<String>>,
}

impl<'a> SnippetLoader<'a> {
    /// Creates a loader rooted at `scan_root` (or no-op if `None`).
    fn new(scan_root: Option<&'a Path>) -> Self {
        Self {
            scan_root,
            cache: HashMap::new(),
        }
    }

    /// Returns the snippet of `source[start..end]` for the file at
    /// `relative` under the configured scan root, plus the 1-indexed
    /// starting line number. `None` when the file cannot be loaded or
    /// the byte range is outside the file.
    fn snippet(&mut self, relative: &Path, start: usize, end: usize) -> Option<(String, usize)> {
        let source = self.source(relative)?;
        let safe_end = end.min(source.len());
        let safe_start = start.min(safe_end);
        let slice = source.get(safe_start..safe_end)?;
        let line = line_for_offset(source, safe_start);
        Some((slice.to_owned(), line))
    }

    /// Resolves and caches the full source text for `relative`. Returns
    /// `None` when the source cannot be read as UTF-8 (binary blobs are
    /// not displayed inline).
    fn source(&mut self, relative: &Path) -> Option<&str> {
        let cached = self
            .cache
            .entry(relative.to_path_buf())
            .or_insert_with(|| {
                let root = self.scan_root?;
                let absolute = root.join(relative);
                fs::read_to_string(&absolute).ok()
            });
        cached.as_deref()
    }
}

/// Returns the 1-indexed line number that contains `offset` in `source`.
/// Counts `\n` bytes directly so we don't allocate a per-line table for
/// each lookup.
fn line_for_offset(source: &str, offset: usize) -> usize {
    let safe = offset.min(source.len());
    let prefix = source.get(..safe).unwrap_or("");
    prefix.bytes().filter(|b| *b == b'\n').count().saturating_add(1)
}

/// Writes `<head>` metadata and the inline stylesheet.
fn write_head(out: &mut String, report: &Report) {
    let _ = write!(out, "<meta charset=\"utf-8\">");
    let _ = write!(
        out,
        "<title>codededup report (schema v{schema})</title>",
        schema = report.report_schema_version,
    );
    let _ = write!(out, "<style>{CSS}</style>");
}

/// Writes the page banner with run stats.
fn write_header(out: &mut String, report: &Report) {
    let _ = write!(
        out,
        "<header><h1>CodeDedup report</h1>\
         <p>Tool <code>{tool}</code> · schema v{schema} · {files} file(s) · \
         {visible} visible cluster(s) · {hidden} hidden</p>\
         <p class=\"embeddings\">{embeddings}</p></header>",
        tool = escape(&report.tool_version),
        schema = report.report_schema_version,
        files = report.files_analysed,
        visible = report.clusters.len(),
        hidden = report.clusters_hidden,
        embeddings = escape(&format_provenance(report)),
    );
}

/// Returns the human-readable embedding provenance line for the
/// header. Mirrors the text-renderer format so the two views agree.
fn format_provenance(report: &Report) -> String {
    report.embedding_provenance.as_ref().map_or_else(
        || "embeddings: off".to_owned(),
        |provenance| {
            format!(
                "embeddings: {provider}/{model}@{version} ({dims}-d)",
                provider = provenance.provider_id,
                model = provenance.model_id,
                version = provenance.model_version,
                dims = provenance.dimensions,
            )
        },
    )
}

/// Writes the action-hint playbook so a reader sees it before the
/// clusters and can apply it as a decision table.
fn write_action_hints(out: &mut String, report: &Report) {
    if report.action_hints.is_empty() {
        return;
    }
    let _ = write!(out, "<section class=\"hints\"><h2>Action hints</h2><ul>");
    for hint in &report.action_hints {
        let _ = write!(
            out,
            "<li><code>{pattern}</code> — {rec}</li>",
            pattern = escape(&hint.pattern),
            rec = escape(&hint.recommendation),
        );
    }
    let _ = write!(out, "</ul></section>");
}

/// Embeds the canonical schema documentation in a collapsed
/// `<details>` so the page opens compact but the reference is one
/// click away.
fn write_schema_doc(out: &mut String, report: &Report) {
    let _ = write!(
        out,
        "<details class=\"schema-doc\"><summary>Schema reference</summary><pre>{doc}</pre></details>",
        doc = escape(&report.schema_doc),
    );
}

/// Writes each cluster in ranked order.
fn write_clusters(out: &mut String, report: &Report, snippets: &mut SnippetLoader<'_>) {
    let _ = write!(out, "<section class=\"clusters\"><h2>Clusters</h2>");
    if report.clusters.is_empty() {
        let _ = write!(out, "<p class=\"empty\">No duplication detected.</p>");
    }
    for (idx, cluster) in report.clusters.iter().enumerate() {
        write_cluster(out, idx, cluster, snippets);
    }
    let _ = write!(out, "</section>");
}

/// Writes a single cluster block with its occurrences.
fn write_cluster(
    out: &mut String,
    idx: usize,
    cluster: &ReportCluster,
    snippets: &mut SnippetLoader<'_>,
) {
    let _ = write!(
        out,
        "<article class=\"cluster\"><h3>#{rank} <code>{id}</code></h3>\
         <p class=\"meta\">weight {weight:.2} · size {size} · {nodes} nodes</p>\
         <p class=\"signals\">structural {s:.2} · token_jaccard {j:.2} · embedding_cos {e:.2} · fused {f:.2}</p>\
         <p class=\"interp\">{interp}</p>",
        rank = idx.saturating_add(1),
        id = escape(&cluster.id),
        weight = cluster.weight,
        size = cluster.size,
        nodes = cluster.canonical_node_count,
        s = cluster.signals.structural,
        j = cluster.signals.token_jaccard,
        e = cluster.signals.embedding_cos,
        f = cluster.signals.fused,
        interp = escape(&cluster.interpretation),
    );
    write_occurrences(out, &cluster.occurrences, snippets);
    let _ = write!(out, "</article>");
}

/// Writes every occurrence of `cluster` as a `<details>` panel
/// containing the syntax-highlighted snippet. The first
/// [`OPEN_OCCURRENCES_PER_CLUSTER`] occurrences are rendered with the
/// `open` attribute so the cluster lands open-by-default at one
/// example; the rest stay collapsed.
fn write_occurrences(
    out: &mut String,
    occurrences: &[ReportOccurrence],
    snippets: &mut SnippetLoader<'_>,
) {
    let _ = write!(out, "<div class=\"occurrences\">");
    for (index, occ) in occurrences.iter().enumerate() {
        write_occurrence_panel(out, index, occ, snippets);
    }
    let _ = write!(out, "</div>");
}

/// Renders one occurrence as a `<details>` panel: summary line carries
/// the path + line range + hidden marker; body carries the highlighted
/// snippet (or the placeholder when source is unavailable).
fn write_occurrence_panel(
    out: &mut String,
    index: usize,
    occ: &ReportOccurrence,
    snippets: &mut SnippetLoader<'_>,
) {
    let language = language_for_path(&occ.path);
    let snippet = snippets.snippet(&occ.path, occ.start_byte, occ.end_byte);
    let (line_label, body) = match snippet {
        Some((source, start_line)) => {
            let end_line = start_line.saturating_add(source.matches('\n').count());
            (
                format!(":{start_line}-{end_line}"),
                render_snippet_body(&source, start_line, language),
            )
        }
        None => (
            format!(" (bytes {}-{})", occ.start_byte, occ.end_byte),
            "<p class=\"snippet-missing\">source unavailable</p>".to_owned(),
        ),
    };
    let class = if occ.hidden {
        "occurrence hidden"
    } else {
        "occurrence"
    };
    let open = if index < OPEN_OCCURRENCES_PER_CLUSTER {
        " open"
    } else {
        ""
    };
    let hidden_marker = if occ.hidden { " · hidden" } else { "" };
    let _ = write!(
        out,
        "<details class=\"{class}\"{open}><summary><code>{path}</code>{line_label}{hidden_marker}</summary>{body}</details>",
        path = escape(&occ.path.display().to_string()),
    );
}

/// Renders the snippet body: a `<pre>` containing one `<div>` per
/// source line with a leading line-number gutter, with the source bytes
/// passed through the syntax highlighter.
fn render_snippet_body(source: &str, start_line: usize, language: &str) -> String {
    let highlighted = highlight_snippet(source, language);
    let lines: Vec<&str> = split_html_lines(&highlighted);
    let line_count = lines.len();
    let gutter_width = digits(start_line.saturating_add(line_count.saturating_sub(1)));
    let mut out = String::with_capacity(highlighted.len().saturating_add(line_count.saturating_mul(20)));
    out.push_str("<pre class=\"snippet\">");
    for (index, line) in lines.iter().enumerate() {
        let line_no = start_line.saturating_add(index);
        let _ = writeln!(
            out,
            "<span class=\"ln\">{line_no:>gutter_width$}</span> {line}",
        );
    }
    out.push_str("</pre>");
    out
}

/// Splits `highlighted` HTML into one entry per source line. Splits on
/// raw `\n` bytes — the highlighter never emits a `\n` inside a `<span>`
/// (tree-sitter leaves don't span line boundaries for the kinds we
/// classify) so the splits never break a tag.
fn split_html_lines(highlighted: &str) -> Vec<&str> {
    if highlighted.is_empty() {
        return vec![""];
    }
    highlighted.split('\n').collect()
}

/// Returns the decimal digit count of `value`, with a floor of 1.
fn digits(value: usize) -> usize {
    let mut n = value;
    if n == 0 {
        return 1;
    }
    let mut count: usize = 0;
    while n > 0 {
        count = count.saturating_add(1);
        n /= 10;
    }
    count
}

/// Maps a file extension to a language id understood by the
/// highlighter. Unknown extensions return `"unknown"`, which the
/// highlighter degrades to plain escaped text.
fn language_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("cs") => "csharp",
        Some("rs") => "rust",
        Some("py") => "python",
        _ => "unknown",
    }
}

/// HTML-escapes the four characters that can break out of content
/// context. Never emits entities for anything else so the output
/// stays human-diffable.
fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}

/// Inline stylesheet. Plain, readable, no web fonts. Keeps the report
/// legible on any browser without network access.
const CSS: &str = "body{font-family:system-ui,-apple-system,sans-serif;max-width:960px;margin:2em auto;padding:0 1em;color:#111}\
header h1{margin-bottom:0}header p{color:#555}\
.hints ul{list-style:none;padding:0}.hints li{margin:.25em 0}\
.schema-doc pre{white-space:pre-wrap;background:#f6f6f6;padding:1em;border-radius:4px}\
.cluster{border:1px solid #ddd;border-radius:6px;padding:1em;margin:1em 0}\
.cluster h3{margin:0 0 .25em}.cluster .meta,.cluster .signals{color:#555;margin:.15em 0}\
.cluster .interp{font-style:italic;margin:.25em 0 .75em}\
.occurrences{list-style:none;padding-left:0}\
.occurrences li{font-family:ui-monospace,Menlo,Consolas,monospace;font-size:.9em}\
.occurrences li.hidden{color:#888}\
code{background:#f6f6f6;padding:.1em .3em;border-radius:3px}";
