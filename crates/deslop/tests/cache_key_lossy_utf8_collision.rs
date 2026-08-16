//! The incremental fingerprint cache must key on source bytes, never on
//! lossy text ([PIPELINE-INCREMENTAL]).
//!
//! Pins gh #382. `FingerprintCache::path_for`
//! (`crates/deslop-core/src/fpcache.rs`) once computed its key as
//! `content_hash(&String::from_utf8_lossy(source))`. Every byte sequence
//! that is not valid UTF-8 collapses to U+FFFD before hashing, so two
//! files with *different bytes* — and different byte lengths — shared one
//! cache entry, and the second file read in a run was served the first
//! file's normalised tree and fingerprints: its clone reported one line
//! and one byte early, on the comment instead of the function. When the
//! colliding files are not structurally identical the same collision
//! hands the second file the first file's fingerprints outright — a false
//! positive for code it does not contain, a false negative for the code
//! it does.
//!
//! `beta.rs` below carries a two-byte truncated UTF-8 sequence where
//! `alpha.rs` carries a one-byte invalid unit; both lossy-decode to the
//! same string, so the pair is byte-distinct, lossy-identical, and one
//! byte apart — the minimal collision.
//!
//! `--no-incremental` never consults the cache, so it is the ground truth
//! every cached run is asserted against. The cold cached run must miss
//! for *both* files (an injective key can never let byte-distinct files
//! share an entry), and a second, warm cached run must hit for both and
//! still render the truth spans — proving the cache is genuinely
//! exercised and rehydrates the right file.

use std::{fs, path::Path};

use anyhow::Result;
use assert_cmd::Command;
use serde_json::Value;

mod common;
use crate::common::*;

/// The clone body shared by both files. Seven lines, identical in each,
/// so a cluster spanning the pair is guaranteed regardless of the
/// comment above it.
const BODY: &[u8] = b"pub fn compute(items: &[i32]) -> i32 {\n\
    \x20   let mut total = 0;\n\
    \x20   for item in items {\n\
    \x20       if *item > 0 { total += item * 2; } else { total -= item; }\n\
    \x20   }\n\
    \x20   total\n\
}\n";

/// One invalid UTF-8 unit. Lossy-decodes to a single U+FFFD.
const ONE_BYTE_INVALID: &[u8] = b"\xff";

/// A truncated four-byte UTF-8 sequence — two bytes, and also a single
/// maximal invalid subpart, so it lossy-decodes to the same single
/// U+FFFD as [`ONE_BYTE_INVALID`] while occupying one more byte.
const TWO_BYTE_INVALID: &[u8] = b"\xf0\x9f";

/// `min-nodes` low enough that the shared body fingerprints as one
/// subtree.
const MIN_NODES: &str = "8";

/// Writes `// <invalid>\n` followed by [`BODY`] to `dir/name`.
fn write_clone(dir: &Path, name: &str, invalid: &[u8]) -> Result<()> {
    let mut bytes = b"//".to_vec();
    bytes.extend_from_slice(invalid);
    bytes.push(b'\n');
    bytes.extend_from_slice(BODY);
    Ok(fs::write(dir.join(name), bytes)?)
}

/// Seeds a fresh scan root holding the byte-distinct, lossy-identical
/// pair. Under the defective lossy key, `alpha.rs` (first in sort
/// order) populated the one entry `beta.rs` then collided onto.
fn seed_colliding_pair(scan_root: &Path) -> Result<()> {
    fs::create_dir_all(scan_root)?;
    write_clone(scan_root, "alpha.rs", ONE_BYTE_INVALID)?;
    write_clone(scan_root, "beta.rs", TWO_BYTE_INVALID)?;
    Ok(())
}

/// Runs `deslop` over `scan_root` and returns the JSON report. Passing
/// `incremental` runs the default cache-on path; otherwise
/// `--no-incremental` is added and the cache is never consulted.
fn report(scan_root: &Path, incremental: bool) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let mut cmd = Command::cargo_bin("deslop")?;
    let _args = cmd.arg(scan_root).arg("--output").arg(&output).args([
        "--min-nodes",
        MIN_NODES,
        "--embeddings",
        "off",
    ]);
    if !incremental {
        let _flag = cmd.arg("--no-incremental");
    }
    let _assertion = cmd.assert().success();
    load_json(&output.with_extension("json"))
}

