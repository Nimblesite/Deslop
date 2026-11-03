//! [OUTPUT-REPORT-REUSE] Export formats share completed, current calculations.

use super::super::support::*;
use anyhow::Context as _;

/// The trace emitted when the CLI starts analysis rather than rendering a snapshot.
pub(super) const ANALYSIS_STARTED: &str = "pipeline session initialising";
/// Expected source-file population before deleting a copy.
const INITIAL_FILES: u64 = 2;
/// Expected source-file population after deleting a copy.
const REMAINING_FILES: u64 = 1;
/// The only duplicated group is ranked first.
const FIRST_RANK: u64 = 1;
/// The source removed to invalidate the report cache.
const DELETED_COPY: &str = "Beta.cs";
/// Rendering-only options must not invalidate completed calculations.
const FORMAT_OPTIONS: &[&[&str]] = &[&["--notext"], &["--nohtml"], &["--nojson"]];

#[test]
fn unchanged_exports_reuse_calculations_and_source_changes_invalidate_them() -> Result<()> {
    let (tmp, root) = seeded_fixture_root(CSHARP_SMALL_FIXTURE)?;
    let base = tmp.path().join(REPORT_OUTPUT_STEM);
    let cold_log = export(&root, &base, &[])?;
    assert!(
        cold_log.contains(ANALYSIS_STARTED),
        "cold scan must analyse"
    );
    let original = read_json_report(&base.with_extension("json"))?;
    assert_populated_report(&original)?;
    assert_unchanged_exports(&root, &base, &original)?;
    assert_deleted_copy_invalidates(&root, &base)
}

/// A changed source must replace the calculations, and that result is reusable too.
fn assert_deleted_copy_invalidates(root: &Path, base: &Path) -> Result<()> {
    fs::remove_file(root.join(DELETED_COPY))?;
    let changed_log = export(root, base, &[])?;
    assert!(
        changed_log.contains(ANALYSIS_STARTED),
        "deletion must invalidate calculations"
    );
    let changed = read_json_report(&base.with_extension("json"))?;
    assert_eq!(field(&changed, "files_analysed"), REMAINING_FILES);
    assert_eq!(field(&changed, "clusters"), &serde_json::json!([]));
    assert_unchanged_exports(root, base, &changed)
}

/// Drives the actual CLI and captures analysis traces without timing assertions.
pub(super) fn export(root: &Path, base: &Path, formats: &[&str]) -> Result<String> {
    let mut command = deslop_command(root, base)?;
    let assertion = command
        .env("RUST_LOG", "info")
        .args([
            MIN_NODES_FLAG,
            MIN_NODES_VALUE,
            NO_COLOR_FLAG,
            "--log-to-console",
        ])
        .args(formats)
        .assert()
        .success();
    Ok(String::from_utf8(assertion.get_output().stderr.clone())?)
}

/// Pins non-empty detector output before testing reuse, so blindness cannot pass.
fn assert_populated_report(report: &Value) -> Result<()> {
    assert_eq!(field(report, "files_analysed"), INITIAL_FILES);
    let clusters = field(report, "clusters")
        .as_array()
        .context("cluster array")?;
    let first = clusters.first().context("real duplicated C# methods")?;
    assert_eq!(field(first, "rank"), FIRST_RANK);
    let occurrences = field(first, "occurrences")
        .as_array()
        .context("occurrences")?;
    assert_eq!(occurrences.len(), usize::try_from(INITIAL_FILES)?);
    let paths: std::collections::BTreeSet<_> = occurrences
        .iter()
        .map(|occurrence| field(occurrence, "path").as_str().context("source path"))
        .collect::<Result<_>>()?;
    assert_eq!(paths, ["Alpha.cs", DELETED_COPY].into_iter().collect());
    Ok(())
}

/// Each format renders the same snapshot without another pipeline initialisation.
fn assert_unchanged_exports(root: &Path, base: &Path, expected: &Value) -> Result<()> {
    for formats in FORMAT_OPTIONS {
        let log = export(root, base, formats)?;
        assert!(
            !log.contains(ANALYSIS_STARTED),
            "unchanged report export must reuse completed calculations; analysis restarted:\n{log}"
        );
        let actual = read_json_report(&base.with_extension("json"))?;
        assert_eq!(
            &actual, expected,
            "all calculated report fields must be preserved"
        );
        assert!(fs::read_to_string(base.with_extension("html"))?.contains("Deslop report"));
        assert_ne!(fs::read_to_string(base.with_extension("txt"))?, "");
    }
    Ok(())
}
