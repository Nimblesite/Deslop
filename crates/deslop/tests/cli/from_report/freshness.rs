//! [OUTPUT-REPORT-REUSE] Content, membership, policy, and corrupt-entry journeys.

use std::io::Write as _;

use super::{
    super::support::*,
    reuse::{export, ANALYSIS_STARTED},
};

/// Complete cache entry is bounded to one replaceable file per workspace.
const CACHE_FILE: &str = ".deslop/cache/completed-report.bin";
/// Sources whose membership is changed independently of file contents.
const COPY: &str = "Beta.cs";
/// A newly introduced, independent source.
const ADDED: &str = "Gamma.cs";
/// Independent source avoids introducing additional clone families in this membership test.
const UNIQUE_SOURCE: &str =
    "public class Gamma { public string Name() { return \"distinct\"; } }\n";
/// Default population of the fixture.
const TWO_FILES: u64 = 2;
/// Population after adding an identical source.
const THREE_FILES: u64 = 3;
/// Population after excluding one source.
const ONE_FILE: u64 = 1;
/// Configuration used to hide one copy from discovery.
const EXCLUDE_COPY: &str = "[defaults]\nexclude = [\"Beta.cs\"]\n";

#[test]
fn completed_reports_track_membership_and_content_even_with_preserved_mtime() -> Result<()> {
    let (tmp, root) = seeded_fixture_root(CSHARP_SMALL_FIXTURE)?;
    let base = tmp.path().join(REPORT_OUTPUT_STEM);
    let _cold = assert_scan(&root, &base, TWO_FILES, true)?;
    assert_reuse_at_another_output(&root, &tmp.path().join("another-output"))?;
    fs::write(root.join(ADDED), UNIQUE_SOURCE)?;
    let added = assert_scan(&root, &base, THREE_FILES, true)?;
    assert_eq!(field(&added, "clusters").as_array().map(Vec::len), Some(1));
    assert_first_occurrences(&added, TWO_FILES);
    fs::remove_file(root.join(ADDED))?;
    let _removed = assert_scan(&root, &base, TWO_FILES, true)?;
    edit_preserving_mtime(&root.join(COPY))?;
    let _edited = assert_scan(&root, &base, TWO_FILES, true)?;
    let _reused = assert_scan(&root, &base, TWO_FILES, false)?;
    Ok(())
}

#[test]
fn completed_reports_invalidate_for_policy_corruption_and_cache_opt_out() -> Result<()> {
    let (tmp, root) = seeded_fixture_root(CSHARP_SMALL_FIXTURE)?;
    let base = tmp.path().join(REPORT_OUTPUT_STEM);
    let _cold = assert_scan(&root, &base, TWO_FILES, true)?;
    fs::write(root.join(".ignore"), "Beta.cs\n")?;
    let excluded = assert_scan(&root, &base, ONE_FILE, true)?;
    assert_eq!(field(&excluded, "clusters"), &serde_json::json!([]));
    fs::remove_file(root.join(".ignore"))?;
    let _restored = assert_scan(&root, &base, TWO_FILES, true)?;
    fs::write(root.join(".deslop.toml"), EXCLUDE_COPY)?;
    let _configured = assert_scan(&root, &base, ONE_FILE, true)?;
    fs::write(root.join(CACHE_FILE), b"truncated cache entry")?;
    let _repaired = assert_scan(&root, &base, ONE_FILE, true)?;
    let _reused = assert_scan(&root, &base, ONE_FILE, false)?;
    assert!(export(&root, &base, &[NO_INCREMENTAL_FLAG])?.contains(ANALYSIS_STARTED));
    Ok(())
}

/// The existing calculations are independent of the requested output filename.
fn assert_reuse_at_another_output(root: &Path, base: &Path) -> Result<()> {
    let report = assert_scan(root, base, TWO_FILES, false)?;
    assert_first_occurrences(&report, TWO_FILES);
    assert!(base.with_extension("html").is_file());
    assert!(base.with_extension("txt").is_file());
    Ok(())
}

/// Checks actual analysis work and the canonical report for each user action.
fn assert_scan(root: &Path, base: &Path, files: u64, recompute: bool) -> Result<Value> {
    let log = export(root, base, &[])?;
    assert_eq!(
        log.contains(ANALYSIS_STARTED),
        recompute,
        "analysis work:\n{log}"
    );
    let report = read_json_report(&base.with_extension("json"))?;
    assert_eq!(field(&report, "files_analysed"), files);
    Ok(report)
}

/// A real duplicate must survive every reused and expanded corpus.
fn assert_first_occurrences(report: &Value, expected: u64) {
    let occurrence_count = report.pointer("/clusters/0/occurrence_count");
    assert_eq!(occurrence_count, Some(&Value::from(expected)));
}

/// Content changes invalidate even when a tool preserves the source timestamp.
fn edit_preserving_mtime(path: &Path) -> Result<()> {
    let original_mtime = fs::metadata(path)?.modified()?;
    let mut file = fs::OpenOptions::new().append(true).open(path)?;
    file.write_all(b"\n")?;
    file.set_times(fs::FileTimes::new().set_modified(original_mtime))?;
    assert_eq!(fs::metadata(path)?.modified()?, original_mtime);
    Ok(())
}
