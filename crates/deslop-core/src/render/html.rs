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
    buckets::{bucket_labels, classify, ClusterKind},
    clone_category::CloneCategory,
    pipeline::language_for_path,
    render::{
        highlight::highlight_snippet,
        html_css::{REPORT_CSS, SITE_CSS},
        html_escape::escape,
        html_footer::write_run_details,
    },
    report::{Report, ReportCluster, ReportOccurrence},
    report_location::format_occurrence,
    report_metrics::ThresholdSource,
};

/// Renders `report` as a single-file HTML document. `scan_root` is the
/// directory occurrence paths are relative to; pass `None` (e.g. for
/// `--from-report` when the source is no longer available) and snippet
/// bodies degrade to a "source unavailable" placeholder. When
/// `split_by_language` is set the body is divided into one section per
/// language ([OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS]); otherwise the
/// output is byte-identical to the single ranked list.
#[must_use]
pub fn render_html(report: &Report, scan_root: Option<&Path>, split_by_language: bool) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "<!doctype html><html lang=\"en\" data-theme=\"dark\"><head>"
    );
    write_head(&mut out, report);
    let _ = write!(out, "</head><body><main class=\"report-shell\">");
    write_intro(&mut out, report, split_by_language);
    let mut snippets = SnippetLoader::new(scan_root);
    write_clusters(&mut out, report, &mut snippets, split_by_language);
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

    /// Formats the occurrence location from cached source when present.
    fn location(&mut self, relative: &Path, start: usize) -> String {
        let source = self.source(relative).map(str::as_bytes);
        format_occurrence(relative, start, source)
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
        "<title>Deslop report — {clusters} duplicate group(s) across {files} file(s)</title>",
        clusters = report.clusters.len(),
        files = report.files_analysed,
    );
    let _ = write!(out, "<style>{SITE_CSS}{REPORT_CSS}</style>");
}

/// Writes the page title and a one-paragraph plain-English summary
/// telling the reader what the report is and how to read it. When
/// `split_by_language` is set the lede gains a per-language breakdown
/// ([OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS]); otherwise it is unchanged.
fn write_intro(out: &mut String, report: &Report, split_by_language: bool) {
    let mut lede = intro_summary(report);
    if split_by_language {
        let breakdown = language_breakdown(report);
        if !breakdown.is_empty() {
            lede.push(' ');
            lede.push_str(&breakdown);
        }
    }
    let _ = write!(
        out,
        "<h1>Deslop report</h1><p class=\"lede\">{summary}</p>",
        summary = escape(&lede),
    );
    write_metrics_banner(out, report);
}

/// Writes the repo-wide duplication banner per [METRICS-REPO]. Colour
/// comes from a class selector driven by the threshold verdict so
/// themes override it cleanly:
/// `metrics-banner--ok` (green) / `--breached` (red) / `--neutral`.
fn write_metrics_banner(out: &mut String, report: &Report) {
    let metrics = &report.metrics;
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
    let metrics = &report.metrics;
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

/// Returns a one-line breakdown of how many groups fall into each
/// [CLONE-BUCKETS] bucket. Pure-visual surface (HTML report body) so
/// uses `plain_title` lower-cased — no `Type-N` per
/// [CLONE-BUCKETS-DUAL-LABEL].
fn classify_groups(clusters: &[ReportCluster]) -> String {
    let parts: Vec<String> = ClusterKind::all()
        .into_iter()
        .filter_map(|kind| {
            let count = clusters
                .iter()
                .filter(|cluster| classify(cluster) == kind)
                .count();
            if count == 0 {
                return None;
            }
            let labels = bucket_labels(kind);
            Some(format!("{count} {}", labels.plain_title.to_lowercase()))
        })
        .collect();
    if parts.is_empty() {
        String::new()
    } else {
        format!("Breakdown: {}.", parts.join(" · "))
    }
}

/// CSS class suffix for the card's left border. Drives the band
/// colour family per [CLONE-BUCKETS]: green/crimson for identical,
/// yellow/blue for nearly identical, neutral for loosely similar,
/// purple/cyan for same-behavior.
fn kind_class(kind: ClusterKind) -> String {
    format!("kind-{}", bucket_labels(kind).css_suffix)
}

/// Plain-visual title for the card head, e.g.
/// `"Identical code in 12 places"`. Pure-visual surface — no `Type-N`
/// ever per [CLONE-BUCKETS-DUAL-LABEL].
fn kind_title(kind: ClusterKind, occurrences: usize) -> String {
    format!(
        "{} in {occurrences} places",
        bucket_labels(kind).plain_title
    )
}

/// Plain-visual action sentence shown under the card title.
fn kind_action(kind: ClusterKind) -> &'static str {
    bucket_labels(kind).action_sentence
}

