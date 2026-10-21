//! [LIVE-WATCHER-DIRECTORIES] Conservative alias delivery never widens the corpus.

use super::{
    directories::{
        assert_clone, assert_membership, CLONE_FILES, FIRST_SOURCE, FIXTURE, SECOND_SOURCE,
    },
    *,
};

/// Sibling sources are outside the analyzed workspace.
const EXTERNAL_PREFIX: &str = "external-sources";
/// The OS rename event does not carry a supported language suffix.
const ORIGINAL_ALIAS: &str = "original-link.tmp";
/// Final unsupported alias name delivered by the watcher.
const RENAMED_ALIAS: &str = "renamed-link.tmp";
/// The original cold scan's entire admitted corpus.
const EXPECTED_PATHS: &[&str] = &[FIRST_SOURCE, SECOND_SOURCE];

/// Keeps the outside target alive for both initial discovery and the rename event.
fn alias_fixture() -> Result<(tempfile::TempDir, tempfile::TempDir)> {
    let root = copy_fixture(FIXTURE)?;
    let external = tempfile::Builder::new().prefix(EXTERNAL_PREFIX).tempdir()?;
    let source = external.path().join(SECOND_SOURCE);
    fs::write(&source, fs::read(root.path().join(SECOND_SOURCE))?)?;
    std::os::unix::fs::symlink(source, root.path().join(ORIGINAL_ALIAS))?;
    Ok((root, external))
}

/// Pins clone identity, scope, and every rendered field to the original cold report.
fn assert_original_report(
    report: &deslop_core::Report,
    original: &deslop_core::Report,
) -> Result<()> {
    assert_clone(report, EXPECTED_PATHS, CLONE_FILES)?;
    assert_membership(report, EXPECTED_PATHS);
    assert_eq!(
        serde_json::to_value(report)?,
        serde_json::to_value(original)?
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn renamed_external_source_alias_preserves_the_complete_cold_report() -> Result<()> {
    let (root, _external) = alias_fixture()?;
    let mut session = live_session(root.path())?;
    let original = session.report();
    assert_clone(&original, EXPECTED_PATHS, CLONE_FILES)?;
    let renamed = root.path().join(RENAMED_ALIAS);
    fs::rename(root.path().join(ORIGINAL_ALIAS), &renamed)?;
    let _delta = session.apply_changes(&[renamed])?;
    assert_original_report(&session.report(), &original)
}
