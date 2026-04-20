//! Single-file HTML renderer.
//!
//! Human-readable view of the canonical report ([OUTPUT-HUMAN-HTML]).
//! Inline CSS, no JS, no external assets — opens offline. Each cluster
//! is rendered as a Terminal Card per the Kinetic Manuscript design
//! system: one example snippet expanded with syntax highlighting plus
//! a compact list of the other locations. The verbose run metadata
//! (action hints, schema doc, signal numbers) is tucked into a single
//! collapsed "Run details" footer so the body of the report stays
//! scannable. CSS lives in [`super::html_css`]; the footer in
//! [`super::html_footer`].

use std::{
    collections::HashMap,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    render::{
        highlight::highlight_snippet,
        html_css::{REPORT_CSS, SITE_CSS},
        html_footer::write_run_details,
    },
    report::{Report, ReportCluster, ReportOccurrence},
    report_metrics::ThresholdSource,
};

/// Renders `report` as a single-file HTML document. `scan_root` is the
/// directory occurrence paths are relative to; pass `None` (e.g. for
/// `--from-report` when the source is no longer available) and snippet
/// bodies degrade to a "source unavailable" placeholder.
#[must_use]
pub fn render_html(report: &Report, scan_root: Option<&Path>) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "<!doctype html><html lang=\"en\" data-theme=\"dark\"><head>"
    );
    write_head(&mut out, report);
    let _ = write!(out, "</head><body><main class=\"report-shell\">");
    write_intro(&mut out, report);
    let mut snippets = SnippetLoader::new(scan_root);
    write_clusters(&mut out, report, &mut snippets);
    write_run_details(&mut out, report, escape);
    let _ = write!(out, "</main></body></html>");
    out
}

/// Reads source files lazily and caches them so a cluster with many
/// occurrences in the same file does only one disk read.
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
        let cached = self.cache.entry(relative.to_path_buf()).or_insert_with(|| {
            let root = self.scan_root?;
            let absolute = root.join(relative);
            fs::read_to_string(&absolute).ok()
        });
        cached.as_deref()
    }
}

/// Returns the 1-indexed line number that contains `offset` in `source`.
fn line_for_offset(source: &str, offset: usize) -> usize {
    let safe = offset.min(source.len());
    let prefix = source.get(..safe).unwrap_or("");
    prefix
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
        .saturating_add(1)
}

/// Writes `<head>` metadata and the inline stylesheet.
fn write_head(out: &mut String, report: &Report) {
    let _ = write!(out, "<meta charset=\"utf-8\">");
    let _ = write!(
        out,
        "<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">",
    );
    let _ = write!(
        out,
        "<title>CodeDedup report — {clusters} duplicate group(s) across {files} file(s)</title>",
        clusters = report.clusters.len(),
        files = report.files_analysed,
    );
    let _ = write!(out, "<style>{SITE_CSS}{REPORT_CSS}</style>");
}

/// Writes the page title and a one-paragraph plain-English summary
/// telling the reader what the report is and how to read it.
fn write_intro(out: &mut String, report: &Report) {
    let _ = write!(
        out,
        "<h1>CodeDedup report</h1><p class=\"lede\">{summary}</p>",
        summary = escape(&intro_summary(report)),
    );
    write_metrics_banner(out, report);
}

/// Writes the repo-wide duplication banner per [METRICS-REPO]. Colour
/// comes from a class selector driven by the threshold verdict so
/// themes override it cleanly:
/// `metrics-banner--ok` (green) / `--breached` (red) / `--neutral`.
fn write_metrics_banner(out: &mut String, report: &Report) {
    let metrics = report.metrics;
    let variant = match (metrics.threshold.source, metrics.threshold.breached) {
        (ThresholdSource::None, _) => "neutral",
        (_, true) => "breached",
        (_, false) => "ok",
    };
    let _ = write!(
        out,
        "<p class=\"metrics-banner metrics-banner--{variant}\">{body}</p>",
        body = escape(&metrics_banner_text(report)),
    );
}

