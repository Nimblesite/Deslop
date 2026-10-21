//! [LIVE-SCHEDULER-REMOVAL-COST] Missing paths do only useful resolution work.

use std::path::{Path, PathBuf};

use super::canonicalise_path;

const MISSING_FILE: &str = "removed.tmp";
const MISSING_DIRECTORY: &str = "removed-directory";
const EXISTING_FILE: &str = "present.rs";
const EMPTY_SOURCE: &str = "";
const NO_RESOLUTIONS: usize = 0;
const ONE_RESOLUTION: usize = 1;
#[cfg(unix)]
const TWO_RESOLUTIONS: usize = 2;
#[cfg(unix)]
const SOURCE_ALIAS: &str = "source-alias";
#[cfg(unix)]
const BROKEN_ALIAS: &str = "broken-alias";

fn resolve_counted(path: &Path) -> (PathBuf, usize) {
    let mut calls = NO_RESOLUTIONS;
    let resolved = canonicalise_path(path, |candidate| {
        calls = calls.saturating_add(ONE_RESOLUTION);
        std::fs::canonicalize(candidate)
    });
    (resolved, calls)
}

#[cfg(unix)]
fn assert_symlink_resolution(root: &Path, target: &Path, expected: &Path) -> std::io::Result<()> {
    let alias = root.join(SOURCE_ALIAS);
    let broken = root.join(BROKEN_ALIAS);
    std::os::unix::fs::symlink(target, &alias)?;
    std::os::unix::fs::symlink(root.join(MISSING_FILE), &broken)?;
    assert_eq!(
        resolve_counted(&alias),
        (expected.to_path_buf(), ONE_RESOLUTION)
    );
    let expected_broken = std::fs::canonicalize(root)?.join(BROKEN_ALIAS);
    assert_eq!(resolve_counted(&broken), (expected_broken, TWO_RESOLUTIONS));
    Ok(())
}

#[test]
fn missing_source_resolves_its_surviving_parent_once() -> std::io::Result<()> {
    let workspace = tempfile::tempdir()?;
    let missing = workspace.path().join(MISSING_FILE);
    let expected = std::fs::canonicalize(workspace.path())?.join(MISSING_FILE);
    let (resolved, calls) = resolve_counted(&missing);
    assert_eq!(resolved, expected, "parent aliases must still resolve");
    assert_eq!(
        calls, ONE_RESOLUTION,
        "a missing leaf must not repeat its parent's realpath walk"
    );
    Ok(())
}

#[test]
fn existing_source_resolves_the_complete_path_once() -> std::io::Result<()> {
    let workspace = tempfile::tempdir()?;
    let path = workspace.path().join(EXISTING_FILE);
    std::fs::write(&path, EMPTY_SOURCE)?;
    let expected = std::fs::canonicalize(&path)?;
    assert_eq!(resolve_counted(&path), (expected.clone(), ONE_RESOLUTION));
    #[cfg(unix)]
    assert_symlink_resolution(workspace.path(), &path, &expected)?;
    Ok(())
}

#[test]
fn absent_parent_preserves_the_original_removal_reference() -> std::io::Result<()> {
    let workspace = tempfile::tempdir()?;
    let path = workspace.path().join(MISSING_DIRECTORY).join(MISSING_FILE);
    let (resolved, calls) = resolve_counted(&path);
    assert_eq!(resolved, path);
    assert_eq!(calls, ONE_RESOLUTION);
    Ok(())
}
