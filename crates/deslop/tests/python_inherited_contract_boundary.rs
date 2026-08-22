//! E2E regression for [CLONE-NOISE-POLYMORPHIC-CONTRACT] — the boundary
//! of the [CLONE-NOISE-POLYMORPHIC-SIGNATURE] suppression.
//!
//! The polymorphic gate may only delete a same-named cross-file cluster
//! when a contract *forces* the signature it matched on. Reading that
//! requirement as "the enclosing type names some base" makes every
//! ordinary subclass a contract implementation, so a method copied into
//! two unrelated subclasses of one shared base is hidden the moment the
//! copies rename the collaborators they reach for — a false negative on
//! exactly the duplication a user most wants back.
//!
//! Both directions live in ONE scan so a fix for either can never trade
//! away the other. `LedgerSink` is an `ABC` that declares
//! `record_entry`, so its two implementations are genuinely forced into
//! the same signature and must stay hidden. `CommonWorker` declares only
//! `__init__` and `stamp`; `InvoiceWorker.synchronise` and
//! `UserWorker.synchronise` are a copy-paste with every local, parameter
//! and collaborator renamed, and nothing forces them to agree — that
//! clone must surface with its real files, ranges, bucket and signals.
//! An empty report satisfies the absence half and fails the presence
//! half, so a detector that went blind cannot pass this test.

use crate::common::*;

/// The fixture holding the real contract pair and the inherited-but-not-
/// declared copy.
const FIXTURE: &str = "python-inherited-contract-boundary";

/// Node floor for the scan. Low enough to admit both ten-line subjects,
/// so neither half of the test can pass by not matching.
const MIN_NODES: u32 = 12;

/// Every `.py` file in the fixture: the abstract base, its two
/// implementations, the plain base, and the two copies.
const FILES_ANALYSED: u64 = 6;

/// The `LedgerSink` implementation that writes to buckets.
const S3_SINK: &str = "s3_sink.py";

/// The `LedgerSink` implementation that writes to blobs.
const GCS_SINK: &str = "gcs_sink.py";

/// The copy's canonical half, a `CommonWorker` subclass.
const INVOICE_WORKER: &str = "invoice_worker.py";

/// The copy, with every local, parameter and collaborator renamed.
const USER_WORKER: &str = "user_worker.py";

/// The bucket a total consistent rename lands in.
const NEARLY_IDENTICAL: &str = "nearly_identical";

/// First line of the matched view in both copies — the whole module,
/// because the import and the class shell around `synchronise` are
/// identical too.
const CLONE_FIRST_LINE: u64 = 1;

/// Last line of `synchronise` in both copies.
const CLONE_LAST_LINE: u64 = 14;

/// One occurrence per file.
const CLONE_OCCURRENCES: u64 = 2;

/// A consistent rename must at least reach the read-the-canonical-
/// occurrence band.
const RENAME_FUSED_FLOOR: f64 = 0.6;

#[test]
fn an_inherited_method_no_base_declares_is_not_a_contract_implementation() -> Result<()> {
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
        cluster_spanning(&report, &[S3_SINK, GCS_SINK]).is_none(),
        "`LedgerSink` declares the abstract `record_entry` both sinks \
         override, so the contract is what forces their signature and \
         statement shape to agree; buckets against blobs, `serialise` \
         against `encode` are the entire behavioural difference. A \
         cluster pairing them reports the contract as duplication: \
         {visible:#?}"
    );
    assert!(
        clusters_hidden(&report) >= 1,
        "the contract pair must be actively suppressed, not merely \
         absent from a report that found nothing: {report:#}"
    );

    let clone = expect_cluster_spanning(&report, &[INVOICE_WORKER, USER_WORKER])?;
    assert_eq!(
        cluster_count(&report),
        1,
        "the copied `synchronise` pair is the only duplication in this \
         fixture: {visible:#?}"
    );
    assert_eq!(
        cluster_bucket(clone),
        NEARLY_IDENTICAL,
        "a total consistent rename is the definition of \
         nearly-identical: {report:#}"
    );
    assert_eq!(
        cluster_size(clone),
        CLONE_OCCURRENCES,
        "one occurrence per file: {report:#}"
    );
    assert!(
        approx(signal(clone, "structural"), 1.0),
        "identifier renames are invisible to the normalised tree: \
         {report:#}"
    );
    assert!(
        approx(signal(clone, "token_jaccard"), 1.0),
        "the token layer is rename-invariant by design: {report:#}"
    );
    assert!(
        approx(signal(clone, "rename_consistency"), 1.0),
        "every identifier is renamed the same way in every occurrence, \
         which is what makes this a copy rather than an implementation: \
         {report:#}"
    );
    assert!(
        signal(clone, "fused") >= RENAME_FUSED_FLOOR,
        "`CommonWorker` never declares `synchronise`, so nothing forces \
         these two bodies to agree and the copy must keep its rank: \
         {report:#}"
    );
    for occurrence in occurrences(clone) {
        assert_eq!(
            field(occurrence, "start_line").as_u64(),
            Some(CLONE_FIRST_LINE),
            "the clone covers the module from its import down in both \
             files: \
             {visible:#?}"
        );
        assert_eq!(
            field(occurrence, "end_line").as_u64(),
            Some(CLONE_LAST_LINE),
            "the clone covers the whole method in both files: \
             {visible:#?}"
        );
    }
    assert!(
        visible_duplicated_loc(&report) > 0,
        "two rename-identical methods duplicate real lines: {report:#}"
    );
    assert!(
        metric_field(&report, "duplication_percent")
            .as_f64()
            .unwrap_or(0.0)
            > 0.0,
        "the headline figure must count the surviving copy: {report:#}"
    );
    Ok(())
}
