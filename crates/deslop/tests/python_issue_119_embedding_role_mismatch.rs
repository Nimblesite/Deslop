//! E2E regression for GH #119 [CLONE-NOISE-EMBEDDING-ROLE-MISMATCH] on
//! Python.
//!
//! The embedding pass can pair two snippets that share a topic vocabulary
//! but live in structurally incompatible constructs — a reusable helper
//! *class* and a constructor-storage *test method*. Such a pair reaches
//! `structural=0.00`, `embedding_cos>=0.80`, and would surface as "Same
//! behavior, different code" even though a class definition and a
//! function have no safe shared extraction.
//!
//! The contract itself lives in [`crate::common::role_gate`], which every
//! language's suite shares: a per-suite copy would let one language drift
//! into asserting less than another, which is how a gate comes to be
//! covered in four places and enforced in none. What is Python-specific
//! is here — the fixtures, and the source markers that name each role.
//!
//! Determinism: the in-process [`MockOllama`] embeds each snippet to a
//! signed feature hash of its distinct 5-byte shingles (GH #369, replacing
//! the length-residue vector of GH #366), so cosine tracks *lexical*
//! overlap and the near-identical role-mismatch pair clears the embedding
//! floor on that alone. The same-role pair is genuinely Type-4 — same
//! behaviour, different text — which no content statistic can measure, so
//! the test declares that ground truth with
//! [`MockOllama::spawn_semantic`]: snippets naming either function are
//! behaviour-equivalent, which lifts their cosine above the floor while
//! unrelated snippets keep their honest shingle cosine.

#[path = "cli/mock_ollama.rs"]
mod mock_ollama;

use anyhow::Result;
use mock_ollama::MockOllama;

mod common;
use crate::common::role_gate::*;

/// Source text unique to the Python helper-class body in the
/// role-mismatch fixture.
const PYTHON_CLASS_MARKER: &str = "alpha = 0";
/// Source text unique to the Python test-function body it must not pair
/// with.
const PYTHON_FUNCTION_MARKER: &str = "saved.bind";

// GH #119 acceptance: an embedding-dominant pair whose members have
// different top-level roles must NOT surface.
#[test]
fn class_function_role_mismatch_does_not_surface() -> Result<()> {
    let server = MockOllama::spawn()?;
    assert_role_mismatch_is_suppressed(
        "python-issue-119-role-mismatch",
        "Python",
        server.endpoint(),
        PYTHON_CLASS_MARKER,
        PYTHON_FUNCTION_MARKER,
    )
}

// GH #119 guard against over-suppression: two genuinely behaviour-
// equivalent FUNCTIONS (recursive vs iterative sum) share one top-level
// role, so the role gate must NOT hide them.
#[test]
fn same_role_function_pair_still_surfaces() -> Result<()> {
    let server = MockOllama::spawn_semantic(&[&["total_recursive", "total_iterative"]])?;
    assert_same_role_pair_surfaces(
        "python-issue-119-same-role",
        "Python",
        server.endpoint(),
        "total_recursive",
        "while index",
    )
}
