//! [PIPELINE-DETERMINISM] Cold-run report golden.
//!
//! One fixed corpus (`tests/fixtures/report-golden/src`), one fixed flag
//! set (`--no-incremental --embeddings off --min-nodes 16 --notext
//! --nohtml`), one committed rendering
//! (`tests/fixtures/report-golden/expected-report.json`). The pipeline
//! must render bit-identical reports over an unchanged corpus, and every
//! reuse path owes this exact cold report
//! ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]) — a warm cache, a
//! spliced live session, a delta re-analysis — so any drift in ranking,
//! spans, cluster ids, metrics arithmetic, or serialisation order fails
//! here first.
//!
//! Two halves, mirroring the AST golden guard in
//! `tests/cli/cache_and_debug.rs`: **unchanged** — the rendered bytes
//! must equal the committed golden byte-for-byte; **correct** — the
//! committed golden must independently satisfy invariants derived from
//! the authored fixture sources, so a wrongly-blessed golden cannot
//! self-certify.
//!
//! Regenerate with `DESLOP_BLESS=1 cargo test -p deslop --test suite
//! report_golden::`, then review the diff — see
//! `tests/fixtures/report-golden/README.md`.

use std::{fs, path::PathBuf};

use crate::common::{golden::*, *};

mod contract;

/// Fixed `--min-nodes` the golden is rendered at. 16 sits above the
/// 12-node `settle_invoice` signature subtree (which would otherwise
/// surface as a third cluster) and below both authored clone bodies
/// (58 and 38 nodes), so exactly the two authored clusters render.
const GOLDEN_MIN_NODES: u64 = 16;

/// The three corpus files carrying the byte-identical `settle_invoice`
/// clone — the larger, higher-ranked cluster.
const TRIO_FILES: [&str; 3] = ["alpha.rs", "beta.rs", "gamma.rs"];

/// The two corpus files carrying the byte-identical `merge_labels`
/// clone — the smaller, lower-ranked cluster.
const PAIR_FILES: [&str; 2] = ["delta.rs", "epsilon.rs"];

/// `tests/fixtures/report-golden`.
fn golden_dir() -> PathBuf {
    fixture("report-golden")
}

/// The authored corpus the golden report describes.
fn corpus_dir() -> PathBuf {
    golden_dir().join("src")
}

/// The committed golden rendering.
fn golden_report_path() -> PathBuf {
    golden_dir().join("expected-report.json")
}

/// The command that regenerates the committed golden.
const BLESS: &str = "`DESLOP_BLESS=1 cargo test -p deslop --test suite report_golden::`";

/// Why a drift here is worth investigating before it is blessed away.
const DRIFT_HINT: &str = "Ranking, spans, ids and metrics all change user-visible output.";

/// Copies the corpus into a throwaway scan root and renders the cold
/// report with the fixed flag set, returning the raw JSON bytes. The
/// checked-in fixture is never scanned in place, and `deslop_cmd`
/// carries `--no-incremental`, so no run can seed a cache anywhere.
fn render_cold_report() -> Result<Vec<u8>> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed(&corpus_dir(), &scan_root)?;
    let output = tmp.path().join("out").join("report");
    let mut cmd = deslop_cmd(&scan_root, &output)?;
    let _assertion = cmd
        .args(["--embeddings", "off", "--min-nodes"])
        .arg(GOLDEN_MIN_NODES.to_string())
        .args(["--notext", "--nohtml"])
        .assert()
        .success();
    Ok(fs::read(output.with_extension("json"))?)
}

// [PIPELINE-DETERMINISM] Half one: unchanged. Two fresh scan roots must
// render byte-identical reports, and those bytes must equal the
// committed golden exactly. `tool_version` is embedded in the report,
// so a workspace version bump legitimately lands here and requires a
// reviewed re-bless.
#[test]
fn cold_report_matches_committed_golden_byte_for_byte() -> Result<()> {
    let first = render_cold_report()?;
    let second = render_cold_report()?;
    let rendered = String::from_utf8(first)?;
    assert_eq!(
        rendered,
        String::from_utf8(second)?,
        "two cold renders over identical corpora must be bit-identical [PIPELINE-DETERMINISM]"
    );
    assert_matches_golden(&rendered, &golden_report_path(), BLESS, DRIFT_HINT)
}
