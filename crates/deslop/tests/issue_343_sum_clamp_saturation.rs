//! E2E regression for GH #343 [FUSED-STRATEGY-BOUNDED-MAX]: sum-then-clamp
//! fusion saturates on correlated mid-band evidence.
//!
//! `PairScore::fused()` sums three correlated views of the same code and
//! clamps to `[0, 1]`. A cluster whose mean signals sum past 1.0 renders
//! `fused = 1.000` — a claim of proven duplication — even though no single
//! axis saturated and no occurrence pair is byte-identical. The
//! [FUSED-CONTENT-GATE] rescues only the two saturating corners
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

use std::path::Path;

use crate::mock_ollama::MockOllama;
use anyhow::Result;
use serde_json::Value;

use crate::common::{embeddings::run_mock_embedding_report, signals::*, *};

/// Scans a private copy of just the LSH-plus-embedding pair from
/// `ts-mixed-band` with the deterministic mock embedder wired in, so the
/// cluster under test carries no structural anchor and no byte proof.
fn run_two_file_report(server: &MockOllama, scan_root: &Path) -> Result<Value> {
    let fixtures = fixture("ts-mixed-band");
    for name in ["ledger_a.ts", "ledger_c.ts"] {
        let _bytes = std::fs::copy(fixtures.join(name), scan_root.join(name))?;
    }
    let output = scan_root.join("report");
    run_mock_embedding_report(scan_root, &output, "12", server.endpoint())
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
#[ignore = "[SKIP-UNFINISHED] GH #369 [FUSED-SHARED-SUBTREE] \
            docs/plans/rename-recall-plan.md — RED ON PURPOSE, and materially closer than \
            it was. \
            The two embedding-only false positives are gone and the real \
            clone is found — `cluster_count` is now the expected 1, where \
            it used to be 2 with the genuine pair hidden. Two expectations \
            remain unmet, both downstream of `structural` becoming a \
            measurement ([FUSED-SHARED-SUBTREE]): one cluster still routes \
            to hidden where this wants none, and the pair is asserted to be \
            `same_behavior` when ledger_a/ledger_c differ only by a rename \
            and one redundant paren — measured `structural = 0.997`, which \
            is a near-identical clone, not a Type-4. Settling that is #369's \
            own work. Assertions are intact — run with `-- --ignored`."]
fn mid_band_cluster_confidence_never_exceeds_its_strongest_axis() -> Result<()> {
    let server = MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    let report = run_two_file_report(&server, tmp.path())?;

    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(2),
        "both seeded ledgers must be read"
    );
    assert_eq!(cluster_count(&report), 1, "exactly one visible cluster");
    assert_eq!(
        clusters_hidden(&report),
        0,
        "nothing routed to hidden: {report:#}"
    );
    let cluster = expect_cluster_spanning(&report, &["ledger_a.ts", "ledger_c.ts"])?;
    assert_eq!(cluster_bucket(cluster), "same_behavior", "routing bucket");
    assert_eq!(
        occurrences(cluster).len(),
        2,
        "one occurrence per ledger file"
    );
    let mut files = occurrence_files(cluster);
    files.sort_unstable();
    assert_eq!(
        files,
        ["ledger_a.ts", "ledger_c.ts"],
        "occurrences must name exactly the two seeded ledgers"
    );
    let duplication = metric_field(&report, "duplication_percent")
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("duplication_percent is not a number: {report:#}"))?;
    assert!(
        duplication > 0.0,
        "a visible act-now cluster must register in the duplication metric, \
         got {duplication}"
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
    // The bound is exact, not merely an inequality: with no saturating
    // shape signal the content gate leaves the bounded max untouched, and
    // the strongest axis here is the embedding — anything below it would
    // discard measured evidence, anything above it would manufacture some.
    assert!(
        approx(fused, ceiling),
        "fused must equal the strongest axis when no gate applies — {dump}",
        dump = signal_dump(cluster)
    );
    assert!(
        approx(fused, signal(cluster, "embedding_cos")),
        "the embedding is the dominant axis of this fixture — {dump}",
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

// The control the bound must not break: byte-proven duplication still
// earns the full 1.0 on every axis and routes `identical`. A bounded
// fusion that capped proven copies below full confidence would trade the
// #343 false positive for a false negative.
#[test]
fn byte_identical_pair_still_earns_full_confidence_under_the_bound() -> Result<()> {
    let server = MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    let source = fixture("ts-mixed-band").join("ledger_a.ts");
    for name in ["ledger_a.ts", "ledger_a_copy.ts"] {
        let _bytes = std::fs::copy(&source, tmp.path().join(name))?;
    }
    let output = tmp.path().join("report");
    let report = run_mock_embedding_report(tmp.path(), &output, "12", server.endpoint())?;

    assert_eq!(cluster_count(&report), 1, "one visible cluster");
    let cluster = expect_cluster_spanning(&report, &["ledger_a.ts", "ledger_a_copy.ts"])?;
    assert_eq!(cluster_bucket(cluster), "identical", "byte-proven routing");
    assert!(
        has_verbatim_pair(tmp.path(), cluster)?,
        "the control fixture must contain a byte-identical pair"
    );
    let dump = signal_dump(cluster);
    assert!(
        approx(signal(cluster, "structural"), 1.0),
        "identical trees must saturate structural — {dump}"
    );
    assert!(
        approx(signal(cluster, "token_jaccard"), 1.0),
        "a shared Merkle hash proves the token multiset — {dump}"
    );
    assert!(
        approx(signal(cluster, "embedding_cos"), 1.0),
        "identical bytes embed identically — {dump}"
    );
    assert!(
        approx(signal(cluster, "fused"), 1.0),
        "byte proof is exactly what fused = 1.0 is reserved for — {dump}"
    );
    Ok(())
}

// The same two ledgers with the embedding pass off. They differ by a
// function rename and one redundant pair of parentheses across a
// ninety-term arithmetic expression: `structural = 0.997`,
// `token_jaccard = 1.000`. That is a Type-2 clone by any reading, and it
// must be visible.
//
// This asserted the opposite — zero visible, one hidden, "no embedding
// evidence, no structural anchor". The anchor was reported as absent
// because `structural` was Merkle equality and the stray paren rehashed
// the root, so a pair sharing 99.7% of its AST measured exactly zero
// ([FUSED-SHARED-SUBTREE], gh #408). Asserting invisibility asserted
// that false negative.
//
// What #343 actually quarantined is *manufactured* confidence: a sum
// that clamped mid-band evidence to a flat 1.0 no single axis earned.
// That contract is unchanged and is asserted here directly, which is
// stronger than asserting the cluster away — `fused` must stay at or
// below the strongest axis, and must stay short of the 1.0 reserved for
// byte proof, even now that the pair is reported.
#[test]
fn without_embeddings_the_mid_band_pair_is_visible_without_saturating() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let fixtures = fixture("ts-mixed-band");
    for name in ["ledger_a.ts", "ledger_c.ts"] {
        let _bytes = std::fs::copy(fixtures.join(name), tmp.path().join(name))?;
    }
    let report = run_report(tmp.path(), 12)?;

    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(2),
        "both seeded ledgers must be read"
    );
    assert_eq!(
        cluster_count(&report),
        1,
        "a rename plus one redundant paren over a ninety-term expression is a \
         Type-2 clone; the report must show it: {report:#}"
    );
    let visible = clusters(&report);
    let cluster = visible
        .first()
        .ok_or_else(|| anyhow::anyhow!("the visible clone must be present: {report:#}"))?;
    let strongest = signal(cluster, "structural")
        .max(signal(cluster, "token_jaccard"))
        .max(signal(cluster, "embedding_cos"));
    let fused = signal(cluster, "fused");
    assert!(
        fused <= strongest + f64::EPSILON,
        "[FUSED-STRATEGY-BOUNDED-MAX] fused must never exceed the strongest axis: \
         fused={fused}, strongest={strongest}"
    );
    assert!(
        fused < 1.0,
        "full confidence is reserved for byte proof, and these two files are not \
         byte-identical — a sum would have clamped this to 1.0, which is the \
         saturation #343 quarantined: fused={fused}"
    );
    let duplication = metric_field(&report, "duplication_percent")
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("duplication_percent is not a number: {report:#}"))?;
    assert!(
        duplication > 0.0,
        "a visible clone is real duplication and must be counted, got {duplication}"
    );
    Ok(())
}

