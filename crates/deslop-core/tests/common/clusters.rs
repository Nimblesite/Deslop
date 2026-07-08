//! Synthetic-cluster builders and cluster finders shared by the
//! refactor E2E suites — one canonical `ReportCluster` literal instead
//! of per-suite copies (this repo's own top offender, found by its own
//! detector).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

/// First cross-file `identical` cluster in the ranked report — the
/// consolidation suites' shared finder ([AUTOFIX-CONSOLIDATE-SURFACE]).
pub(crate) fn cross_file_identical_cluster(
    report: &deslop_core::report::Report,
) -> Result<deslop_core::report::ReportCluster> {
    report
        .clusters
        .iter()
        .find(|cluster| {
            cluster.bucket == "identical" && {
                let paths: std::collections::HashSet<_> = cluster
                    .occurrences
                    .iter()
                    .map(|occurrence| &occurrence.path)
                    .collect();
                paths.len() >= 2
            }
        })
        .cloned()
        .ok_or_else(|| anyhow!("a cross-file identical cluster must surface"))
}

/// A synthetic report cluster over explicit occurrences — full control
/// of the precondition-relevant fields for the refactor suites.
/// Signals are proven-Identical; the caller picks the bucket label.
pub(crate) fn synthetic_report_cluster(
    occurrences: Vec<deslop_core::report::ReportOccurrence>,
    bucket: &str,
) -> deslop_core::report::ReportCluster {
    deslop_core::report::ReportCluster {
        id: "abcdef0123456789".to_owned(),
        weight: 1.0,
        size: occurrences.len(),
        canonical_node_count: 40,
        signals: deslop_core::report::ReportSignals {
            structural: 1.0,
            token_jaccard: 1.0,
            embedding_cos: 0.0,
            fused: 1.0,
        },
        bucket: bucket.to_owned(),
        category: "logic".to_owned(),
        occurrences_total: occurrences.len(),
        occurrences,
        occurrences_truncated: false,
        summary: String::new(),
        interpretation: String::new(),
    }
}

/// One report occurrence over `[start, end)` of `file_name`.
pub(crate) fn report_occurrence(
    file_name: &str,
    span: (usize, usize),
    hidden: bool,
) -> deslop_core::report::ReportOccurrence {
    deslop_core::report::ReportOccurrence {
        path: PathBuf::from(file_name),
        start_byte: span.0,
        end_byte: span.1,
        start_line: 0,
        end_line: 0,
        hidden,
    }
}

/// The two byte spans of `needle` in `text` — the fixture-anchoring
/// primitive shared by the refactor suites.
pub(crate) fn both_spans(text: &str, needle: &str) -> Result<((usize, usize), (usize, usize))> {
    let first = text.find(needle).context("first needle")?;
    let resume = first.saturating_add(needle.len());
    let second = text
        .get(resume..)
        .and_then(|rest| rest.find(needle))
        .map(|offset| resume.saturating_add(offset))
        .context("second needle")?;
    Ok((
        (first, first.saturating_add(needle.len())),
        (second, second.saturating_add(needle.len())),
    ))
}

/// Computes the extract plan for a synthetic proven-Identical cluster
/// over the two occurrences of `needle` in `text`, parsed per
/// `file_name`'s language — the precondition-rule tests drive this end
/// to end.
pub(crate) fn needle_cluster_plan(
    text: &str,
    needle: &str,
    file_name: &str,
) -> Result<Option<deslop_core::refactor::ExtractMethodPlan>> {
    let (first, second) = both_spans(text, needle)?;
    let cluster = synthetic_report_cluster(
        vec![
            report_occurrence(file_name, first, false),
            report_occurrence(file_name, second, false),
        ],
        "identical",
    );
    let parser = deslop_core::refactor::parser_for_path(Path::new(file_name))
        .ok_or_else(|| anyhow!("no parser for {file_name}"))?;
    deslop_core::refactor::compute_plan(&cluster, text.as_bytes(), parser.as_ref())
        .map_err(|error| anyhow!("unexpected error {error}"))
}
