//! [CLONE-NOISE-POLYMORPHIC-SIGNATURE] — one scan carrying both halves
//! of gh #69.
//!
//! An abstract base forces two concrete implementations into the same
//! signature and the same statement shape, so the contract is what makes
//! them look alike and a cluster pairing them reports the contract as
//! duplication. That pair must stay hidden. In the *same* scan, a
//! consistently renamed clone must still surface with its real files,
//! extent and occurrence count, so a fix for either half can never trade
//! away the other. An empty report satisfies the absence half and fails
//! the presence half, so a detector that went blind cannot pass.

use super::{
    cluster_count, cluster_size, cluster_spanning, clusters_hidden, expect_cluster_spanning, field,
    fixture, metric_field, occurrences, run_report,
    signals::{
        assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
    },
    visible_cluster_lines, visible_duplicated_loc, Result,
};

/// One occurrence per file — the shape both halves of a clone pair have.
const CLONE_OCCURRENCES: u64 = 2;

/// One contract-boundary scan and everything it must prove.
pub(crate) struct ContractBoundaryCase<'a> {
    /// Fixture directory holding the contract pair and the clone pair.
    pub(crate) fixture: &'a str,
    /// Node floor low enough to admit both subjects, so neither half can
    /// pass by not matching.
    pub(crate) min_nodes: u32,
    /// Every source file the scan must parse.
    pub(crate) files_analysed: u64,
    /// The two implementations one contract forces to agree. They must
    /// never share a cluster.
    pub(crate) contract_pair: [&'a str; 2],
    /// Why those two look alike — the behavioural difference a reader
    /// needs in the failure message.
    pub(crate) contract_reason: &'a str,
    /// The renamed clone that must surface despite the suppression.
    pub(crate) clone_pair: [&'a str; 2],
    /// The clone's first and last line, identical in both files.
    pub(crate) clone_lines: (u64, u64),
    /// What the clone is, for assertion messages.
    pub(crate) clone_subject: &'a str,
}

impl ContractBoundaryCase<'_> {
    /// Runs the scan and asserts both halves.
    pub(crate) fn assert(&self) -> Result<()> {
        let scan_root = fixture(self.fixture);
        let report = run_report(&scan_root, self.min_nodes)?;
        self.assert_contract_pair_hidden(&report);
        self.assert_clone_surfaces(&scan_root, &report)?;
        Ok(())
    }

    /// The absence half: the contract pair is actively suppressed, and
    /// the scan really did parse every file rather than measuring
    /// nothing.
    fn assert_contract_pair_hidden(&self, report: &serde_json::Value) {
        let visible = visible_cluster_lines(report);
        assert_eq!(
            field(report, "files_analysed").as_u64(),
            Some(self.files_analysed),
            "{}: every fixture file must be parsed — a scan that skipped them \
             would satisfy the absence half by measuring nothing: {report:#}",
            self.fixture,
        );
        assert!(
            cluster_spanning(report, &self.contract_pair).is_none(),
            "{reason} A cluster pairing them reports the contract as \
             duplication: {visible:#?}",
            reason = self.contract_reason,
        );
        assert!(
            clusters_hidden(report) >= 1,
            "{}: the contract pair must be actively suppressed, not merely \
             absent from a report that found nothing: {report:#}",
            self.fixture,
        );
    }

    /// The presence half: the renamed clone is the scan's only
    /// duplication, and it is admitted, mass-honest, clean-surfaced and
    /// byte-distinct ([PIPELINE-CLUSTER-CLOSURE]) across its real extent.
    fn assert_clone_surfaces(
        &self,
        scan_root: &std::path::Path,
        report: &serde_json::Value,
    ) -> Result<()> {
        let visible = visible_cluster_lines(report);
        let clone = expect_cluster_spanning(report, &self.clone_pair)?;
        assert_eq!(
            cluster_count(report),
            1,
            "the {subject} pair is the only duplication in this fixture: {visible:#?}",
            subject = self.clone_subject,
        );
        assert_eq!(
            cluster_size(clone),
            CLONE_OCCURRENCES,
            "one occurrence per file: {report:#}"
        );
        assert_structural_only_contract(clone, self.clone_subject);
        assert_no_pair_surface_on_cluster(clone, self.clone_subject);
        assert!(
            !has_verbatim_pair(scan_root, clone)?,
            "{subject} is a rename and must slice to differing bytes: {report:#}",
            subject = self.clone_subject,
        );
        self.assert_clone_extent(clone, &visible);
        Self::assert_clone_counts_toward_the_headline(report);
        Ok(())
    }

    /// Every occurrence covers the same lines in both files.
    fn assert_clone_extent(&self, clone: &serde_json::Value, visible: &[String]) {
        let (first, last) = self.clone_lines;
        for occurrence in occurrences(clone) {
            assert_eq!(
                field(occurrence, "start_line").as_u64(),
                Some(first),
                "the clone starts at the same line in both files: {visible:#?}"
            );
            assert_eq!(
                field(occurrence, "end_line").as_u64(),
                Some(last),
                "the clone covers the whole subject in both files: {visible:#?}"
            );
        }
    }

    /// The surviving copy reaches the headline figures.
    fn assert_clone_counts_toward_the_headline(report: &serde_json::Value) {
        assert!(
            visible_duplicated_loc(report) > 0,
            "two rename-identical subjects duplicate real lines: {report:#}"
        );
        assert!(
            metric_field(report, "duplication_percent")
                .as_f64()
                .unwrap_or(0.0)
                > 0.0,
            "the headline figure must count the surviving copy: {report:#}"
        );
    }
}
