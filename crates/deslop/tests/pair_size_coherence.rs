//! E2E accuracy regression for [PAIR-SIZE-COHERENCE]: an embedding-only
//! pair may not join occurrences of wildly different size.
//!
//! `survival_decision` (`crates/deslop-core/src/pair.rs`) consults
//! `min_node_count` only for LSH-only pairs. A pair carrying no
//! structural anchor but a high `embedding_cos` is admitted to transitive
//! closure whatever the size of its two endpoints, so a cluster can grow a
//! member that shares nothing with the rest of it.
//!
//! Against `ts-mixed-band`'s `ledger_a.ts` / `ledger_c.ts` that produces a
//! rendered cluster reading
//!
//! > 3 copies of a 19-node subtree at ledger_a.ts:1:22, ledger_c.ts:2:10,
//! > ledger_c.ts:1:23
//!
//! whose members are the 67-byte parameter list
//! `(alpha: number, beta: number, gamma: number, delta: number): number`
//! twice — a real duplicate — and `ledger_c.ts[101..966]`, an 865-byte,
//! 274-node arithmetic chain that is not a copy of anything. The cluster
//! renders `same_behavior` at `fused = 1.00` and is not hidden, so a user
//! is told a parameter list and a 90-term expression are the same code.
//!
//! Nothing here depends on the embedder agreeing with the mock: a 14x
//! node-count disparity is self-contradictory against the cluster's own
//! `canonical_node_count`, and real embedding models return high cosine
//! for same-language, same-identifier code of very different lengths.
//!
//! Contract pinned here: every occurrence in a visible cluster is size
//! coherent with its siblings, and the two-ledger scan reports exactly the
//! one real duplicate family.

use std::path::Path;

use crate::mock_ollama::MockOllama;
use anyhow::Result;
use serde_json::Value;

use crate::common::{embeddings::run_mock_embedding_report, signals::*, *};

/// Largest byte span an occurrence may have relative to the smallest in
/// the same cluster. A duplicate family is a set of copies; a member four
/// times the size of another is a different quantity of code, not a copy
/// of it. Deliberately loose — Type-3 clones do grow and shrink — so that
/// a failure here means genuine incoherence rather than a tight threshold.
const MAX_SPAN_RATIO: f64 = 4.0;

/// Byte span of one rendered occurrence.
fn occurrence_span(occurrence: &Value) -> Result<usize> {
    let start = occurrence_byte(occurrence, "start_byte")?;
    let end = occurrence_byte(occurrence, "end_byte")?;
    Ok(end.saturating_sub(start))
}

/// Renders a cluster's occurrences as `path[start..end] (N bytes)` so a
/// failure names the offending member instead of a bare ratio.
fn span_dump(cluster: &Value) -> Result<String> {
    let mut lines = Vec::new();
    for occurrence in occurrences(cluster) {
        let start = occurrence_byte(occurrence, "start_byte")?;
        let end = occurrence_byte(occurrence, "end_byte")?;
        lines.push(format!(
            "{}[{start}..{end}] ({} bytes)",
            occurrence_path(occurrence)?,
            end.saturating_sub(start),
        ));
    }
    Ok(lines.join(", "))
}

/// Asserts every occurrence in `cluster` is size coherent with the
/// smallest one ([PAIR-SIZE-COHERENCE]).
fn assert_cluster_is_size_coherent(cluster: &Value) -> Result<()> {
    let mut spans = Vec::new();
    for occurrence in occurrences(cluster) {
        spans.push(occurrence_span(occurrence)?);
    }
    let (Some(&smallest), Some(&largest)) = (spans.iter().min(), spans.iter().max()) else {
        anyhow::bail!("cluster rendered with no occurrences: {cluster:#}");
    };
    let ratio = lossy_ratio(largest, smallest);
    assert!(
        ratio <= MAX_SPAN_RATIO,
        "cluster {id} claims {size} copies of a {nodes}-node subtree but its \
         members differ {ratio:.1}x in size — {dump} [{signals}]",
        id = cluster_id(cluster),
        size = cluster_size(cluster),
        nodes = field(cluster, "canonical_node_count"),
        dump = span_dump(cluster)?,
        signals = signal_dump(cluster),
    );
    Ok(())
}

/// Span ratio as `f64`, treating a zero-width smallest span as maximally
/// incoherent rather than dividing by zero.
fn lossy_ratio(largest: usize, smallest: usize) -> f64 {
    let denominator = u32::try_from(smallest).unwrap_or(u32::MAX);
    if denominator == 0 {
        return f64::INFINITY;
    }
    f64::from(u32::try_from(largest).unwrap_or(u32::MAX)) / f64::from(denominator)
}

/// Seeds just the embedding-driven ledger pair and scans it with the
/// deterministic mock embedder wired in.
fn run_two_ledger_report(server: &MockOllama, scan_root: &Path) -> Result<Value> {
    let fixtures = fixture("ts-mixed-band");
    for name in ["ledger_a.ts", "ledger_c.ts"] {
        let _bytes = std::fs::copy(fixtures.join(name), scan_root.join(name))?;
    }
    let output = scan_root.join("report");
    run_mock_embedding_report(scan_root, &output, "12", server.endpoint())
}

// [PAIR-SIZE-COHERENCE] The two-ledger scan must report the one real
// near-duplicate family and nothing else. Today it also reports a
// 19-node parameter list and an 865-byte arithmetic chain as copies of
// each other.
#[test]
fn an_embedding_only_pair_does_not_join_occurrences_of_different_size() -> Result<()> {
    let server = MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    let report = run_two_ledger_report(&server, tmp.path())?;

    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(2),
        "both seeded ledgers must be read"
    );
    for cluster in clusters(&report) {
        assert_cluster_is_size_coherent(cluster)?;
    }
    assert_eq!(
        cluster_count(&report),
        1,
        "the ledger pair is one near-duplicate family, not two: {report:#}"
    );
    let family = expect_cluster_spanning(&report, &["ledger_a.ts", "ledger_c.ts"])?;
    assert_eq!(
        occurrences(family).len(),
        2,
        "one occurrence per ledger — {dump}",
        dump = span_dump(family)?
    );
    assert_structural_only_contract(family, "pair-size coherence near-duplicate");
    assert_no_pair_surface_on_cluster(family, "pair-size coherence near-duplicate");
    Ok(())
}

// [PAIR-SIZE-COHERENCE] The size guard must not erase real duplication:
// the whole five-ledger corpus still reports its rename family, and every
// surviving cluster is internally coherent.
#[test]
fn size_coherence_keeps_every_genuine_ledger_family_visible() -> Result<()> {
    let server = MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    seed(&fixture("ts-mixed-band"), tmp.path())?;
    let output = tmp.path().join("report");
    let report = run_mock_embedding_report(tmp.path(), &output, "12", server.endpoint())?;

    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(5),
        "all five ledgers must be read"
    );
    assert!(
        cluster_count(&report) > 0,
        "the rename family must stay visible: {report:#}"
    );
    let family = expect_cluster_spanning(&report, &["ledger_a.ts", "ledger_b.ts"])?;
    assert_structural_only_contract(family, "pair-size coherence a/b family");
    assert_no_pair_surface_on_cluster(family, "pair-size coherence a/b family");
    for cluster in clusters(&report) {
        assert_cluster_is_size_coherent(cluster)?;
    }
    Ok(())
}
