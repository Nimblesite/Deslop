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
//! with its real files, ranges and occurrence count (gh #373). An empty
//! report satisfies the absence half and fails the presence half, so a
//! detector that went blind cannot pass this test.

use crate::common::{contract_boundary::ContractBoundaryCase, Result};

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

#[test]
fn same_shaped_backends_stay_hidden_while_the_renamed_helper_surfaces() -> Result<()> {
    ContractBoundaryCase {
        fixture: FIXTURE,
        min_nodes: MIN_NODES,
        files_analysed: FILES_ANALYSED,
        contract_pair: [DOCKER_HOST, FLY_HOST],
        contract_reason: "one abstract `tool_call` contract forces both implementations \
             into the same signature and the same statement shape; the \
             collaborators they reach for — containers against machines, \
             `invoke` against `execute` — are the entire behavioural \
             difference.",
        clone_pair: [ALPHA_QUEUE, BETA_QUEUE],
        clone_lines: (CLONE_FIRST_LINE, CLONE_LAST_LINE),
        clone_subject: "the renamed `drain_queue`",
    }
    .assert()
}
