//! Terse ASCII text renderer.
//!
//! AI-readable pretty-print over the canonical JSON report. No colour,
//! no Unicode box-drawing, line-oriented — consumable by any LLM (not
//! just Claude) as demonstrated in
//! `docs/specs/REPORTING-CONTEXT.md`.

use std::fmt::Write as _;

use crate::{
    clone_category::CloneCategory,
    report::{Report, ReportCluster},
    report_location::diff_badge,
    report_metrics::ThresholdSource,
};

/// Renders `report` as terse ASCII text suitable for piping to an agent.
#[must_use]
pub fn render_text(report: &Report) -> String {
    let mut out = String::new();
    write_header(&mut out, report);
    write_metrics(&mut out, report);
    write_diff_metrics(&mut out, report);
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
    // The repo line is repo-scoped: under `--only-changed`,
    // `clusters_total` follows the filtered body ([METRICS-REPO]), so
    // the repo-wide count is body + omitted ([METRICS-DIFF-SCOPE]).
    let repo_clusters = metrics
        .clusters_total
        .saturating_add(report.clusters_outside_diff.unwrap_or(0));
    let _ = writeln!(
        out,
        "repo: {percent:.1}% duplicated ({dup} / {total} LOC, {clusters} clusters across {files} files)",
        percent = metrics.duplication_percent,
        dup = metrics.duplicated_loc,
        total = metrics.analysed_loc,
        clusters = repo_clusters,
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

/// Writes the diff-scoped metrics block ([METRICS-DIFF-SCOPE]):
/// added-line duplication, the diff gate verdict when one governs, and
/// the newly-introduced delta when `--only-changed` filtered the list.
/// Absent entirely on a run without `--diff`, so no-diff output stays
/// byte-identical.
fn write_diff_metrics(out: &mut String, report: &Report) {
    let Some(diff) = report.metrics.diff.as_ref() else {
        return;
    };
    let _ = writeln!(
        out,
        "diff: {percent:.1}% of added lines duplicated ({dup} / {added} added LOC)",
        percent = diff.duplication_percent,
        dup = diff.duplicated_added_loc,
        added = diff.added_loc,
    );
    if !matches!(diff.threshold.source, ThresholdSource::None) {
        let verdict = if diff.threshold.breached {
            "breached"
        } else {
            "ok"
        };
        let _ = writeln!(
            out,
            "diff threshold: {pct:.2}% ({verdict})",
            pct = diff.threshold.percent,
        );
    }
    write_diff_delta(out, report);
}

/// Writes the `--only-changed` delta line ([CLI-ARG-ONLY-CHANGED],
/// [METRICS-DIFF-SCOPE]): every surviving cluster intersects the diff
/// by construction, split into newly introduced and cross-file with
/// untouched code (#364's requested classification), with the omitted
/// count beside them so all four figures reconcile.
fn write_diff_delta(out: &mut String, report: &Report) {
    let Some(outside) = report.clusters_outside_diff else {
        return;
    };
    let newly = report
        .clusters
        .iter()
        .filter(|cluster| cluster.is_newly_introduced == Some(true))
        .count();
    let cross_file = report.clusters.len().saturating_sub(newly);
    let _ = writeln!(
        out,
        "delta: {touched} cluster(s) intersect the diff — {newly} newly introduced, {cross_file} cross-file with untouched code; {outside} untouched cluster(s) omitted",
        touched = report.clusters.len(),
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

/// Writes a single cluster block. A `data`-category cluster carries a
/// `[data table]` chip on the header line and the category-specific action
/// sentence (builder / asset hint) on the interpretation line, both sourced
/// from [`CloneCategory`] so every surface renders the same words
/// ([RANK-CATEGORY]).
fn write_cluster(out: &mut String, idx: usize, cluster: &ReportCluster) {
    let category = CloneCategory::from_wire_label(&cluster.category);
    let chip = category
        .chip()
        .map_or(String::new(), |label| format!(" [{label}]"));
    let interpretation = match category {
        CloneCategory::DataTable => category.action_sentence(),
        CloneCategory::Logic => cluster.interpretation.as_str(),
    };
    let _ = writeln!(
        out,
        "#{rank} [{id}]{chip} weight={weight:.2} size={size} nodes={nodes}\n  {summary}\n  :: {interpretation}\n  content evidence: {evidence}",
        rank = idx.saturating_add(1),
        id = cluster.id,
        weight = cluster.weight,
        size = cluster.size,
        nodes = cluster.canonical_node_count,
        summary = cluster.summary,
        evidence = crate::render::signals::evidence_summary(cluster.signals),
    );
    write_cluster_occurrences(out, cluster);
}

/// Writes one badged row per occurrence on a `--diff` run, through the
/// shared [`diff_badge`] source ([OUTPUT-SCHEMA-DIFF-TAGS]). Silent on
/// a run without `--diff` — the cluster block stays byte-identical.
fn write_cluster_occurrences(out: &mut String, cluster: &ReportCluster) {
    for occurrence in &cluster.occurrences {
        let Some(badge) = diff_badge(occurrence.in_diff) else {
            continue;
        };
        let _ = writeln!(
            out,
            "  - {path}:{start}-{end} {badge}",
            path = occurrence.path.display(),
            start = occurrence.start_line,
            end = occurrence.end_line,
        );
    }
}
