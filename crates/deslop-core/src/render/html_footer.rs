//! "Run details" footer for the HTML report.
//!
//! Per [OUTPUT-HUMAN-HTML] the body of the report stays scannable —
//! the verbose run metadata (tool version, embedding
//! provenance and schema documentation lives inside one collapsed `<details>` tucked at
//! the bottom of the page. Humans can ignore it; agents and
//! `--from-report` consumers can still read everything they need.

use std::fmt::Write as _;

use crate::report::Report;

/// Writes the collapsed footer holding all the run metadata.
pub fn write_run_details(out: &mut String, report: &Report, escape: fn(&str) -> String) {
    let _ = write!(
        out,
        "<details class=\"run-details\"><summary>Run details and schema reference</summary>\
         <dl>\
         <dt>Tool</dt><dd>{tool}</dd>\
         <dt>Files analysed</dt><dd>{files}</dd>\
         <dt>Visible groups</dt><dd>{visible}</dd>\
         <dt>Hidden groups</dt><dd>{hidden}</dd>\
         <dt>Embeddings</dt><dd>{embeddings}</dd>\
         </dl>",
        tool = escape(&report.tool_version),
        files = report.files_analysed,
        visible = report.clusters.len(),
        hidden = report.clusters_hidden,
        embeddings = escape(&format_provenance(report)),
    );
    write_boilerplate_hints(out, report, escape);
    write_schema(out, report, escape);
    let _ = write!(out, "</details>");
}

/// Low-noise import/prologue hygiene hints.
fn write_boilerplate_hints(out: &mut String, report: &Report, escape: fn(&str) -> String) {
    if report.boilerplate_hints.is_empty() {
        return;
    }
    let _ = write!(out, "<h3>Boilerplate hints</h3><ul>");
    for hint in &report.boilerplate_hints {
        let _ = write!(
            out,
            "<li><code>{lang}/{kind}</code> - {rec} ({count} occurrence(s))</li>",
            lang = escape(&hint.language),
            kind = escape(&hint.kind),
            rec = escape(&hint.recommendation),
            count = hint.occurrences.len(),
        );
    }
    let _ = write!(out, "</ul>");
}

/// Embedded schema-doc markdown so agents can self-document the JSON
/// schema without a second round-trip. Skipped when the report omits the
/// `schema_doc` (the CLI drops it — it is served on demand), so a
/// human HTML report never carries an empty schema section.
fn write_schema(out: &mut String, report: &Report, escape: fn(&str) -> String) {
    if report.schema_doc.is_empty() {
        return;
    }
    let _ = write!(
        out,
        "<h3>Schema reference</h3><pre>{doc}</pre>",
        doc = escape(&report.schema_doc),
    );
}

/// Returns the human-readable embedding provenance line.
fn format_provenance(report: &Report) -> String {
    report.embedding_provenance.as_ref().map_or_else(
        || "embeddings: off".to_owned(),
        |provenance| {
            format!(
                "embeddings: {provider}/{model}@{version} ({dims}-d, embedded {succeeded}/{attempted} \
                 subtrees via {indexed} index points, failures {failed})",
                provider = provenance.provider_id,
                model = provenance.model_id,
                version = provenance.model_version,
                dims = provenance.dimensions,
                succeeded = provenance.succeeded_subtrees,
                indexed = provenance.indexed_subtrees,
                failed = provenance.failed_subtrees,
                attempted = provenance.attempted_subtrees,
            )
        },
    )
}
