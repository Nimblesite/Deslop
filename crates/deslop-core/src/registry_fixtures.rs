//! Two-file [`FileRegistry`] fixtures for pair-measurement suites
//! ([STATE-FILE-REGISTRY]).
//!
//! Fourteen sites across this crate opened with the same three lines:
//! construct a registry, register a left-hand path, register a
//! right-hand path. Every copy spelled its two paths as bare string
//! literals, so the fixture corpus was scattered across six files with
//! no way to see which suites shared a shape. The detector reported the
//! run as its own worst cluster (mass 468, fourteen occurrences).
//!
//! One constructor instead, and the paths are named. A suite that wants
//! the ordinary `left`/`right` pair asks for it by name; a suite whose
//! measurement depends on the specific file names — the absent-file
//! lookup, the wide-function overlap, the F# fold windows — names those
//! too, so the reason a test uses an unusual path stays legible.
//!
//! Compiled only for tests, on the same gate as [`crate::report_fixtures`]:
//! this crate's own `#[cfg(test)]` modules reach it directly, and its
//! integration suites enable it through the `test-support` feature
//! already carried in the dev-dependencies.

use std::path::PathBuf;

use crate::state::{FileId, FileRegistry};

/// Left-hand member of the ordinary Rust pair.
pub const LEFT_RS: &str = "left.rs";

/// Right-hand member of the ordinary Rust pair.
pub const RIGHT_RS: &str = "right.rs";

/// A Rust path deliberately never registered's counterpart: registered
/// here so a lookup for it resolves, while the suite asserts on a file
/// the measurement cannot reach.
pub const ABSENT_RS: &str = "absent.rs";

/// Left-hand member of the wide-function overlap pair, whose bodies are
/// generated wide enough to cross the shared-subtree floor.
pub const WIDE_LEFT_RS: &str = "wide_left.rs";

/// Right-hand member of the wide-function overlap pair.
pub const WIDE_RIGHT_RS: &str = "wide_right.rs";

/// The fingerprint-cache entry a lookup is served from.
pub const STORED_RS: &str = "stored.rs";

/// The fingerprint-cache entry a lookup asks for, distinct from
/// [`STORED_RS`] so a cache hit across file identities is visible.
pub const REQUESTED_RS: &str = "requested.rs";

/// Shorter F# fold window in the signature fold-parity pair.
pub const WINDOW_A_FS: &str = "window_a.fs";

/// Longer F# fold window in the signature fold-parity pair.
pub const WINDOW_B_FS: &str = "window_b.fs";

/// First F# module in the repeated-window fold-parity pair.
pub const REPEAT_A_FS: &str = "repeat_a.fs";

/// Second F# module in the repeated-window fold-parity pair.
pub const REPEAT_B_FS: &str = "repeat_b.fs";

/// Left-hand member of the ordinary Python pair.
pub const LEFT_PY: &str = "left.py";

/// Right-hand member of the ordinary Python pair.
pub const RIGHT_PY: &str = "right.py";

/// Left-hand member of the LSH-uniqueness Python pair.
pub const LSH_LEFT_PY: &str = "lsh_left.py";

/// Right-hand member of the LSH-uniqueness Python pair.
pub const LSH_RIGHT_PY: &str = "lsh_right.py";

/// Registers `left` then `right` in a fresh registry, returning it with
/// both freshly allocated ids.
///
/// Registration order is the contract: `left` always receives the lower
/// [`FileId`], which the pair orderings downstream of a measurement
/// depend on ([STATE-FILE-REGISTRY]). Use this when the suite registers
/// further files; [`pair_ids`] covers the plain two-file case.
#[must_use]
pub fn registry_pair(left: &str, right: &str) -> (FileRegistry, FileId, FileId) {
    let mut registry = FileRegistry::new();
    let left_id = registry.register(PathBuf::from(left));
    let right_id = registry.register(PathBuf::from(right));
    (registry, left_id, right_id)
}

/// The two ids [`registry_pair`] issues for `left` and `right`, for the
/// suites that measure over ids alone and never resolve a path back.
#[must_use]
pub fn pair_ids(left: &str, right: &str) -> (FileId, FileId) {
    let (_registry, left_id, right_id) = registry_pair(left, right);
    (left_id, right_id)
}

/// Ids for the ordinary [`LEFT_RS`] / [`RIGHT_RS`] pair, for suites
/// whose measurement does not depend on the file names.
#[must_use]
pub fn rust_pair_ids() -> (FileId, FileId) {
    pair_ids(LEFT_RS, RIGHT_RS)
}

/// Ids for the ordinary [`LEFT_PY`] / [`RIGHT_PY`] pair, for suites
/// whose measurement does not depend on the file names.
#[must_use]
pub fn python_pair_ids() -> (FileId, FileId) {
    pair_ids(LEFT_PY, RIGHT_PY)
}