/// Dispatches between the single ranked list and the per-language
/// sections ([OUTPUT-HUMAN-HTML-LANGUAGE-SECTIONS]). The flat path is
/// used for an empty report so the "no duplication" message is
/// identical in both modes.
fn write_clusters(
    out: &mut String,
    report: &Report,
    snippets: &mut SnippetLoader<'_>,
    split_by_language: bool,
) {
    if split_by_language && !report.clusters.is_empty() {
        write_clusters_by_language(out, report, snippets);
    } else {
        write_clusters_flat(out, report, snippets);
    }
}

/// Writes the single "Duplicate groups" section with every cluster card
/// in global worst-first order.
fn write_clusters_flat(out: &mut String, report: &Report, snippets: &mut SnippetLoader<'_>) {
    let _ = write!(out, "<section><h2>Duplicate groups</h2>");
    if report.clusters.is_empty() {
        let _ = write!(out, "<p class=\"empty\">No duplication detected.</p>");
    }
    for cluster in &report.clusters {
        write_cluster_card(out, cluster, snippets);
    }
    let _ = write!(out, "</section>");
}

/// Writes one `<section>` per language, each headed by the language
/// display name and its group count. Sections are ordered by worst
/// cluster weight (first-seen in the globally worst-first list) and
/// clusters keep their worst-first order within each section.
fn write_clusters_by_language(out: &mut String, report: &Report, snippets: &mut SnippetLoader<'_>) {
    for (language, clusters) in group_clusters_by_language(&report.clusters) {
        let _ = write!(
            out,
            "<section><h2>{name} — {count} group(s)</h2>",
            name = escape(language_display_name(language)),
            count = clusters.len(),
        );
        for cluster in clusters {
            write_cluster_card(out, cluster, snippets);
        }
        let _ = write!(out, "</section>");
    }
}

/// Buckets clusters by their canonical occurrence's language, preserving
/// the input worst-first order within each bucket. The returned order is
/// first-seen — and because the input is globally worst-first, the
/// first language seen owns the worst cluster, so sections come out
/// ordered by worst weight desc.
fn group_clusters_by_language(
    clusters: &[ReportCluster],
) -> Vec<(&'static str, Vec<&ReportCluster>)> {
    let mut order: Vec<&'static str> = Vec::new();
    let mut buckets: HashMap<&'static str, Vec<&ReportCluster>> = HashMap::new();
    for cluster in clusters {
        let language = canonical_language(cluster);
        let bucket = buckets.entry(language).or_insert_with(|| {
            order.push(language);
            Vec::new()
        });
        bucket.push(cluster);
    }
    order
        .into_iter()
        .filter_map(|language| buckets.remove(language).map(|list| (language, list)))
        .collect()
}

/// Language id of a cluster's canonical (first) occurrence, or
/// `"unknown"` when it has none.
fn canonical_language(cluster: &ReportCluster) -> &'static str {
    cluster
        .occurrences
        .first()
        .map_or("unknown", |occurrence| language_for_path(&occurrence.path))
}

/// One-line per-language count clause for the intro lede, worst-first.
fn language_breakdown(report: &Report) -> String {
    let groups = group_clusters_by_language(&report.clusters);
    if groups.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = groups
        .iter()
        .map(|(language, clusters)| {
            format!("{} ({})", language_display_name(language), clusters.len())
        })
        .collect();
    format!("By language: {}.", parts.join(" · "))
}

/// Human display name for a language id used in section headings.
fn language_display_name(language: &str) -> &'static str {
    match language {
        "csharp" => "C#",
        "rust" => "Rust",
        "python" => "Python",
        "dart" => "Dart",
        _ => "Other",
    }
}