/// Plain-English metrics + threshold sentence rendered inside the
/// banner. Kept as text (not HTML) so escaping is uniform with the
/// rest of the intro.
fn metrics_banner_text(report: &Report) -> String {
    let metrics = report.metrics;
    let head = format!(
        "repo: {pct:.1}% duplicated ({dup} / {total} LOC, {clusters} clusters across {files} files)",
        pct = metrics.duplication_percent,
        dup = metrics.duplicated_loc,
        total = metrics.analysed_loc,
        clusters = metrics.clusters_total,
        files = metrics.duplicated_files,
    );
    match metrics.threshold.source {
        ThresholdSource::None => head,
        ThresholdSource::Cli | ThresholdSource::Config => {
            let verdict = if metrics.threshold.breached {
                "breached"
            } else {
                "ok"
            };
            format!(
                "{head} · threshold {pct:.2}% ({verdict})",
                pct = metrics.threshold.percent
            )
        }
    }
}

/// Builds the plain-English intro line. Avoids jargon; says what was
/// found, where to focus, and how the page is organised.
fn intro_summary(report: &Report) -> String {
    let groups = report.clusters.len();
    let files = report.files_analysed;
    let hidden = report.clusters_hidden;
    let kinds = classify_groups(&report.clusters);
    if groups == 0 {
        return format!("Scanned {files} file(s). No duplicated code worth reporting was found.");
    }
    let mut sentence =
        format!("Scanned {files} file(s) and found {groups} group(s) of duplicated code. ");
    sentence.push_str(&kinds);
    sentence.push_str(
        " Worst offenders are listed first — each card shows one example with syntax \
         highlighting and tells you where else the same code appears.",
    );
    if hidden > 0 {
        let _ = write!(sentence, " ({hidden} group(s) were hidden by your config.)");
    }
    sentence
}

/// Returns a one-line breakdown of how many groups in `clusters` look
/// identical vs nearly identical vs weakly similar. Plain English only.
fn classify_groups(clusters: &[ReportCluster]) -> String {
    let (mut exact, mut near, mut weak) = (0_usize, 0_usize, 0_usize);
    for cluster in clusters {
        match cluster_kind(cluster) {
            ClusterKind::Exact => exact = exact.saturating_add(1),
            ClusterKind::Near => near = near.saturating_add(1),
            ClusterKind::Weak => weak = weak.saturating_add(1),
        }
    }
    let mut parts: Vec<String> = Vec::new();
    if exact > 0 {
        parts.push(format!("{exact} identical (safe to merge)"));
    }
    if near > 0 {
        parts.push(format!("{near} nearly identical (review then merge)"));
    }
    if weak > 0 {
        parts.push(format!("{weak} loosely similar (treat as a hint)"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("Breakdown: {}.", parts.join(" · "))
    }
}

/// Coarse classification of a cluster from its fused signals. Drives
/// the colour band on the card and the verb in the intro.
#[derive(Clone, Copy)]
enum ClusterKind {
    /// Type-1/Type-2 exact / renamed clones — safe to extract.
    Exact,
    /// Strong token overlap or strong fused signal — review then merge.
    Near,
    /// LSH-only / weak fused — hint, not a directive.
    Weak,
}

/// Maps cluster signals to a [`ClusterKind`]. Mirrors the buckets in
/// [`crate::report::interpret`] so the HTML and the canonical
/// interpretation never disagree.
fn cluster_kind(cluster: &ReportCluster) -> ClusterKind {
    let signals = cluster.signals;
    if signals.structural >= 0.99 && signals.token_jaccard >= 0.99 {
        ClusterKind::Exact
    } else if signals.structural >= 0.99
        || (signals.structural > 0.0 && signals.token_jaccard >= 0.95)
        || (signals.structural <= 0.01 && signals.token_jaccard >= 0.90)
    {
        ClusterKind::Near
    } else {
        ClusterKind::Weak
    }
}

/// CSS class suffix for the card's left border. Drives the band
/// colour: crimson for exact, blue for near, neutral for weak.
fn kind_class(kind: ClusterKind) -> &'static str {
    match kind {
        ClusterKind::Exact => "kind-exact",
        ClusterKind::Near => "kind-near",
        ClusterKind::Weak => "kind-weak",
    }
}

/// Plain-English title for the card head, e.g.
/// `"Identical code in 12 places"`.
fn kind_title(kind: ClusterKind, occurrences: usize) -> String {
    let verb = match kind {
        ClusterKind::Exact => "Identical code",
        ClusterKind::Near => "Nearly identical code",
        ClusterKind::Weak => "Loosely similar code",
    };
    format!("{verb} in {occurrences} places")
}

/// Plain-English action sentence shown under the card title.
fn kind_action(kind: ClusterKind) -> &'static str {
    match kind {
        ClusterKind::Exact => {
            "Safe to extract into a single shared function — every copy is the same."
        }
        ClusterKind::Near => {
            "Review the example and the other locations — small differences may matter."
        }
        ClusterKind::Weak => "Loose textual overlap. Treat as a hint, not a directive.",
    }
}

