//! E2E regression for GH #343 [FUSION-STRATEGY-MAX-SUM]: sum-then-clamp
//! fusion saturates on correlated mid-band evidence.
//!
//! `PairScore::fused()` sums three correlated views of the same code and
//! clamps to `[0, 1]`. A cluster whose mean signals sum past 1.0 renders
//! `fused = 1.000` — a claim of proven duplication — even though no single
//! axis saturated and no occurrence pair is byte-identical. The
//! [FUSION-CONTENT-GATE] rescues only the two saturating corners
//! (`structural >= 0.99`, `token_jaccard >= 0.95`); the mid band passes
//! under the gate untouched.
//!
//! The fixture pair is `ledger_a.ts` / `ledger_c.ts` from `ts-mixed-band`:
//! two 90-term arithmetic chains differing by one parenthesised head term.
//! The head difference breaks every >= 12-node subtree prefix (the chains
//! are left-leaning), so the pair connects through token LSH and the
//! embedding pass alone: `structural ~ 0`, `token_jaccard` mid-band,
//! `embedding_cos ~ 0.95` under the deterministic [`MockOllama`] vectors.
//! No axis reaches 1.0 and the files are not byte-identical, yet the
//! summed fusion clamps to full confidence.
//!
//! Contract pinned here: without a byte-identical occurrence pair, the
//! rendered confidence never exceeds the strongest single axis, and in
//! particular never saturates at 1.0. The cluster itself is a genuine
//! near-duplicate, so the confidence must also stay act-now-worthy — the
//! fix bounds the fusion, it does not erase the evidence.

#[path = "cli/mock_ollama.rs"]
mod mock_ollama;

use std::path::Path;

use anyhow::Result;
use mock_ollama::MockOllama;
use serde_json::Value;

mod common;
use crate::common::{signals::*, *};

/// Scans a private copy of just the LSH-plus-embedding pair from
/// `ts-mixed-band` with the deterministic mock embedder wired in, so the
/// cluster under test carries no structural anchor and no byte proof.
fn run_two_file_report(server: &MockOllama, scan_root: &Path) -> Result<Value> {
    let fixtures = fixture("ts-mixed-band");
    for name in ["ledger_a.ts", "ledger_c.ts"] {
        let _bytes = std::fs::copy(fixtures.join(name), scan_root.join(name))?;
    }
    let output = scan_root.join("report");
    let mut cmd = deslop_cmd(scan_root, &output)?;
    let _assertion = cmd
        .args([
            "--min-nodes",
            "12",
            "--embeddings",
            "required",
            "--embedding-provider",
            "ollama",
            "--embedding-model",
            "nomic-embed-text",
            "--embedding-endpoint",
            server.endpoint(),
        ])
        .assert()
        .success();
    load_json(&output.with_extension("json"))
}

/// The strongest single axis of the rendered signal triple — the ceiling
/// a bounded fusion of correlated evidence may reach without byte proof.
fn strongest_axis(cluster: &Value) -> f64 {
    signal(cluster, "structural")
        .max(signal(cluster, "token_jaccard"))
        .max(signal(cluster, "embedding_cos"))
}

/// The evidence shape that exposes the clamp: an embedding-dominant
/// near-duplicate with no structural anchor, a mid-band token signal,
/// and no saturated axis. If these drift, the fixture no longer proves
/// anything about the mid band — fail loudly rather than vacuously.
fn assert_mid_band_evidence(scan_root: &Path, cluster: &Value) -> Result<()> {
    let dump = signal_dump(cluster);
    assert!(
        signal(cluster, "structural") < 0.05,
        "the head change must break every shared subtree — {dump}"
    );
    assert!(
        signal(cluster, "token_jaccard") < 0.95,
        "token evidence must stay below the content-gate corner — {dump}"
    );
    let embedding = signal(cluster, "embedding_cos");
    assert!(
        (0.80..=0.99).contains(&embedding),
        "embedding must dominate without saturating — {dump}"
    );
    assert!(
        !has_verbatim_pair(scan_root, cluster)?,
        "the fixture pair must not contain byte-identical occurrences — {dump}"
    );
    Ok(())
}

// GH #343 acceptance: correlated mid-band evidence must not saturate the
// rendered confidence. Today `fused()` clamps 0.00 + 0.30 + 0.95 to a
// flat 1.000 — indistinguishable from a byte-proven verbatim copy.
#[test]
fn mid_band_cluster_confidence_never_exceeds_its_strongest_axis() -> Result<()> {
    let server = MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    let report = run_two_file_report(&server, tmp.path())?;

    assert_eq!(cluster_count(&report), 1, "exactly one visible cluster");
    assert_eq!(clusters_hidden(&report), 0, "nothing routed to hidden");
    let cluster = expect_cluster_spanning(&report, &["ledger_a.ts", "ledger_c.ts"])?;
    assert_eq!(cluster_bucket(cluster), "same_behavior", "routing bucket");
    assert_eq!(
        occurrences(cluster).len(),
        2,
        "one occurrence per ledger file"
    );
    assert_mid_band_evidence(tmp.path(), cluster)?;

    let fused = signal(cluster, "fused");
    let ceiling = strongest_axis(cluster);
    assert!(
        fused <= ceiling + 1e-6,
        "fused confidence exceeded its strongest axis without a byte-identical \
         pair: fused {fused:.4} > max axis {ceiling:.4} — sum-then-clamp \
         saturation, GH #343 — {dump}",
        dump = signal_dump(cluster)
    );
    assert!(
        !approx(fused, 1.0),
        "full confidence without byte proof — {dump}",
        dump = signal_dump(cluster)
    );
    assert!(
        fused >= ACT_NOW_FUSED,
        "bounding the fusion must not erase genuine near-duplicate evidence: \
         fused {fused:.4} fell below the act-now line {ACT_NOW_FUSED} — {dump}",
        dump = signal_dump(cluster)
    );
    Ok(())
}
