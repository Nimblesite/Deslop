//! The incremental fingerprint cache keys on lossy text, not source bytes
//! ([PIPELINE-INCREMENTAL]).
//!
//! `FingerprintCache::path_for` (`crates/deslop-core/src/fpcache.rs`) computes
//! its key as `content_hash(&String::from_utf8_lossy(source))`. Every
//! byte sequence that is not valid UTF-8 collapses to U+FFFD before it is
//! hashed, so two files with *different bytes* — and different byte
//! lengths — share one cache entry. The second file read in a run is
//! served the first file's normalised tree and fingerprints.
//!
//! The consequence is a report that points at the wrong code. `beta.rs`
//! below carries a two-byte truncated UTF-8 sequence where `alpha.rs`
//! carries a one-byte invalid unit; both lossy-decode to the same string,
//! so `beta.rs` is one byte longer and every offset inside it is shifted
//! by one. Served `alpha.rs`'s cached ranges, the cached run reports
//! `beta.rs`'s clone one line and one byte early — a user following the
//! report lands on the comment line, not the function.
//!
//! The shift is bounded only by how many invalid units the two files
//! disagree on, so the same collision can slide a reported range onto
//! entirely unrelated code, and — when the colliding files are not
//! structurally identical — hand the second file the first file's
//! fingerprints outright, which is a false positive for code `beta.rs`
//! does not contain and a false negative for the code it does.
//!
//! `--no-incremental` never consults the cache, so it is the ground truth
//! the cached run is asserted against.

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

/// Seeds a fresh scan root holding the colliding pair. `alpha.rs` sorts
/// first, so it populates the cache entry that `beta.rs` then collides
/// onto within the same run.
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
    let _args = cmd
        .arg(scan_root)
        .arg("--output")
        .arg(&output)
        .args(["--min-nodes", MIN_NODES, "--embeddings", "off"]);
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
        .flat_map(|cluster| field(cluster, "occurrences").as_array().cloned().unwrap_or_default())
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
    assert_ne!(alpha_bytes, beta_bytes, "fixture files must differ in bytes");
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
    let cached = report(&scan_root, true)?;

    // The cached run must actually have consulted the cache, or this
    // test would pass without exercising the defect at all.
    let stats = field(&cached, "cache_stats");
    assert_eq!(
        field(stats, "hits").as_u64(),
        Some(1),
        "beta.rs must be served from alpha.rs's entry for this to test anything: {cached}"
    );
    assert_eq!(
        field(stats, "misses").as_u64(),
        Some(1),
        "alpha.rs must be the sole miss: {cached}"
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

    // The defect: served alpha.rs's cached tree, beta.rs is reported one
    // line and one byte early — the comment line, not the function.
    assert_eq!(
        occurrence_span(&cached, "alpha.rs")?,
        occurrence_span(&truth, "alpha.rs")?,
        "cached alpha.rs span must match the uncached run: {cached}"
    );
    assert_eq!(
        occurrence_span(&cached, "beta.rs")?,
        (2, 8, 5, 176),
        "cache collision reported beta.rs at alpha.rs's offsets: {cached}"
    );

    Ok(())
}
