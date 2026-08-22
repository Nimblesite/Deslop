//! [CORPUS-CEILINGS] The harness's own plumbing, isolated.
//!
//! These pin the two things that decide whether a corpus test can start at
//! all: where it looks for the binary it measures, and how it reads a peak
//! RSS back out of the tool that measured it. Both are invisible to every
//! assertion in `corpus_repos.rs`, because a harness that cannot launch the
//! scan never reaches them.

use std::{env::consts::EXE_SUFFIX, path::Path};

use super::{peak_rss_mb, release_binary_path};

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

/// GNU `/usr/bin/time -v` reports kbytes, and the label says so.
#[test]
fn a_gnu_peak_is_read_as_kbytes() {
    let stderr = "\tMaximum resident set size (kbytes): 7340032\n";
    let parsed = peak_rss_mb(stderr).expect("GNU peak parses");
    assert_eq!(
        parsed, 7168,
        "7340032 kbytes is 7168 MB, the runner ceiling"
    );
}

/// BSD `/usr/bin/time -l` reports bytes, with no unit in the label.
#[test]
fn a_bsd_peak_is_read_as_bytes() {
    let stderr = "         7516192768  maximum resident set size\n";
    let parsed = peak_rss_mb(stderr).expect("BSD peak parses");
    assert_eq!(
        parsed, 7168,
        "7516192768 bytes is 7168 MB, the runner ceiling"
    );
}

/// A measurement that never appeared is an error, never a zero: a silent
/// zero would clear every memory ceiling in the corpus at once.
#[test]
fn a_missing_peak_is_an_error_rather_than_zero() {
    let error = peak_rss_mb("nothing useful here\n")
        .expect_err("a stderr with no peak line must not yield a number");
    assert!(
        error.to_string().contains("maximum resident set size"),
        "the error must name what it could not find, got: {error}"
    );
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
