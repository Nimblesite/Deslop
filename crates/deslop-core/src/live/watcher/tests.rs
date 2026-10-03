//! Config path lookup work and alias handling ([LIVE-WATCHER-CONFIG-COST]).

mod delivery;

use std::{cell::Cell, collections::HashSet, path::Path};

use super::watched_config_path;

/// Workspace configuration filename.
const CONFIG_NAME: &str = ".deslop.toml";
/// An existing build output unrelated to configuration.
const BUILD_NAME: &str = "compiled.tmp";
/// A removed build output unrelated to configuration.
const MISSING_NAME: &str = "removed.tmp";
/// Alternate case must not prevent filesystem-based alias resolution.
const CASE_ALIAS: &str = ".DESLOP.TOML";
/// Unicode filename comparison is deliberately left to the filesystem.
const UNICODE_ALIAS: &str = ".de\u{301}slop.toml";
/// Differently named symlink to the watched configuration.
#[cfg(unix)]
const LEAF_ALIAS: &str = "selected-configuration";
/// Symlink to the directory holding the watched configuration.
#[cfg(unix)]
const DIRECTORY_ALIAS: &str = "workspace-alias";
/// Independent directory containing a config with the same basename.
const OTHER_DIRECTORY: &str = "other-workspace";
/// Bytes are immaterial to path matching.
const CONTENTS: &str = "fixture contents";
/// Filesystem resolver calls on a path that cannot alias a configuration.
const NO_RESOLUTIONS: usize = 0;
/// One canonical resolution for an alias candidate.
const ONE_RESOLUTION: usize = 1;

/// Measures the real canonical resolver without replacing its path semantics.
fn lookup(path: &Path, config: &Path) -> (bool, usize) {
    lookup_with(path, config, |candidate| std::fs::canonicalize(candidate))
}

/// Counts one supplied resolver while exercising the production matching function.
fn lookup_with(
    path: &Path,
    config: &Path,
    resolve: impl FnOnce(&Path) -> std::io::Result<std::path::PathBuf>,
) -> (bool, usize) {
    let calls = Cell::new(NO_RESOLUTIONS);
    let watched = HashSet::from([config.to_path_buf()]);
    let matched = watched_config_path(path, &watched, |candidate| {
        calls.set(calls.get().saturating_add(ONE_RESOLUTION));
        resolve(candidate)
    });
    (matched, calls.get())
}

#[test]
fn potential_config_aliases_preserve_resolver_results() {
    let config = Path::new(CONFIG_NAME);
    let aliases = [Path::new(CASE_ALIAS), Path::new(UNICODE_ALIAS)];
    for alias in aliases {
        let accepted = lookup_with(alias, config, |_| Ok(config.to_path_buf()));
        let unrelated = lookup_with(alias, config, |_| Ok(Path::new(BUILD_NAME).to_path_buf()));
        let failed = lookup_with(alias, config, |_| Err(std::io::ErrorKind::NotFound.into()));
        assert_eq!(accepted, (true, ONE_RESOLUTION));
        assert_eq!(unrelated, (false, ONE_RESOLUTION));
        assert_eq!(failed, (false, ONE_RESOLUTION));
    }
}

#[test]
fn matching_config_basename_still_requires_the_registered_path() -> std::io::Result<()> {
    let workspace = tempfile::tempdir()?;
    let other_directory = workspace.path().join(OTHER_DIRECTORY);
    std::fs::create_dir(&other_directory)?;
    let config = workspace.path().join(CONFIG_NAME);
    let unrelated = other_directory.join(CONFIG_NAME);
    std::fs::write(&config, CONTENTS)?;
    std::fs::write(&unrelated, CONTENTS)?;
    let canonical = std::fs::canonicalize(&config)?;
    assert_eq!(lookup(&unrelated, &canonical), (false, ONE_RESOLUTION));
    assert_eq!(lookup(&canonical, &canonical), (true, NO_RESOLUTIONS));
    Ok(())
}

#[cfg(unix)]
#[test]
fn config_symlinks_keep_both_leaf_and_parent_aliases() -> std::io::Result<()> {
    let workspace = tempfile::tempdir()?;
    let config = workspace.path().join(CONFIG_NAME);
    std::fs::write(&config, CONTENTS)?;
    let canonical = std::fs::canonicalize(&config)?;
    let leaf_alias = workspace.path().join(LEAF_ALIAS);
    let parent_alias = workspace.path().join(DIRECTORY_ALIAS);
    std::os::unix::fs::symlink(&canonical, &leaf_alias)?;
    std::os::unix::fs::symlink(workspace.path(), &parent_alias)?;
    for alias in [leaf_alias, parent_alias.join(CONFIG_NAME)] {
        assert_eq!(lookup(&alias, &canonical), (true, ONE_RESOLUTION));
    }
    assert_eq!(lookup(&canonical, &canonical), (true, NO_RESOLUTIONS));
    Ok(())
}

#[test]
fn unrelated_build_paths_avoid_config_canonicalization() -> std::io::Result<()> {
    let workspace = tempfile::tempdir()?;
    let config = workspace.path().join(CONFIG_NAME);
    let build = workspace.path().join(BUILD_NAME);
    let removed = workspace.path().join(MISSING_NAME);
    std::fs::write(&build, CONTENTS)?;
    let regular_calls = if cfg!(windows) {
        ONE_RESOLUTION
    } else {
        NO_RESOLUTIONS
    };
    assert_eq!(lookup(&build, &config), (false, regular_calls));
    assert_eq!(lookup(&removed, &config), (false, NO_RESOLUTIONS));
    assert_eq!(lookup(&config, &config), (true, NO_RESOLUTIONS));
    Ok(())
}