/// Writes the section heading and every cluster card.
fn write_clusters(out: &mut String, report: &Report, snippets: &mut SnippetLoader<'_>) {
    let _ = write!(out, "<section><h2>Duplicate groups</h2>");
    if report.clusters.is_empty() {
        let _ = write!(out, "<p class=\"empty\">No duplication detected.</p>");
    }
    for cluster in &report.clusters {
        write_cluster_card(out, cluster, snippets);
    }
    let _ = write!(out, "</section>");
}

/// Writes a single cluster as a Terminal Card: title + action sentence
/// + one expanded example snippet + compact "also found in …" list.
fn write_cluster_card(out: &mut String, cluster: &ReportCluster, snippets: &mut SnippetLoader<'_>) {
    let kind = cluster_kind(cluster);
    let occurrences = &cluster.occurrences;
    let _ = write!(
        out,
        "<article class=\"cluster-card {kind_class}\">\
         <header class=\"cluster-card__head\">\
         <h3 class=\"cluster-card__title\">{title}</h3>\
         <span class=\"cluster-card__cost\">{cost}</span>\
         </header>\
         <p class=\"cluster-card__action\">{action}</p>",
        kind_class = kind_class(kind),
        title = escape(&kind_title(kind, occurrences.len())),
        cost = escape(&cost_chip(cluster)),
        action = escape(kind_action(kind)),
    );
    write_example(out, occurrences, snippets);
    write_also_list(out, occurrences);
    let _ = write!(out, "</article>");
}

/// Returns a compact "scope" chip text — number of AST nodes the
/// canonical example covers, in plain language.
fn cost_chip(cluster: &ReportCluster) -> String {
    let nodes = cluster.canonical_node_count;
    format!("~{nodes} AST nodes per copy")
}

/// Renders the canonical example: file path label + highlighted
/// snippet body.
fn write_example(
    out: &mut String,
    occurrences: &[ReportOccurrence],
    snippets: &mut SnippetLoader<'_>,
) {
    let Some(example) = occurrences.first() else {
        return;
    };
    let language = language_for_path(&example.path);
    match snippets.snippet(&example.path, example.start_byte, example.end_byte) {
        Some((source, start_line)) => {
            let end_line = start_line.saturating_add(source.matches('\n').count());
            let _ = write!(
                out,
                "<p class=\"cluster-card__example\">Example — {path}:{start}-{end}</p>",
                path = escape(&example.path.display().to_string()),
                start = start_line,
                end = end_line,
            );
            out.push_str(&render_snippet_body(&source, start_line, language));
        }
        None => {
            let _ = write!(
                out,
                "<p class=\"cluster-card__example\">Example — {path} (bytes {start}-{end})</p>\
                 <p class=\"snippet-missing\">Source unavailable on disk.</p>",
                path = escape(&example.path.display().to_string()),
                start = example.start_byte,
                end = example.end_byte,
            );
        }
    }
}

