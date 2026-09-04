//! [OUTPUT-DIR] Canonical on-disk output layout for a workspace.
//!
//! Every artefact Deslop writes for a scanned workspace lands under a
//! single `.deslop/` directory at the scan root, so a user has exactly
//! one path to gitignore, inspect, or delete:
//!
//! ```text
//! <root>/
//!.deslop.toml # config — user-authored, tracked
//!.deslop/ # everything Deslop writes
//! deslop-report.{json,txt,html} # rendered reports (CLI)
//! logs/deslop-<epoch>.log # tracing sink (CLI)
//! cache/ # analysis state, never hand-edited
//!       fingerprints/ embeddings/
//!       live-report.json deslop.sock deslop.port
//! ```
//!
//! The module also owns the other half of "where a path goes": how a
//! path is *spelled* once Deslop publishes it ([`reported`]).
//!
//! The CLI, LSP, and MCP surfaces all resolve through this module, so
//! the three never disagree about where a workspace's artefacts live.
//! The CLI's `--output` flag overrides the report base (and with it the
//! log directory); nothing else is configurable, because the cache is
//! addressed by the LSP and MCP independently and must be discoverable
//! from the scan root alone.

use std::path::{Component, Path, PathBuf};

/// Name of the per-workspace output directory, relative to the scan
/// root. Dot-prefixed so the discovery pass's hidden-directory prune
/// keeps Deslop's own artefacts out of the corpus it analyses.
pub const OUTPUT_DIR_NAME: &str = ".deslop";

/// Cache subdirectory of [`OUTPUT_DIR_NAME`], holding derived analysis
/// state — fingerprints, embeddings, the live state file, and the IPC
/// endpoint artifacts. Safe to delete; everything in it is rebuildable.
pub const CACHE_DIR_NAME: &str = "cache";

/// Log subdirectory of [`OUTPUT_DIR_NAME`]. Timestamped log files
/// accumulate, so they get their own directory rather than piling up
/// alongside the reports a user actually opens.
pub const LOGS_DIR_NAME: &str = "logs";

/// Base file name, without extension, of the rendered reports. The
/// renderers append `.json`, `.txt`, and `.html`.
pub const REPORT_STEM: &str = "deslop-report";

/// Output directory for `root` — `<root>/.deslop`.
#[must_use]
pub fn output_dir(root: &Path) -> PathBuf {
    root.join(OUTPUT_DIR_NAME)
}

/// Cache directory for `root` — `<root>/.deslop/cache`.
#[must_use]
pub fn cache_dir(root: &Path) -> PathBuf {
    output_dir(root).join(CACHE_DIR_NAME)
}

/// Default report base path for `root` — `<root>/.deslop/deslop-report`.
/// Callers append the per-format extension.
#[must_use]
pub fn report_base(root: &Path) -> PathBuf {
    output_dir(root).join(REPORT_STEM)
}

/// [OUTPUT-SCHEMA-PATH-SEPARATOR] The one character Deslop puts
/// between the segments of a path it *publishes*, on every platform.
///
/// Reports are read somewhere other than the machine that wrote them —
/// a Windows developer's report opened in a Linux CI gate, two
/// platforms' reports compared in one dashboard — so a path is part of
/// the output contract and cannot be spelled the host's way. Paths
/// Deslop *opens* keep the host's own separators; only rendered ones
/// pass through [`reported`].
pub const REPORT_SEPARATOR: char = '/';

/// Respells `path` for publication: its segments joined with
/// [`REPORT_SEPARATOR`], whatever the host joined them with.
///
/// Only the separators change. A Windows drive or UNC prefix keeps its
/// own spelling, because it names a volume rather than a segment and
/// respelling it would produce something that is neither a valid
/// Windows path nor a portable one. Nothing else is rewritten: no
/// canonicalisation, no `.`/`..` collapsing, no case folding.
///
/// The result compares equal to its input as a [`Path`] on every
/// platform — Windows accepts both separators when it walks
/// components, and on Unix the function is the identity — so a
/// respelled path stays a usable map key beside paths that were not.
#[must_use]
pub fn reported(path: &Path) -> PathBuf {
    let mut spelled = String::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => spelled.push_str(&prefix.as_os_str().to_string_lossy()),
            Component::RootDir => spelled.push(REPORT_SEPARATOR),
            segment => push_segment(&mut spelled, &segment.as_os_str().to_string_lossy()),
        }
    }
    PathBuf::from(spelled)
}

