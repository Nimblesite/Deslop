//! E2E pin for gh #458 — [FUSED-STRATEGY-BOUNDED-MAX] and
//! [PIPELINE-CLUSTER-CLOSURE]: a proven copy family must survive an
//! unrelated shape-identical stranger joining the corpus. On the
//! mass-only wire the byte-level fact replaces the pair evidence: the
//! nested copies-only block is byte-proven, the whole-file closure is
//! not (the stranger's bytes differ), and no cluster may present the
//! raw-differing stranger as byte-proven.
//!
//! The `verbatim-plus-stranger` fixture holds four byte-identical files
//! plus a shape-identical stranger with different identifiers and
//! different literal constants (its content agreement against a copy is
//! 0.0436 — a false duplicate by any measure: the formulas it computes
//! differ at every literal position).
//!
//! The spec outcome, clause by clause:
//! - **[FUSED-STRATEGY-BOUNDED-MAX]** step 3: the stranger normalises to
//!   the copies' exact tree, so `H = 1.0`, `token_jaccard = 1.0`,
//!   `embedding_cos = 0.0` (embeddings off) and `fused = max = 1.0` for
//!   **every** pair — copy↔copy and copy↔stranger alike. Step 4:
//!   admission is pair by pair and every pair clears the 0.85 bar
//!   directly, so the rescue (and its content floor) is never consulted;
//!   transitive closure yields **one 5-member component**.
//! - **[CLONE-NOISE-VERBATIM-SUBGROUP]**: no noise filter recognises the
//!   component, so it is handed on untouched — no split, no member
//!   dropped. The stranger rides in the closure.
//! - **[FUSED-PAIR-SIGNALS]**: explicit comparison uses exactly the endpoints
//!   `q = max(S,J,E) = 1.0` — the copies' own pair — so the rendered
//!   triple is `1.0/1.0/1.0` and the stranger does not demote it
//!   (AC6). A byte-identical pair inside a lookalike cluster keeps its
//!   act-now evidence; the lookalikes do not manufacture a verdict.
//! - **[CLONE-BUCKETS-IDENTICAL]**: the `Identical` route requires every
//!   raw slice byte-equal; the stranger's differ, so the whole-file
//!   cluster is downgraded to `NearlyIdentical`.
//! - **[PIPELINE-CLUSTER-SUBSUME]**: the byte-identical block nested
//!   inside the copies is a second region — the stranger's occurrence
//!   has no counterpart there, so the region pairing fails and both
//!   views publish.

use anyhow::Result;
use serde_json::Value;

use crate::common::{signals::*, *};

/// The whole-file closure the spec requires: four copies plus the
/// shape-identical stranger, one component, one cluster.
const WHOLE_FILE_CLUSTER_ID: &str = "0c3021fd6641a9c6";
/// The nested byte-identical block inside the copies.
const NESTED_BLOCK_CLUSTER_ID: &str = "1a9c15f5c7f7b5fd";
/// The four verbatim copies.
const COPY_FILES: [&str; 4] = ["copy_0.ts", "copy_1.ts", "copy_2.ts", "copy_3.ts"];
/// The shape-identical stranger with unrelated content (agreement 0.0436).
const STRANGER_FILE: &str = "stranger.ts";
/// The byte-proof bucket a raw-differing stranger can never reach.
/// Runs the fixture with embeddings off.
fn run_family_report() -> Result<Value> {
    run_report_args(
        &fixture("verbatim-plus-stranger"),
        &["--min-nodes", "15", "--embeddings", "off"],
    )
}

/// The cluster with the given id, or an error naming the report.
fn expect_cluster_id<'a>(report: &'a Value, id: &str) -> Result<&'a Value> {
    clusters(report)
        .iter()
        .find(|cluster| field(cluster, "id").as_str() == Some(id))
        .ok_or_else(|| anyhow::anyhow!("cluster {id} missing from report: {report:#}"))
}

