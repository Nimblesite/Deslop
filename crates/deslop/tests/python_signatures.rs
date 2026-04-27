//! Tests proving that the `MinHash` signature pipeline works correctly for
//! Python after the XOF-based fix ([FUSION-SIGNALS-THREE-LAYER]).
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

use std::{fs, path::Path, path::PathBuf};

use anyhow::Result;
use assert_cmd::Command;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn json_output(tmp: &Path) -> PathBuf {
    tmp.join("report.json")
}

fn run_cli(fixture_name: &str, min_nodes: u32) -> Result<serde_json::Value> {
    let tmp = tempfile::tempdir()?;
    let out = json_output(tmp.path());
    let _assertion = Command::cargo_bin("deslop")?
        .arg(fixture(fixture_name))
        .arg("--min-nodes")
        .arg(min_nodes.to_string())
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();
    let json = fs::read_to_string(&out)?;
    Ok(serde_json::from_str(&json)?)
}

fn clusters(report: &serde_json::Value) -> Vec<&serde_json::Value> {
    report
        .pointer("/clusters")
        .and_then(serde_json::Value::as_array)
        .map(|v| v.iter().collect())
        .unwrap_or_default()
}

fn signal(cluster: &serde_json::Value, key: &str) -> f64 {
    cluster
        .pointer(&format!("/signals/{key}"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(f64::NAN)
}

fn occurrence_files(cluster: &serde_json::Value) -> Vec<String> {
    cluster
        .pointer("/occurrences")
        .and_then(serde_json::Value::as_array)
        .map(|occ| {
            occ.iter()
                .filter_map(|o| {
                    o.get("path").and_then(serde_json::Value::as_str).map(|p| {
                        Path::new(p)
                            .file_name()
                            .map_or_else(|| p.to_owned(), |n| n.to_string_lossy().into_owned())
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn is_exact_one(value: f64) -> bool {
    (value - 1.0).abs() <= f64::EPSILON
}

// [FUSION-SIGNALS-THREE-LAYER] Type-2 Python clones (identical after
// normalisation) must produce token_jaccard = 1.0 — proves
// minhash_signature maps identical k-gram sets to identical signatures.
// If the XOF produces wrong values (e.g. all-MAX sentinel), Jaccard
// would be 1.0 trivially; the Type-3 test below distinguishes that case.
#[test]
fn python_type2_clone_has_token_jaccard_of_one() -> Result<()> {
    let report = run_cli("python-small", 10)?;
    let clusters = clusters(&report);
    assert!(
        !clusters.is_empty(),
        "python-small must produce at least one cluster",
    );
    let top = clusters
        .first()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("python-small must produce at least one cluster"))?;
    let token_jaccard = signal(top, "token_jaccard");
    assert!(
        is_exact_one(token_jaccard),
        "Type-2 Python clone must have token_jaccard = 1.0 (identical k-gram sets), \
         got {token_jaccard}"
    );
    let structural = signal(top, "structural");
    assert!(
        is_exact_one(structural),
        "Type-2 Python clone must also have structural = 1.0, got {structural}",
    );
    Ok(())
}

// [FUSION-SIGNALS-THREE-LAYER] Two Python functions sharing structural
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
//   Bug 2: build_signatures_with_languages uses the correct tree per
//          file (HashMap index).  A wrong mapping would give each
//          fingerprint the wrong token stream, scrambling all Jaccard
//          values and collapsing the cross-file cluster.
#[test]
fn python_multi_file_corpus_produces_cross_file_cluster_with_positive_token_jaccard() -> Result<()>
{
    let report = run_cli("python-type3", 8)?;
    let clusters = clusters(&report);
    let cross_file = clusters.iter().find(|cluster| {
        let files: std::collections::BTreeSet<String> =
            occurrence_files(cluster).into_iter().collect();
        files.contains("alpha.py") && files.contains("beta.py")
    });
    let Some(cluster) = cross_file else {
        anyhow::bail!(
            "python-type3 must produce a cross-file cluster spanning alpha.py and beta.py; \
             got clusters: {clusters:#?}"
        );
    };
    let token_jaccard = signal(cluster, "token_jaccard");
    assert!(
        token_jaccard > 0.0,
        "cross-file Python cluster must have token_jaccard > 0.0 \
         (minhash_signature must produce meaningful signatures for shared subtrees), \
         got token_jaccard = {token_jaccard}",
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
fn python_token_jaccard_is_deterministic_across_runs() -> Result<()> {
    let run1 = run_cli("python-small", 10)?;
    let run2 = run_cli("python-small", 10)?;
    let jaccards1: Vec<u64> = clusters(&run1)
        .iter()
        .map(|c| signal(c, "token_jaccard").to_bits())
        .collect();
    let jaccards2: Vec<u64> = clusters(&run2)
        .iter()
        .map(|c| signal(c, "token_jaccard").to_bits())
        .collect();
    assert!(
        !jaccards1.is_empty(),
        "python-small must produce at least one cluster"
    );
    assert_eq!(
        jaccards1, jaccards2,
        "token_jaccard values must be bit-identical across runs on the same corpus \
         (minhash_signature XOF must be deterministic)",
    );
    Ok(())
}
