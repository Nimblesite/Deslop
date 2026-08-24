//! Restamps the derived fields a report carries but does not measure
//! ([SEVERITY-BAND], [FUSION-CONTENT-GATE], [METRICS-REPO]).
//!
//! Rank, severity band, shape reading, occurrence count, fused-gate
//! verdict and the evidence sentence are all *computed* values. The
//! accuracy contract puts every such computation in the engine and
//! carries the result on the wire, so no consumer re-derives a figure
//! the report already states. That obligation has two entry points and
//! this module owns both: the render path stamps a freshly built
//! cluster, and `--from-report` restamps a report read back from disk,
//! which may predate any of these fields.
//!
//! Recomputation is idempotent — every field here is a pure function of
//! data the cluster already carries — so a fresh report round-trips
//! through it unchanged.

use crate::{
    pipeline::language_for_path,
    report::{occurrence_count, Report, ReportCluster},
};

/// Restamps every derived field on `report`, then renumbers the ranks
/// over the clusters it actually carries.
///
/// A canonical report is already stored worst-first, so restamping
/// reproduces the ranks the run that wrote it published; a report
/// written before the fields existed gains them instead of rendering
/// zeroes.
pub fn restamp_derived_fields(report: &mut Report) {
    for cluster in &mut report.clusters {
        restamp_cluster(cluster);
    }
    crate::report_weight::stamp_ranks(&mut report.clusters);
}

/// Restamps one cluster's derived fields.
///
/// `language` is filled only when absent: the render path resolves it
/// from the parse pass's own file-language map, which is better
/// evidence than a path extension, and a replayed report has only the
/// path to go on.
pub(crate) fn restamp_cluster(cluster: &mut ReportCluster) {
    cluster.signals.shape = cluster.signals.shape_score();
    cluster.occurrence_count = occurrence_count(cluster);
    cluster.meets_fused_gate = cluster.signals.fused >= crate::pair::FUSED_THRESHOLD;
    cluster.evidence_verdict = crate::render::signals::content_evidence_verdict(cluster.signals);
    if cluster.language.is_empty() {
        cluster.language = cluster
            .occurrences
            .first()
            .map_or("unknown", |occurrence| language_for_path(&occurrence.path))
            .to_owned();
    }
}