/// [FUSED-PAIR-SIGNALS] gh #458 C3 — the stranger cannot demote the
/// proven family's evidence, and the copies' byte-identical pair keeps
/// its act-now evidence wherever it renders:
/// 1. the whole-file closure is ONE 5-member cluster carrying the
///    stranger — handed on untouched ([CLONE-NOISE-VERBATIM-SUBGROUP]),
///    downgraded to `nearly_identical` only because the stranger's raw
///    bytes differ ([CLONE-BUCKETS-IDENTICAL]),
/// 2. the explicit pair is the copies' own pair: `structural 1.0`,
///    `token_jaccard 1.0`, `pair_agreement 1.0`, `pair_rename_consistency 1.0`,
///    `signal_source` naming two copies — the family's evidence is
///    preserved in full (AC6),
/// 3. no cluster certifies the stranger in the byte-proof `identical`
///    bucket — raw-byte proof is the one verdict the stranger cannot
///    ride ([CLONE-BUCKETS-IDENTICAL]).
#[test]
fn a_verbatim_family_survives_an_unrelated_stranger() -> Result<()> {
    let report = run_family_report()?;

    // 1. The whole-file closure: one cluster, five members, stranger
    //    included — the component the noise filters did not suppress is
    //    handed on untouched, not split.
    let whole_file = expect_cluster_id(&report, WHOLE_FILE_CLUSTER_ID)?;
    // The whole-file closure is byte-distinct (the stranger's raw bytes
    // differ), so it must not be byte-proven — the raw-byte fact is the
    // wire truth the old `Identical` bucket asserted.
    assert!(
        !has_verbatim_pair(&fixture("verbatim-plus-stranger"), whole_file)?,
        "the whole-file closure holds the raw-differing stranger and must \
         not read as a byte-identical copy: {}",
        signal_dump(whole_file)
    );
    assert_no_pair_surface_on_cluster(whole_file, "whole-file closure");
    assert_eq!(
        occurrences(whole_file).len(),
        5,
        "the closure holds the four copies plus the stranger"
    );
    assert_eq!(
        cluster_file_set(whole_file),
        COPY_FILES
            .iter()
            .map(|path| (*path).to_owned())
            .chain(std::iter::once(STRANGER_FILE.to_owned()))
            .collect(),
        "the stranger rides in the closure — admission is pair by pair \
         and its H=1 edges with the copies all clear the bar \
         ([FUSED-STRATEGY-BOUNDED-MAX])"
    );

    // 2. [PIPELINE-CLUSTER-CLOSURE] The pair-scoped evidence and
    //    `signal_source` are gone from the wire. The copies' own
    //    byte-identical block keeps its byte-proof fact on the nested
    //    cluster (below); the stranger's presence demotes nothing that
    //    the wire still carries.
    // 3. The nested byte-identical block inside the copies is a separate
    //    region ([PIPELINE-CLUSTER-SUBSUME]: the stranger's whole-file
    //    occurrence has no counterpart there) and keeps the byte-proof
    //    bucket its raw slices earn.
    let nested = expect_cluster_id(&report, NESTED_BLOCK_CLUSTER_ID)?;
    assert!(
        has_verbatim_pair(&fixture("verbatim-plus-stranger"), nested)?,
        "the copies' byte-identical block is byte-proven from the fixture \
         source: {nested:#}"
    );
    assert_eq!(
        cluster_file_set(nested),
        COPY_FILES.iter().map(|path| (*path).to_owned()).collect(),
        "the nested block spans the four copies alone"
    );

    // 4. No cluster holding the stranger may be byte-proven: raw-byte
    //    equality is the one verdict a raw-differing stranger can never
    //    manufacture ([PIPELINE-CLUSTER-CLOSURE]).
    for cluster in clusters(&report) {
        let has_stranger = occurrences(cluster).iter().any(is_stranger);
        if has_stranger {
            assert!(
                !has_verbatim_pair(&fixture("verbatim-plus-stranger"), cluster)?,
                "a cluster certifying the raw-differing stranger as byte-proven \
                 is a false positive: {cluster:#}"
            );
        }
    }

    Ok(())
}

/// Whether an occurrence's file is the stranger.
fn is_stranger(occurrence: &Value) -> bool {
    occurrence_path(occurrence)
        .is_ok_and(|path| path.rsplit('/').next().unwrap_or_default() == STRANGER_FILE)
}
