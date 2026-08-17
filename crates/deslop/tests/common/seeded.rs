//! The three-file seeded Rust corpus shared by the store-accounting
//! suites (`signature_reuse.rs`, `cache_blob_integrity.rs`): one
//! byte-identical clone pair plus one unrelated file, so cold/warm
//! cache stats are exactly `{0,3}` / `{3,0}` and exactly one
//! `identical` cluster spans the pair. One definition, so the suites
//! can never disagree about what the corpus contains.

use std::{fs, path::Path};

use super::{cluster_bucket, cluster_size, expect_cluster_spanning, field, Result};

/// Files the seeded corpus contains, as the `u64` the cache counters use.
pub(crate) const SEEDED_FILE_COUNT: u64 = 3;

/// Subtree-size floor the seeded corpus is analysed at.
pub(crate) const SEEDED_MIN_NODES: u32 = 8;

/// The clone body shared verbatim by `alpha.rs` and `beta.rs`. Seven
/// lines, byte-identical in both files, so one cluster spanning the
/// pair is guaranteed at `--min-nodes 8`.
const CLONE_BODY: &str = "pub fn compute(items: &[i32]) -> i32 {\n\
    \x20   let mut total = 0;\n\
    \x20   for item in items {\n\
    \x20       if *item > 0 { total += item * 2; } else { total -= item; }\n\
    \x20   }\n\
    \x20   total\n\
}\n";

/// A genuinely different function for `gamma.rs` — real code that
/// duplicates nothing, so the corpus has exactly one clone pair.
const DISTINCT_SOURCE: &str = "pub fn label(count: usize) -> String {\n\
    \x20   match count {\n\
    \x20       0 => \"none\".to_owned(),\n\
    \x20       1 => \"one\".to_owned(),\n\
    \x20       other => format!(\"{other} items\"),\n\
    \x20   }\n\
}\n";

/// Seeds three byte-distinct Rust files: the `alpha.rs`/`beta.rs`
/// clone pair (distinct leading comments keep the file bytes — and so
/// the content-addressed store keys — distinct) plus the unrelated
/// `gamma.rs`.
pub(crate) fn seed_corpus(scan_root: &Path) -> Result<()> {
    fs::create_dir_all(scan_root)?;
    fs::write(
        scan_root.join("alpha.rs"),
        format!("// alpha: the canonical copy.\n{CLONE_BODY}"),
    )?;
    fs::write(
        scan_root.join("beta.rs"),
        format!("// beta: the pasted copy.\n{CLONE_BODY}"),
    )?;
    fs::write(scan_root.join("gamma.rs"), DISTINCT_SOURCE)?;
    Ok(())
}

/// Asserts the corpus really carries the authored clone — three files
/// analysed and one `identical` cluster spanning exactly the pair — so
/// store-accounting assertions can never pass against a blind report.
pub(crate) fn assert_seeded_corpus(report: &serde_json::Value, label: &str) -> Result<()> {
    assert_eq!(
        field(report, "files_analysed").as_u64(),
        Some(SEEDED_FILE_COUNT),
        "{label} run must analyse all three seeded files: {report}"
    );
    let clone = expect_cluster_spanning(report, &["alpha.rs", "beta.rs"])?;
    assert_eq!(
        cluster_bucket(clone),
        "identical",
        "{label}: the seeded pair is byte-identical code in distinct files: {report}"
    );
    assert_eq!(
        cluster_size(clone),
        2,
        "{label}: the clone must span exactly the two seeded occurrences: {report}"
    );
    Ok(())
}
