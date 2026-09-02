//! E2E regression for [CLONE-NOISE-POLYMORPHIC-SIGNATURE] — the
//! same-shaped half of gh #69.
//!
//! `python-issue-69-abstract-method` proves the filter on backends whose
//! bodies differ in node-kind shape. Deciding "different implementation"
//! on a kind stream alone left the other half unguarded: two concrete
//! implementations of one abstract `tool_call` that reach for *different
//! collaborators* through the *same* shape —
//! `self.containers[instance] … container.invoke(…)` against
//! `self.machines[instance] … machine.execute(…)` — linearise to one
//! identical stream. The gate read that as "same implementation", did
//! not suppress, and the pair surfaced as `nearly_identical` at 66%
//! duplicated with `rename_consistency = 1.0`. Nothing about a
//! container backend can be refactored into a machine backend; the
//! contract is what forces the two to look alike.
//!
//! Both directions live in ONE scan so a fix for either can never trade
//! away the other: the contract pair must stay hidden while a
//! consistently renamed `drain_queue` — same function name, same
//! members, locals and parameters renamed throughout — must surface
//! with its real files, ranges, bucket and signals (gh #373). An empty
//! report satisfies the absence half and fails the presence half, so a
//! detector that went blind cannot pass this test.

use crate::common::signals::{
    assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
};
use crate::common::*;

/// The fixture holding the contract pair and the rename clone.
const FIXTURE: &str = "python-same-shape-backends";

/// Node floor for the scan. Low enough to admit both twelve-line
/// subjects, so neither half of the test can pass by not matching, and
/// above the nine-node single call `pending.append(job.identifier)`
/// that `drain_queue` legitimately repeats inside itself — a
/// byte-identical statement the gate admits at any floor it reaches.
const MIN_NODES: u32 = 10;

/// Every `.py` file in the fixture: the abstract base, the two
/// implementations, and the two halves of the rename clone.
const FILES_ANALYSED: u64 = 5;

/// The contract implementation that drives containers.
const DOCKER_HOST: &str = "docker_host.py";

/// The contract implementation that drives machines.
const FLY_HOST: &str = "fly_host.py";

/// The rename clone's canonical half.
const ALPHA_QUEUE: &str = "alpha_queue.py";

/// The rename clone's copy, with every local and parameter renamed.
const BETA_QUEUE: &str = "beta_queue.py";

/// First line of `drain_queue` in both halves of the clone.
const CLONE_FIRST_LINE: u64 = 1;

/// Last line of `drain_queue` in both halves of the clone.
const CLONE_LAST_LINE: u64 = 13;

/// One occurrence per file.
const CLONE_OCCURRENCES: u64 = 2;

#[test]
fn same_shaped_backends_stay_hidden_while_the_renamed_helper_surfaces() -> Result<()> {
    let scan_root = fixture(FIXTURE);
    let report = run_report(&scan_root, MIN_NODES)?;
    let visible = visible_cluster_lines(&report);

    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FILES_ANALYSED),
        "every fixture file must be parsed — a scan that skipped them \
         would satisfy the absence half of this test by measuring \
         nothing: {report:#}"
    );
    assert!(
        cluster_spanning(&report, &[DOCKER_HOST, FLY_HOST]).is_none(),
        "one abstract `tool_call` contract forces both implementations \
         into the same signature and the same statement shape; the \
         collaborators they reach for — containers against machines, \
         `invoke` against `execute` — are the entire behavioural \
         difference. A cluster pairing them reports the contract as \
         duplication: {visible:#?}"
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the contract pair must be actively suppressed, not merely \
         absent from a report that found nothing: {report:#}"
    );

    let clone = expect_cluster_spanning(&report, &[ALPHA_QUEUE, BETA_QUEUE])?;
    assert_eq!(
        cluster_count(&report),
        1,
        "the renamed `drain_queue` pair is the only duplication in this \
         fixture: {visible:#?}"
    );
    assert_eq!(
        cluster_size(clone),
        CLONE_OCCURRENCES,
        "one occurrence per file: {report:#}"
    );
    // [PIPELINE-CLUSTER-CLOSURE] The nearly-identical verdict and the
    // content axes are pair-scoped now. The wire facts that hold the
    // acceptance: the renamed pair is admitted, mass-honest,
    // clean-surfaced and byte-distinct.
    assert_structural_only_contract(clone, "same-shape backends");
    assert_no_pair_surface_on_cluster(clone, "same-shape backends");
    assert!(
        !has_verbatim_pair(&scan_root, clone)?,
        "`drain_queue` is a rename across the two backends and must slice to \
         differing bytes: {report:#}"
    );
    for occurrence in occurrences(clone) {
        assert_eq!(
            field(occurrence, "start_line").as_u64(),
            Some(CLONE_FIRST_LINE),
            "the clone begins at `def drain_queue` in both files: \
             {visible:#?}"
        );
        assert_eq!(
            field(occurrence, "end_line").as_u64(),
            Some(CLONE_LAST_LINE),
            "the clone covers the whole function in both files: \
             {visible:#?}"
        );
    }
    assert!(
        visible_duplicated_loc(&report) > 0,
        "two rename-identical functions duplicate real lines: {report:#}"
    );
    assert!(
        metric_field(&report, "duplication_percent")
            .as_f64()
            .unwrap_or(0.0)
            > 0.0,
        "the headline figure must count the surviving clone: {report:#}"
    );
    Ok(())
}