/// The `(start_line, end_line, start_byte, end_byte)` of the sole
/// occurrence in `report` whose path ends with `file_name`.
fn occurrence_span(report: &Value, file_name: &str) -> Result<(u64, u64, u64, u64)> {
    let clusters = field(report, "clusters")
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("report has no clusters array: {report}"))?;
    clusters
        .iter()
        .flat_map(|cluster| {
            field(cluster, "occurrences")
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .find(|occurrence| {
            field(occurrence, "path")
                .as_str()
                .is_some_and(|path| path.ends_with(file_name))
        })
        .and_then(|occurrence| {
            Some((
                field(&occurrence, "start_line").as_u64()?,
                field(&occurrence, "end_line").as_u64()?,
                field(&occurrence, "start_byte").as_u64()?,
                field(&occurrence, "end_byte").as_u64()?,
            ))
        })
        .ok_or_else(|| anyhow::anyhow!("no occurrence for {file_name} in: {report}"))
}

// Implements [PIPELINE-INCREMENTAL]: a cache hit must rehydrate the
// analysis of *this* file. Two files whose bytes differ may never share a
// cache entry, and the cached run's reported spans must equal the spans
// `--no-incremental` computes from the real bytes.
#[test]
fn lossy_utf8_cache_key_must_not_collide_across_distinct_files() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_colliding_pair(&scan_root)?;

    // The two files really are distinct on disk, and really do decode to
    // the same text — without both halves the collision below proves
    // nothing.
    let alpha_bytes = fs::read(scan_root.join("alpha.rs"))?;
    let beta_bytes = fs::read(scan_root.join("beta.rs"))?;
    assert_ne!(
        alpha_bytes, beta_bytes,
        "fixture files must differ in bytes"
    );
    assert_eq!(
        alpha_bytes.len() + 1,
        beta_bytes.len(),
        "beta.rs must be exactly one byte longer than alpha.rs"
    );
    assert_eq!(
        String::from_utf8_lossy(&alpha_bytes),
        String::from_utf8_lossy(&beta_bytes),
        "fixture files must lossy-decode identically"
    );

    let truth = report(&scan_root, false)?;
    let cold = report(&scan_root, true)?;
    let warm = report(&scan_root, true)?;

    // An injective key can never let byte-distinct files share an entry:
    // the cold cached run must miss for both files. A single hit here IS
    // the collision — beta.rs served alpha.rs's tree.
    let cold_stats = field(&cold, "cache_stats");
    assert_eq!(
        field(cold_stats, "hits").as_u64(),
        Some(0),
        "byte-distinct files shared a cache entry on the cold run: {cold}"
    );
    assert_eq!(
        field(cold_stats, "misses").as_u64(),
        Some(2),
        "both files must miss the cold cache: {cold}"
    );

    // The warm run must hit for both files — proving the cached path is
    // genuinely exercised, not silently bypassed.
    let warm_stats = field(&warm, "cache_stats");
    assert_eq!(
        field(warm_stats, "hits").as_u64(),
        Some(2),
        "both files must be served from cache on the warm run: {warm}"
    );
    assert_eq!(
        field(warm_stats, "misses").as_u64(),
        Some(0),
        "warm run must not re-parse either file: {warm}"
    );

    // Ground truth: the clone starts on line 2 of both files, one byte
    // later in beta.rs than in alpha.rs.
    assert_eq!(
        occurrence_span(&truth, "alpha.rs")?,
        (2, 8, 4, 175),
        "uncached alpha.rs span moved: {truth}"
    );
    assert_eq!(
        occurrence_span(&truth, "beta.rs")?,
        (2, 8, 5, 176),
        "uncached beta.rs span moved: {truth}"
    );

    // Cold and warm cached runs must both render the truth spans. Under
    // the defective lossy key, beta.rs was reported one line and one byte
    // early — at alpha.rs's offsets, pointing at the comment line.
    for (label, cached) in [("cold", &cold), ("warm", &warm)] {
        assert_eq!(
            occurrence_span(cached, "alpha.rs")?,
            (2, 8, 4, 175),
            "{label} cached alpha.rs span must match the uncached run: {cached}"
        );
        assert_eq!(
            occurrence_span(cached, "beta.rs")?,
            (2, 8, 5, 176),
            "{label} cache run reported beta.rs at alpha.rs's offsets: {cached}"
        );
    }

    Ok(())
}
