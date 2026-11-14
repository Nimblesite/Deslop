//! Locates a workspace binary from a test in any package.
//!
//! Cargo sets `CARGO_BIN_EXE_<name>` only for the binaries of the package
//! under test. A `deslop-mcp` test that drives `deslop-lsp` has no such
//! variable, and `assert_cmd` falls back to looking beside the test
//! executable. Cargo 1.100 moved test executables into per-package build
//! directories, so nothing sits beside them and every such test panicked
//! with "`CARGO_BIN_EXE_deslop-lsp` is unset". The profile directory that
//! holds the built binaries is still an ancestor of the test executable
//! under both layouts, so the search walks up to it.

use std::{
    env,
    path::{Path, PathBuf},
};

/// Prefix of the variable Cargo sets for a package's own binaries.
const CARGO_BIN_EXE: &str = "CARGO_BIN_EXE_";

/// Path of the workspace binary `name`.
///
/// # Panics
///
/// Panics, with `assert_cmd`'s own explanation, when the binary has not
/// been built: a test cannot run a program that does not exist.
#[must_use]
pub fn workspace_bin(name: &str) -> PathBuf {
    env::var_os(format!("{CARGO_BIN_EXE}{name}"))
        .map(PathBuf::from)
        .or_else(|| {
            env::current_exe()
                .ok()
                .and_then(|exe| built_above(&exe, name))
        })
        .unwrap_or_else(|| assert_cmd::cargo::cargo_bin(name))
}

/// The nearest ancestor directory of `exe` that holds the binary `name`.
fn built_above(exe: &Path, name: &str) -> Option<PathBuf> {
    let file = format!("{name}{}", env::consts::EXE_SUFFIX);
    exe.ancestors()
        .skip(1)
        .map(|dir| dir.join(&file))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use anyhow::Result;

    use super::built_above;

    const BINARY: &str = "deslop-lsp";

    /// The flat layout: the binary sits beside `deps/`.
    #[test]
    fn finds_the_binary_one_level_above_deps() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let profile = tmp.path().join("release");
        fs::create_dir_all(profile.join("deps"))?;
        let binary = profile.join(format!("{BINARY}{}", std::env::consts::EXE_SUFFIX));
        fs::write(&binary, "")?;
        let found = built_above(&profile.join("deps").join("suite-1234"), BINARY);
        assert_eq!(found, Some(binary));
        Ok(())
    }

    /// The per-package layout: the test executable is several levels down.
    #[test]
    fn finds_the_binary_from_a_nested_build_directory() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let profile = tmp.path().join("release");
        let nested = profile
            .join("build")
            .join("deslop-mcp")
            .join("a1b2")
            .join("deps");
        fs::create_dir_all(&nested)?;
        let binary = profile.join(format!("{BINARY}{}", std::env::consts::EXE_SUFFIX));
        fs::write(&binary, "")?;
        assert_eq!(
            built_above(&nested.join("suite-1234"), BINARY),
            Some(binary)
        );
        Ok(())
    }

    /// Nothing built: the search finds nothing rather than a wrong file.
    #[test]
    fn finds_nothing_when_the_binary_was_not_built() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let deps = tmp.path().join("release").join("deps");
        fs::create_dir_all(&deps)?;
        assert_eq!(built_above(&deps.join("suite-1234"), BINARY), None);
        Ok(())
    }
}