/// Appends one path segment to `spelled`, separating it from whatever
/// is already there unless that ends in a separator of its own.
fn push_segment(spelled: &mut String, segment: &str) {
    if !spelled.is_empty() && !spelled.ends_with(REPORT_SEPARATOR) {
        spelled.push(REPORT_SEPARATOR);
    }
    spelled.push_str(segment);
}

/// Log directory for reports written to `report_dir` —
/// `<report_dir>/logs`. Defined relative to the report directory rather
/// than the scan root so that redirecting reports with `--output` takes
/// the logs with it.
#[must_use]
pub fn logs_dir(report_dir: &Path) -> PathBuf {
    report_dir.join(LOGS_DIR_NAME)
}

#[cfg(test)]
mod tests {
    use super::{reported, REPORT_SEPARATOR};
    use std::path::{Path, PathBuf};

    /// A workspace-relative path two directories deep — the shape every
    /// occurrence, per-file metric and folder rollup carries, spelled
    /// the one way a report is allowed to spell it.
    const NESTED: &str = "src/billing/InvoiceAlpha.cs";

    /// The byte a Windows host joins path segments with. Named by code
    /// point so the constant survives every quoting layer between a
    /// source file and a shell.
    const HOST_JOINER: u8 = 0x5C;

    /// The three segments of [`NESTED`], joined the way *this* host
    /// joins them. On Windows that is the backslash spelling a report
    /// must never carry; on Unix it is already [`NESTED`]. Building it
    /// with [`Path::join`] rather than a literal is what lets one
    /// assertion state the contract on both platforms.
    fn joined_by_the_host() -> PathBuf {
        Path::new("src").join("billing").join("InvoiceAlpha.cs")
    }

    #[test]
    fn a_host_joined_path_is_respelled_for_publication() {
        let native = joined_by_the_host();
        let spelled = reported(&native);
        assert_eq!(
            spelled,
            PathBuf::from(NESTED),
            "a path the host joined must publish as {NESTED}, whatever this host joined it with",
        );
        assert_eq!(
            spelled.to_string_lossy().matches(REPORT_SEPARATOR).count(),
            2,
            "two directory levels means two separators",
        );
        assert!(
            !spelled
                .to_string_lossy()
                .bytes()
                .any(|byte| byte == HOST_JOINER),
            "no rendered path may carry the Windows joiner",
        );
        assert_eq!(
            spelled.as_path(),
            native.as_path(),
            "a spelled path still names the same file, so it keeps working as a map key",
        );
    }

    #[test]
    fn spelling_is_idempotent_and_leaves_simple_paths_alone() {
        let once = reported(Path::new(NESTED));
        assert_eq!(once, PathBuf::from(NESTED));
        assert_eq!(reported(&once), once, "reported must be idempotent");
        assert_eq!(reported(Path::new("alpha.rs")), PathBuf::from("alpha.rs"));
        assert_eq!(reported(Path::new("")), PathBuf::new());
    }

    #[test]
    fn a_rooted_path_keeps_its_leading_separator_and_gains_no_empty_segment() {
        let rooted = Path::new("/srv").join("repo");
        assert_eq!(reported(&rooted), PathBuf::from("/srv/repo"));
        assert_eq!(
            reported(&joined_by_the_host().join("")),
            PathBuf::from(NESTED),
            "a trailing separator contributes no empty segment",
        );
    }

    // A Windows volume names a device rather than a segment: respelling it
    // would produce something that is neither a valid Windows path nor a
    // portable one, so only what follows it is respelled. The expectation is
    // the same on a host with no such concept, where the drive letter is an
    // ordinary first segment.
    #[test]
    fn a_volume_keeps_its_own_spelling_while_what_follows_it_is_respelled() {
        let absolute = Path::new("C:/repo").join("src").join("alpha.rs");
        assert_eq!(reported(&absolute), PathBuf::from("C:/repo/src/alpha.rs"));
    }

    // Spelling changes separators and nothing else. Stated as a property, it
    // holds on every host: on Unix the host joiner is an ordinary byte in a
    // file name and rewriting it would rename the file, while on Windows the
    // same bytes are two segments — and either way the count survives.
    #[test]
    fn spelling_invents_no_segment_and_loses_none() {
        let name = format!("weird{}name.rs", char::from(HOST_JOINER));
        for original in [Path::new("src").join(&name), joined_by_the_host()] {
            let spelled = reported(&original);
            assert_eq!(
                spelled.components().count(),
                original.components().count(),
                "spelling {original:?} changed how many segments it has",
            );
            assert_eq!(
                spelled.as_path(),
                original.as_path(),
                "spelling {original:?} changed which file it names",
            );
        }
    }
}
