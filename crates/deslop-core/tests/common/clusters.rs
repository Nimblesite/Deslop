//! Synthetic-cluster builders and cluster finders shared by the
//! refactor E2E suites — one canonical `ReportCluster` literal instead
//! of per-suite copies (this repo's own top offender, found by its own
//! detector).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, ensure, Context, Result};
use deslop_test_support::enclosure::{strictly_encloses, Span};

/// The enclosure spans of one rendered cluster.
fn spans(cluster: &deslop_core::report::ReportCluster) -> Result<Vec<Span>> {
    cluster
        .occurrences
        .iter()
        .map(|occurrence| {
            Ok(Span::new(
                occurrence.path.to_string_lossy(),
                u64::try_from(occurrence.start_byte)?,
                u64::try_from(occurrence.end_byte)?,
            ))
        })
        .collect()
}

/// [PIPELINE-CLUSTER-EXACT]: asserts the refactor suites planned from the
/// *enclosing* view of the duplication.
///
/// This is what makes a refactor golden provably correct rather than
/// merely code-agreeing. One physical duplication yields two candidate
/// clusters — the whole duplicated body, and the per-statement clones
/// nested inside it — and the nested view ranks heavier because it
/// carries one occurrence per statement. Planning from it would extract
/// a single statement, leave the rest duplicated, and stamp the *nested*
/// cluster's id into the emitted helper name that every golden pins. A
/// golden blessed in that state records the wrong region under a name
/// that looks equally plausible, so no byte comparison can catch it.
pub(crate) fn assert_planned_from_enclosing_view(
    report: &deslop_core::report::Report,
    chosen: &deslop_core::report::ReportCluster,
) -> Result<()> {
    let planned = spans(chosen)?;
    ensure!(
        !planned.is_empty(),
        "the planned cluster must render occurrences"
    );
    for other in &report.clusters {
        ensure!(
            other.id == chosen.id || !strictly_encloses(&spans(other)?, &planned),
            "cluster {chosen} is a nested view of {other}; the refactor must \
             plan from the enclosing duplication, and the golden name embeds \
             the id of whichever cluster it planned from",
            chosen = chosen.id,
            other = other.id,
        );
    }
    Ok(())
}

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
///
/// The content-evidence fields carry `ContentEvidence::unmeasured()`
/// — full pooled agreement, no rename proof, no literal dominance —
/// because no measurement pass ever ran over a hand-built cluster, and
/// [FUSED-CONTENT-GATE]'s contract is that a missing measurement never
/// demotes. Zeroes here would read as "measured, and found nothing" and
/// would make every refusal in `refactor_extract_negative.rs` pass
/// through the content gate instead of through the rule each case was
/// written to pin ([AUTOFIX-EXTRACT-PRECONDITIONS], gh #344).
pub(crate) fn synthetic_report_cluster(
    occurrences: Vec<deslop_core::report::ReportOccurrence>,
    bucket: &str,
) -> deslop_core::report::ReportCluster {
    let mut cluster =
        deslop_core::report_fixtures::fixture_cluster("abcdef0123456789", occurrences);
    cluster.canonical_node_count = 40;
    bucket.clone_into(&mut cluster.bucket);
    // No parse pass ran over a hand-built cluster, so its language is
    // the engine's own unresolvable label rather than a re-derivation
    // from the fixture's file names.
    "unknown".clone_into(&mut cluster.language);
    cluster
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
        in_diff: None,
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
