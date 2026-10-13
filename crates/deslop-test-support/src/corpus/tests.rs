//! [CORPUS-CEILINGS] The harness's own plumbing, isolated.
//!
//! These pin where the harness looks for the binary it measures, which decides
//! whether a corpus test can start at all. It is invisible to every assertion
//! in `corpus_repos.rs`, because a harness that cannot launch the scan never
//! reaches them. How a measurement is read back is pinned beside the parser,
//! in `corpus_measure/tests.rs`.

use std::{env::consts::EXE_SUFFIX, path::Path};

use super::release_binary_path;

/// The binary cargo actually produces is `deslop` on Unix and `deslop.exe`
/// on Windows. Naming the bare stem makes `is_file()` false on Windows, so
/// every corpus test dies on "release binary missing" with the binary
/// sitting right there — the scan never runs and the gate reports a fault
/// that has nothing to do with the corpus.
#[test]
fn the_measured_binary_carries_the_platform_executable_suffix() {
    let path = release_binary_path();
    let expected = format!("deslop{EXE_SUFFIX}");
    assert_eq!(
        path.file_name().and_then(std::ffi::OsStr::to_str),
        Some(expected.as_str()),
        "the corpus harness must look for `{expected}` — the name cargo writes on this \
         platform — not a bare stem that only resolves on Unix. Got {}.",
        path.display()
    );
}

/// The path must still be the release profile's output, not a debug build:
/// the ceilings in every manifest were measured against optimised code.
#[test]
fn the_measured_binary_comes_from_the_release_profile() {
    let path = release_binary_path();
    assert!(
        path.parent() == Some(&repo_target_release()),
        "the corpus harness must measure `target/release`, since [CORPUS-CEILINGS] budgets \
         were measured against optimised code. Got {}.",
        path.display()
    );
}

/// `target/release` under the repository root.
fn repo_target_release() -> std::path::PathBuf {
    super::repo_root().join("target").join("release")
}

/// The harness must not be fooled into reading a directory as a binary.
#[test]
fn the_measured_binary_is_not_a_directory() {
    let path = release_binary_path();
    assert!(
        !Path::new(&path).is_dir(),
        "the corpus harness resolved a directory as the binary it measures: {}",
        path.display()
    );
}