/// Writes a single cluster as a Terminal Card: title plus action sentence
/// plus one expanded example snippet plus a compact "also found in …" list.
/// A `data`-category cluster ([RANK-CATEGORY]) carries a "data table" chip in
/// the header and the category-specific builder/asset action sentence, both
/// sourced from [`CloneCategory`] so the HTML, text, and tree surfaces render
/// identical words.
fn write_cluster_card(out: &mut String, cluster: &ReportCluster, snippets: &mut SnippetLoader<'_>) {
    let kind = classify(cluster);
    let labels = bucket_labels(kind);
    let category = CloneCategory::from_wire_label(&cluster.category);
    let occurrences = &cluster.occurrences;
    let ai_badge = if labels.ai_match {
        "<span class=\"cluster-card__ai-badge\" title=\"Detected by the AI embedding pass — semantically equivalent, syntactically different.\">AI match</span>"
    } else {
        ""
    };
    let action = match category {
        CloneCategory::DataTable => category.action_sentence(),
        CloneCategory::Logic => kind_action(kind),
    };
    let _ = write!(
        out,
        "<article class=\"cluster-card {kind_class}\">\
         <header class=\"cluster-card__head\">\
         <h3 class=\"cluster-card__title\">{title}</h3>\
         {ai_badge}{category_chip}\
         <span class=\"cluster-card__cost\">{cost}</span>\
         </header>\
         <p class=\"cluster-card__action\">{action}</p>",
        kind_class = kind_class(kind),
        title = escape(&kind_title(kind, occurrences.len())),
        category_chip = category_chip(category),
        cost = escape(&cost_chip(cluster)),
        action = escape(action),
    );
    write_example(out, occurrences, snippets);
    write_also_list(out, occurrences, snippets);
    let _ = write!(out, "</article>");
}

/// Renders the `data table` category chip for the card header, or an empty
/// string for an ordinary logic clone. Reuses the `cluster-card__ai-badge`
/// pill style — same visual treatment, zero duplicate CSS — and sources the
/// label from [`CloneCategory::chip`] so it never drifts from the text
/// renderer ([RANK-CATEGORY]).
fn category_chip(category: CloneCategory) -> String {
    category.chip().map_or_else(String::new, |label| {
        format!(
            "<span class=\"cluster-card__ai-badge\" \
             title=\"Repeated data rows, not duplicated logic — demoted in the ranking.\">{}</span>",
            escape(label),
        )
    })
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
    let location = snippets.location(&example.path, example.start_byte);
    match snippets.snippet(&example.path, example.start_byte, example.end_byte) {
        Some((source, start_line)) => {
            let _ = write!(
                out,
                "<p class=\"cluster-card__example\">Example — {location}</p>",
                location = escape(&location),
            );
            out.push_str(&render_snippet_body(&source, start_line, language));
        }
        None => {
            let _ = write!(
                out,
                "<p class=\"cluster-card__example\">Example — {location}</p>\
                 <p class=\"snippet-missing\">Source unavailable on disk.</p>",
                location = escape(&location),
            );
        }
    }
}

/// Renders the compact "also found in …" tail.
fn write_also_list(
    out: &mut String,
    occurrences: &[ReportOccurrence],
    snippets: &mut SnippetLoader<'_>,
) {
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
        write_also_item(out, occ, snippets);
    }
    let _ = write!(out, "</ul>");
    if occurrences.len() > inline_cap {
        let extra = occurrences.len().saturating_sub(inline_cap);
        let _ = write!(
            out,
            "<details class=\"also-toggle\"><summary>Show {extra} more location(s)</summary><ul class=\"also-list\">",
        );
        for occ in occurrences.iter().skip(inline_cap) {
            write_also_item(out, occ, snippets);
        }
        let _ = write!(out, "</ul></details>");
    }
}

/// One row in the "also found in" list.
fn write_also_item(out: &mut String, occ: &ReportOccurrence, snippets: &mut SnippetLoader<'_>) {
    let class = if occ.hidden { "is-hidden" } else { "" };
    let suffix = if occ.hidden {
        " · hidden by your config"
    } else {
        ""
    };
    let location = snippets.location(&occ.path, occ.start_byte);
    let _ = write!(
        out,
        "<li class=\"{class}\">{location}<span class=\"also-loc\">{suffix}</span></li>",
        location = escape(&location),
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