/// Renders the "also found in …" tail. Inline-prints the next five
/// locations and folds anything beyond that into a single `<details>`
/// so a 50-occurrence cluster produces a compact card, not a flood.
fn write_also_list(out: &mut String, occurrences: &[ReportOccurrence]) {
    if occurrences.len() <= 1 {
        return;
    }
    let inline_cap = 6_usize;
    let inline_end = occurrences.len().min(inline_cap);
    let _ = write!(
        out,
        "<p class=\"cluster-card__example\">Also found in:</p><ul class=\"also-list\">"
    );
    for occ in occurrences.iter().take(inline_end).skip(1) {
        write_also_item(out, occ);
    }
    let _ = write!(out, "</ul>");
    if occurrences.len() > inline_cap {
        let extra = occurrences.len().saturating_sub(inline_cap);
        let _ = write!(
            out,
            "<details class=\"also-toggle\"><summary>Show {extra} more location(s)</summary><ul class=\"also-list\">",
        );
        for occ in occurrences.iter().skip(inline_cap) {
            write_also_item(out, occ);
        }
        let _ = write!(out, "</ul></details>");
    }
}

/// One row in the "also found in" list. Path + line range + hidden
/// marker if applicable. No collapsibles, no per-occurrence snippets —
/// the canonical example already shows the code.
fn write_also_item(out: &mut String, occ: &ReportOccurrence) {
    let class = if occ.hidden { "is-hidden" } else { "" };
    let suffix = if occ.hidden {
        " · hidden by your config"
    } else {
        ""
    };
    let _ = write!(
        out,
        "<li class=\"{class}\">{path}<span class=\"also-loc\">bytes {start}-{end}{suffix}</span></li>",
        path = escape(&occ.path.display().to_string()),
        start = occ.start_byte,
        end = occ.end_byte,
    );
}

/// Soft cap on inline snippet height. A 320-line clone is real but
/// useless to scan visually — render the first [`SNIPPET_PREVIEW_LINES`]
/// lines in the visible block and fold the rest into a `<details>` so
/// the card stays compact while the full source remains one click away.
const SNIPPET_PREVIEW_LINES: usize = 40;

/// Renders the snippet body. Up to [`SNIPPET_PREVIEW_LINES`] are shown
/// inline; if the snippet is longer, the remainder is tucked into a
/// `<details>` continuing the line numbers, with a summary chip
/// reporting how many more lines were hidden.
fn render_snippet_body(source: &str, start_line: usize, language: &str) -> String {
    let highlighted = highlight_snippet(source, language);
    let lines: Vec<&str> = split_html_lines(&highlighted);
    let line_count = lines.len();
    let gutter_width = digits(start_line.saturating_add(line_count.saturating_sub(1)));
    let preview_end = line_count.min(SNIPPET_PREVIEW_LINES);
    let mut out = String::with_capacity(
        highlighted
            .len()
            .saturating_add(line_count.saturating_mul(20)),
    );
    let preview = lines.get(..preview_end).unwrap_or(&[]);
    write_snippet_pre(&mut out, preview, start_line, gutter_width);
    if line_count > preview_end {
        let hidden = line_count.saturating_sub(preview_end);
        let rest_start = start_line.saturating_add(preview_end);
        let rest = lines.get(preview_end..).unwrap_or(&[]);
        let _ = write!(
            &mut out,
            "<details class=\"also-toggle\"><summary>Show {hidden} more line(s)</summary>",
        );
        write_snippet_pre(&mut out, rest, rest_start, gutter_width);
        out.push_str("</details>");
    }
    out
}

/// Writes one `<pre class="snippet">` block for `lines`, numbering
/// each row from `start_line` and right-aligning the gutter to
/// `gutter_width` characters.
fn write_snippet_pre(out: &mut String, lines: &[&str], start_line: usize, gutter_width: usize) {
    out.push_str("<pre class=\"snippet\">");
    for (index, line) in lines.iter().enumerate() {
        let line_no = start_line.saturating_add(index);
        let _ = writeln!(
            out,
            "<span class=\"ln\">{line_no:>gutter_width$}</span> {line}",
        );
    }
    out.push_str("</pre>");
}

/// Splits `highlighted` HTML into one entry per source line. Splits on
/// raw `\n` bytes — the highlighter never emits `\n` inside a `<span>`
/// for the kinds we classify, so the split never breaks a tag.
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
/// context. Never emits entities for anything else so the output stays
/// human-diffable.
pub(super) fn escape(input: &str) -> String {
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
