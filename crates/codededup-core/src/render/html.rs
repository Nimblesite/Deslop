//! Single-file HTML renderer.
//!
//! Human-readable view of the canonical report. Inline CSS, no JS, no
//! external fonts — a report opened from disk on a fresh machine works
//! offline. Includes the `schema_doc` and `action_hints` verbatim so a
//! human reader understands what the signals mean without a side-channel
//! document.

use std::fmt::Write as _;

use crate::report::{Report, ReportCluster, ReportOccurrence};

/// Maximum number of occurrences rendered expanded before the rest fall
/// into a collapsed `<details>` list.
const EXPANDED_OCCURRENCES: usize = 8;

/// Renders `report` as a single-file HTML document.
#[must_use]
pub fn render_html(report: &Report) -> String {
    let mut out = String::new();
    let _ = write!(out, "<!doctype html><html lang=\"en\"><head>");
    write_head(&mut out, report);
    let _ = write!(out, "</head><body>");
    write_header(&mut out, report);
    write_action_hints(&mut out, report);
    write_schema_doc(&mut out, report);
    write_clusters(&mut out, report);
    let _ = write!(out, "</body></html>");
    out
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
/// clusters and can apply it as a decision table. `action_hints` is
/// guaranteed non-empty by the report builder.
fn write_action_hints(out: &mut String, report: &Report) {
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
fn write_clusters(out: &mut String, report: &Report) {
    let _ = write!(out, "<section class=\"clusters\"><h2>Clusters</h2>");
    if report.clusters.is_empty() {
        let _ = write!(out, "<p class=\"empty\">No duplication detected.</p>");
    }
    for (idx, cluster) in report.clusters.iter().enumerate() {
        write_cluster(out, idx, cluster);
    }
    let _ = write!(out, "</section>");
}

/// Writes a single cluster block with its occurrences.
fn write_cluster(out: &mut String, idx: usize, cluster: &ReportCluster) {
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
    write_occurrences(out, &cluster.occurrences);
    let _ = write!(out, "</article>");
}

/// Splits the occurrence list into an expanded head and a collapsed
/// `<details>` tail so huge clusters don't blow up the page.
fn write_occurrences(out: &mut String, occurrences: &[ReportOccurrence]) {
    let head_len = occurrences.len().min(EXPANDED_OCCURRENCES);
    let (head, tail) = occurrences.split_at(head_len);
    let _ = write!(out, "<ul class=\"occurrences\">");
    for occ in head {
        write_occurrence_li(out, occ);
    }
    let _ = write!(out, "</ul>");
    if !tail.is_empty() {
        let _ = write!(
            out,
            "<details><summary>{remaining} more occurrence(s)</summary><ul class=\"occurrences\">",
            remaining = tail.len(),
        );
        for occ in tail {
            write_occurrence_li(out, occ);
        }
        let _ = write!(out, "</ul></details>");
    }
}

/// Writes one `<li>` for a single occurrence.
fn write_occurrence_li(out: &mut String, occ: &ReportOccurrence) {
    let class = if occ.hidden { " class=\"hidden\"" } else { "" };
    let _ = write!(
        out,
        "<li{class}><code>{path}</code>:{start}-{end}{marker}</li>",
        path = escape(&occ.path.display().to_string()),
        start = occ.start_byte,
        end = occ.end_byte,
        marker = if occ.hidden { " · hidden" } else { "" },
    );
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
