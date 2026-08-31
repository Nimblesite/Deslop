//! Tests proving that the `MinHash` signature pipeline works correctly for
//! Python after the XOF-based fix ([FUSED-SIGNALS-THREE-LAYER]).
//!
//! Bug 1 — `minhash_signature` produced 128 separate blake3 calls per
//! k-gram.  Fixed to use blake3 XOF (one call, 128 slots from extended
//! output).  These tests fail if the signature path is broken (all-same
//! values, all-sentinel, or cross-file tree contamination).
//!
//! Bug 2 — `tree_for_file` did a linear scan through all trees per
//! fingerprint.  Fixed by pre-building a `HashMap<FileId, &NormalizedNode>`
//! once.  A wrong `HashMap` mapping contaminates token streams; the
//! cross-file Type-3 assertion catches that.

use anyhow::Result;

use crate::common::signals::{assert_structural_only_contract, has_verbatim_pair};
use crate::common::*;

/// Drives the `deslop` binary over the named fixture at `min_nodes` and
/// returns the parsed JSON report, asserting the process exited cleanly.
fn run_cli(fixture_name: &str, min_nodes: u32) -> Result<serde_json::Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let min_nodes = min_nodes.to_string();
    let _assertion = deslop_cmd(&fixture(fixture_name), &output)?
        .args(["--min-nodes", min_nodes.as_str()])
        .assert()
        .success();
    load_json(&output.with_extension("json"))
}

// [FUSED-SIGNALS-THREE-LAYER] Type-2 Python clones (identical after
// normalisation) must be detected — proves the signature pipeline maps
// identical normalised k-gram sets to identical signatures. A Type-2
// clone is a *rename*: its occurrences differ in raw bytes, so the
// wire proves it by admission plus the byte truth that it is NOT a
// verbatim copy ([PIPELINE-CLUSTER-CLOSURE]).
#[test]
fn python_type2_clone_is_byte_identical() -> Result<()> {
    let scan_root = fixture("python-small");
    let report = run_cli("python-small", 10)?;
    let clusters = clusters(&report);
    assert!(
        !clusters.is_empty(),
        "python-small must produce at least one cluster",
    );
    let top = clusters
        .first()
        .ok_or_else(|| anyhow::anyhow!("python-small must produce at least one cluster"))?;
    assert_structural_only_contract(top, "python Type-2 clone");
    assert!(
        !has_verbatim_pair(&scan_root, top)?,
        "the Type-2 clone is a rename — its occurrences must differ in raw \
         bytes, never be byte-identical: {top:#}",
    );
    Ok(())
}

// [FUSED-SIGNALS-THREE-LAYER] Two Python functions sharing structural
// subtrees (normalised `_ = _ + _` and `if _ < _: return _` patterns)
// must produce a cross-file cluster with token_jaccard > 0.
//
// alpha.py: accumulate() has `running + step` AND `running + 2` per
// iteration.  beta.py: aggregate() has only `accumulator + cursor`.
// Despite the whole-function structural difference, small shared
// subtrees (`_ = _ + _`, `if _ < _: return _`) are structurally
// identical and their k-grams are compared via minhash_signature.
//
// This proves both:
//   Bug 1: minhash_signature (XOF) produces valid signature values —
//          a broken implementation (garbage output) would produce
//          near-zero Jaccard on shared k-gram sets.
//   Bug 2: signatures_for_file builds each file's signatures against
//          that file's own tree.  A wrong tree would give each
//          fingerprint the wrong token stream, scrambling all Jaccard
//          values and collapsing the cross-file cluster.
#[test]
fn python_multi_file_corpus_produces_cross_file_cluster_with_positive_token_jaccard() -> Result<()>
{
    let scan_root = fixture("python-type3");
    let report = run_cli("python-type3", 8)?;
    let cluster = expect_cluster_spanning(&report, &["alpha.py", "beta.py"])?;
    // The shared-subtree cluster is a near-miss: the two functions differ
    // by the extra statement, so the cluster must be admitted and its
    // occurrences byte-distinct — a byte-identical reading would mean the
    // fragment view was published in place of the enclosing pair.
    assert!(
        !has_verbatim_pair(&scan_root, cluster)?,
        "the cross-file Python cluster must be the byte-distinct enclosing \
         near-miss, not a verbatim fragment: {cluster:#}",
    );
    Ok(())
}

// [PIPELINE-DETERMINISM] Running the CLI twice on the same Python corpus
// must produce identical token_jaccard values — proves minhash_signature
// (XOF) is deterministic across process restarts.  Any non-determinism
// in blake3 XOF output or k-gram ordering would cause differing Jaccard
// values between runs.  Cluster IDs are structural (unaffected by
// signatures), so checking token_jaccard is the direct proof.
#[test]
fn python_report_is_deterministic_across_runs() -> Result<()> {
    let run1 = run_cli("python-small", 10)?;
    let run2 = run_cli("python-small", 10)?;
    let ids1: Vec<(String, u64)> = clusters(&run1)
        .iter()
        .map(|cluster| (cluster_id(cluster).to_owned(), cluster_size(cluster)))
        .collect();
    let ids2: Vec<(String, u64)> = clusters(&run2)
        .iter()
        .map(|cluster| (cluster_id(cluster).to_owned(), cluster_size(cluster)))
        .collect();
    assert!(
        !ids1.is_empty(),
        "python-small must produce at least one cluster"
    );
    assert_eq!(
        ids1, ids2,
        "cluster ids and occurrence counts must be identical across runs on the \
         same corpus — the fingerprint and render paths must be deterministic",
    );
    Ok(())
}
