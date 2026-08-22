//! E2E for [METRICS-REPO] `folders`: the engine — and only the engine —
//! computes per-folder duplication rows. One row per folder prefix with
//! any duplicated line, summing every child file's `analysed_loc` and
//! `duplicated_loc` (clean files included, keeping the denominator
//! exact) and deriving `duplication_percent` through the same single
//! `percent` function as the repo and per-file figures. Consumers render
//! these rows verbatim; recomputing a percentage outside the engine is
//! prohibited, so this suite is the contract every client leans on.


use std::{fs, path::Path};

use crate::common::*;

/// The clone body shared verbatim by `src/a/alpha.rs` and
/// `src/b/beta.rs`: seven lines, byte-identical, guaranteed to cluster
/// at `--min-nodes 8`.
const CLONE_BODY: &str = "pub fn compute(items: &[i32]) -> i32 {\n\
    \x20   let mut total = 0;\n\
    \x20   for item in items {\n\
    \x20       if *item > 0 { total += item * 2; } else { total -= item; }\n\
    \x20   }\n\
    \x20   total\n\
}\n";

/// Twelve clean lines for `src/a/clean.rs`. Every block has its own
/// shape — one guard `if`, one two-arm `match` — because a repeated
/// shape would be a *real* duplicate and the detector would rightly
/// cluster it. Its lines must stay in the `src/a` and `src`
/// denominators while contributing nothing to any numerator.
const CLEAN_SOURCE: &str = "/// Formats a byte count as a human-readable size.\n\
pub fn format_size(bytes: u64) -> String {\n\
    \x20   let kib = bytes / 1024;\n\
    \x20   if bytes < 1024 {\n\
    \x20       return format!(\"{bytes} B\");\n\
    \x20   }\n\
    \x20   let mib = kib / 1024;\n\
    \x20   match mib {\n\
    \x20       0 => format!(\"{kib} KiB\"),\n\
    \x20       _ => format!(\"{mib} MiB\"),\n\
    \x20   }\n\
}\n";

/// Seven clean lines for `lib/gamma.rs` — an iterator chain, shaped
/// like nothing else here, so the `lib` folder has zero duplicated
/// lines and must be absent from `folders` entirely.
const GAMMA_SOURCE: &str = "/// Counts the vowels in `text`, case-sensitively,\n\
/// as the corpus's unrelated clean-file control.\n\
pub fn vowel_count(text: &str) -> usize {\n\
    \x20   text.chars()\n\
    \x20       .filter(|letter| \"aeiou\".contains(*letter))\n\
    \x20       .count()\n\
}\n";

/// Seeds the nested corpus: the clone pair split across `src/a` and
/// `src/b`, a clean file beside the first copy, and a clean-only `lib`.
fn seed_nested_corpus(scan_root: &Path) -> Result<()> {
    for dir in ["src/a", "src/b", "lib"] {
        fs::create_dir_all(scan_root.join(dir))?;
    }
    fs::write(
        scan_root.join("src/a/alpha.rs"),
        format!("// alpha: the canonical copy.\n{CLONE_BODY}"),
    )?;
    fs::write(scan_root.join("src/a/clean.rs"), CLEAN_SOURCE)?;
    fs::write(
        scan_root.join("src/b/beta.rs"),
        format!("// beta: the pasted copy.\n{CLONE_BODY}"),
    )?;
    fs::write(scan_root.join("lib/gamma.rs"), GAMMA_SOURCE)?;
    Ok(())
}

/// Reads `metrics.folders` as `(path, analysed_loc, duplicated_loc,
/// duplication_percent)` rows in reported order.
fn folder_rows(report: &serde_json::Value) -> Vec<(String, u64, u64, f64)> {
    metric_field(report, "folders")
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|row| {
            (
                field(row, "path").as_str().unwrap_or_default().to_owned(),
                field(row, "analysed_loc").as_u64().unwrap_or(u64::MAX),
                field(row, "duplicated_loc").as_u64().unwrap_or(u64::MAX),
                field(row, "duplication_percent")
                    .as_f64()
                    .unwrap_or(f64::NAN),
            )
        })
        .collect()
}

/// Asserts one folder row carries exactly the expected path, sums, and
/// the official [METRICS-REPO] percentage derived from those sums.
fn assert_folder(row: &(String, u64, u64, f64), expected: (&str, u64, u64, f64)) {
    let (path, analysed, duplicated, percent) = row;
    let (expected_path, expected_analysed, expected_duplicated, expected_percent) = expected;
    assert_eq!(path, expected_path, "folder rows must sort worst-first");
    assert_eq!(
        *analysed, expected_analysed,
        "{path}: analysed_loc must sum every child file, clean files included"
    );
    assert_eq!(
        *duplicated, expected_duplicated,
        "{path}: duplicated_loc must sum every child file's duplicated lines"
    );
    assert!(
        approx(*percent, expected_percent),
        "{path}: duplication_percent {percent} must be 100 × {expected_duplicated} / {expected_analysed} = {expected_percent}"
    );
}

#[test]
fn folder_rollup_rows_are_engine_computed() -> Result<()> {
    let scan_root = tempfile::tempdir()?;
    seed_nested_corpus(scan_root.path())?;
    let report = run_report(scan_root.path(), 8)?;

    // The corpus produces exactly one cluster: the identical pair. A
    // second cluster would mean a "clean" fixture file actually
    // duplicates something, which would corrupt every figure below.
    assert_eq!(
        cluster_count(&report),
        1,
        "the clone pair is the only duplication"
    );
    let clone = expect_cluster_spanning(&report, &["alpha.rs", "beta.rs"])?;
    assert_eq!(cluster_bucket(clone), "identical");

    // Repo headline first — 14 duplicated of 35 analysed lines = 40%.
    assert_eq!(metric_field(&report, "analysed_loc").as_u64(), Some(35));
    assert_eq!(metric_field(&report, "duplicated_loc").as_u64(), Some(14));
    let repo_percent = metric_field(&report, "duplication_percent")
        .as_f64()
        .unwrap_or(f64::NAN);
    assert!(
        approx(repo_percent, 100.0 * 14.0 / 35.0),
        "repo percent {repo_percent} must be 100 × 14 / 35 = 40"
    );

    let rows = folder_rows(&report);
    let paths: Vec<&str> = rows.iter().map(|(path, ..)| path.as_str()).collect();
    // Exactly the duplicated prefixes, worst-first ([METRICS-REPO]):
    // src/b 87.5% → src 50% → src/a 35%. `lib` holds only clean code, so
    // it must not appear at all, and paths are `/`-joined scan-root
    // prefixes on every platform.
    assert_eq!(
        paths,
        vec!["src/b", "src", "src/a"],
        "folders must list every duplicated prefix worst-first and omit clean `lib`"
    );

    let expected_rows = [
        // beta.rs alone: 7 duplicated of 8 lines.
        ("src/b", 8, 7, 100.0 * 7.0 / 8.0),
        // Both copies plus the clean file: 14 of 28.
        ("src", 28, 14, 100.0 * 14.0 / 28.0),
        // alpha.rs plus the 12-line clean file: 7 of 20.
        ("src/a", 20, 7, 100.0 * 7.0 / 20.0),
    ];
    for (row, expected) in rows.iter().zip(expected_rows) {
        assert_folder(row, expected);
    }

    Ok(())
}
