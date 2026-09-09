//! E2E regression for [CLONE-NOISE-POLYMORPHIC-SIGNATURE] — the
//! inherited-method half of gh #69.
//!
//! `LedgerSink` declares the abstract `record_entry` both sinks
//! override, so the contract is what forces their signature and
//! statement shape to agree; buckets against blobs, `serialise` against
//! `encode` are the entire behavioural difference. Nothing about an S3
//! sink can be refactored into a GCS sink.
//!
//! Both directions live in ONE scan so a fix for either can never trade
//! away the other: the contract pair must stay hidden while a copied
//! `synchronise` must surface with its real files, ranges and
//! occurrence count. An empty report satisfies the absence half and
//! fails the presence half, so a detector that went blind cannot pass.

use crate::common::contract_boundary::ContractBoundaryCase;
use crate::common::Result;

/// The fixture holding the contract pair and the copied pair.
const FIXTURE: &str = "python-inherited-contract-boundary";

/// Node floor for the scan, low enough to admit both subjects.
const MIN_NODES: u32 = 8;

/// Every `.py` file in the fixture.
const FILES_ANALYSED: u64 = 5;

/// The contract implementation that writes to S3 buckets.
const S3_SINK: &str = "s3_sink.py";

/// The contract implementation that writes to GCS blobs.
const GCS_SINK: &str = "gcs_sink.py";

/// The copied clone's canonical half.
const INVOICE_WORKER: &str = "invoice_worker.py";

/// The copied clone's other half.
const USER_WORKER: &str = "user_worker.py";

/// First line of the clone in both files.
const CLONE_FIRST_LINE: u64 = 1;

/// Last line of the clone in both files.
const CLONE_LAST_LINE: u64 = 14;

#[test]
fn an_inherited_method_no_base_declares_is_not_a_contract_implementation() -> Result<()> {
    ContractBoundaryCase {
        fixture: FIXTURE,
        min_nodes: MIN_NODES,
        files_analysed: FILES_ANALYSED,
        contract_pair: [S3_SINK, GCS_SINK],
        contract_reason: "`LedgerSink` declares the abstract `record_entry` both sinks \
             override, so the contract is what forces their signature and \
             statement shape to agree; buckets against blobs, `serialise` \
             against `encode` are the entire behavioural difference.",
        clone_pair: [INVOICE_WORKER, USER_WORKER],
        clone_lines: (CLONE_FIRST_LINE, CLONE_LAST_LINE),
        clone_subject: "the copied `synchronise`",
    }
    .assert()
}
