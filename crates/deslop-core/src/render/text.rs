//! Terse ASCII text renderer.
//!
//! AI-readable pretty-print over the canonical JSON report. No colour,
//! no Unicode box-drawing, line-oriented — consumable by any LLM (not
//! just Claude) as demonstrated in
//! `docs/specs/REPORTING-CONTEXT.md`.

use std::fmt::Write as _;

use crate::{
    report::{Report, ReportCluster},
    report_metrics::ThresholdSource,
};

/// Renders `report` as terse ASCII text suitable for piping to an agent.
#[must_use]
pub fn render_text(report: &Report) -> String {
    let mut out = String::new();
    write_header(&mut out, report);
    write_metrics(&mut out, report);
    write_provenance(&mut out, report);
    write_cache_stats(&mut out, report);
    write_action_hints(&mut out, report);
    write_boilerplate_hints(&mut out, report);
    for (idx, cluster) in report.clusters.iter().enumerate() {
        write_cluster(&mut out, idx, cluster);
    }
    out
}

/// Writes the one-line repo-wide duplication header and the active
/// threshold verdict per [METRICS-REPO]. The metric is always shown;
/// the threshold line is omitted when no threshold is configured to
/// keep local runs terse.
fn write_metrics(out: &mut String, report: &Report) {
    let metrics = &report.metrics;
    let _ = writeln!(
        out,
        "repo: {percent:.1}% duplicated ({dup} / {total} LOC, {clusters} clusters across {files} files)",
        percent = metrics.duplication_percent,
        dup = metrics.duplicated_loc,
        total = metrics.analysed_loc,
        clusters = metrics.clusters_total,
        files = metrics.duplicated_files,
    );
    let verdict = match metrics.threshold.source {
        ThresholdSource::None => return,
        ThresholdSource::Cli | ThresholdSource::Config => {
            if metrics.threshold.breached {
                "breached"
            } else {
                "ok"
            }
        }
    };
    let _ = writeln!(
        out,
        "threshold: {pct:.2}% ({verdict})",
        pct = metrics.threshold.percent,
    );
}

/// Writes the incremental-cache line so a human running back-to-back
/// analyses can see whether the second run benefited from the cache
/// ([PIPELINE-INCREMENTAL]). Omits the line when both counters are
/// zero — the `--no-incremental` path or the first-ever run.
fn write_cache_stats(out: &mut String, report: &Report) {
    let stats = report.cache_stats;
    if stats.hits == 0 && stats.misses == 0 {
        return;
    }
    let _ = writeln!(
        out,
        "cache: {hits} hit / {misses} miss",
        hits = stats.hits,
        misses = stats.misses,
    );
}

/// Writes the one-line summary header.
fn write_header(out: &mut String, report: &Report) {
    let _ = writeln!(
        out,
        "deslop {tool} -- {files} file(s), {clusters} cluster(s), {hidden} hidden",
        tool = report.tool_version,
        files = report.files_analysed,
        clusters = report.clusters.len(),
        hidden = report.clusters_hidden,
    );
}

/// Writes the embedding provenance line when the pass ran.
fn write_provenance(out: &mut String, report: &Report) {
    let Some(provenance) = report.embedding_provenance.as_ref() else {
        let _ = writeln!(out, "embeddings: off");
        return;
    };
    let _ = writeln!(
        out,
        "embeddings: {provider}/{model}@{version} ({dims}-d)",
        provider = provenance.provider_id,
        model = provenance.model_id,
        version = provenance.model_version,
        dims = provenance.dimensions,
    );
}

/// Writes the playbook header so an agent can consult the decision
/// table before walking the cluster list. `action_hints` is
/// guaranteed non-empty by the report builder, so there is no empty
/// guard.
fn write_action_hints(out: &mut String, report: &Report) {
    let _ = writeln!(out, "-- action hints --");
    for hint in &report.action_hints {
        let _ = writeln!(out, "  [{}] {}", hint.pattern, hint.recommendation);
    }
}

/// Writes import/prologue hygiene hints when report mode is enabled.
fn write_boilerplate_hints(out: &mut String, report: &Report) {
    if report.boilerplate_hints.is_empty() {
        return;
    }
    let _ = writeln!(out, "-- boilerplate hints --");
    for hint in &report.boilerplate_hints {
        let count = hint.occurrences.len();
        let _ = writeln!(
            out,
            "  [{lang}/{kind}] {rec} ({count} occurrence(s))",
            lang = hint.language,
            kind = hint.kind,
            rec = hint.recommendation,
        );
    }
}

/// Writes a single cluster block.
fn write_cluster(out: &mut String, idx: usize, cluster: &ReportCluster) {
    let _ = writeln!(
        out,
        "#{rank} [{id}] weight={weight:.2} size={size} nodes={nodes}\n  {summary}\n  :: {interpretation}",
        rank = idx.saturating_add(1),
        id = cluster.id,
        weight = cluster.weight,
        size = cluster.size,
        nodes = cluster.canonical_node_count,
        summary = cluster.summary,
        interpretation = cluster.interpretation,
    );
}