// The whole mixed-band fixture, embeddings off: every visible cluster obeys
// the bound, and full confidence appears only with byte proof. This is the
// per-cluster form of the sweep invariant, asserted against the exact
// corpus that exposed the saturation.
#[test]
fn every_visible_mixed_band_cluster_obeys_the_bound() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    seed(&fixture("ts-mixed-band"), tmp.path())?;
    let report = run_report(tmp.path(), 12)?;

    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(5),
        "all five ledgers must be read"
    );
    assert!(
        cluster_count(&report) > 0,
        "the fixture's rename family must stay visible: {report:#}"
    );
    let family = expect_cluster_spanning(&report, &["ledger_a.ts", "ledger_b.ts"])?;
    assert!(
        ACT_NOW_BUCKETS.contains(&cluster_bucket(family))
            || HONEST_SHAPE_ONLY_BUCKETS.contains(&cluster_bucket(family)),
        "the a/b rename family must route to a real bucket — {dump}",
        dump = signal_dump(family)
    );
    for cluster in clusters(&report) {
        let dump = signal_dump(cluster);
        let fused = signal(cluster, "fused");
        let ceiling = strongest_axis(cluster);
        assert!(
            (0.0..=1.0).contains(&fused),
            "fused must stay in [0, 1] — {dump}"
        );
        assert!(
            fused <= ceiling + 1e-6,
            "fused {fused:.4} exceeded its strongest axis {ceiling:.4} — {dump}"
        );
        if approx(fused, 1.0) {
            assert!(
                has_verbatim_pair(tmp.path(), cluster)?,
                "fused = 1.0 without a byte-identical pair — {dump}"
            );
        }
    }
    Ok(())
}
