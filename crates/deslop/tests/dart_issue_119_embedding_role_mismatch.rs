//! E2E regression for GH #119 [CLONE-NOISE-EMBEDDING-ROLE-MISMATCH] on
//! Dart ([LANG-CAND-DART]).
//!
//! The embedding pass can pair two Dart snippets that share a topic
//! vocabulary but live in structurally incompatible constructs — a
//! `class` definition and a top-level function. Such a pair reaches
//! `structural=0.00`, `embedding_cos>=0.80`, and would surface as "Same
//! behavior, different code" even though a class and a function have no
//! safe shared extraction.
//!
//! This proves the role-compatibility gate is wired for Dart's grammar:
//! the gate re-parses each member and resolves its enclosing construct
//! via the Dart `class_declaration` / `function_declaration` node kinds.
//! Dart previously bypassed every re-parse filter (`grammar_for` had no
//! Dart arm), so the gate could never engage.
//!
//! The contract itself lives in [`crate::common::role_gate`], which every
//! language's suite shares: a per-suite copy would let one language drift
//! into asserting less than another, which is how a gate comes to be
//! covered in four places and enforced in none. What is Dart-specific is
//! here — the fixtures, and the source markers that name each role.
//!
//! Determinism: the in-process [`MockOllama`] embeds each snippet to a
//! signed feature hash of its distinct 5-byte shingles (GH #369), so the
//! near-identical role-mismatch pair clears the embedding floor on its
//! own lexical overlap. The same-role pair is genuinely Type-4 — same
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

/// Source text unique to the Dart class body in the role-mismatch
/// fixture, and to the top-level function body it must not pair with.
const DART_CLASS_MARKER: &str = "alpha = 0";
/// Source text unique to the Dart top-level function body.
const DART_FUNCTION_MARKER: &str = "saved.bind";

// GH #119 acceptance on Dart: an embedding-dominant pair whose members
// have different top-level roles must NOT surface.
#[test]
fn dart_class_function_role_mismatch_does_not_surface() -> Result<()> {
    let server = MockOllama::spawn()?;
    assert_role_mismatch_is_suppressed(
        "dart-issue-119-role-mismatch",
        "Dart",
        server.endpoint(),
        DART_CLASS_MARKER,
        DART_FUNCTION_MARKER,
    )
}

// GH #119 guard against over-suppression on Dart: two genuinely
// behaviour-equivalent FUNCTIONS (recursive vs iterative sum) share one
// top-level role, so the role gate must NOT hide them.
#[test]
fn dart_same_role_function_pair_still_surfaces() -> Result<()> {
    let server = MockOllama::spawn_semantic(&[&["totalRecursive", "totalIterative"]])?;
    assert_same_role_pair_surfaces(
        "dart-issue-119-same-role",
        "Dart",
        server.endpoint(),
        "totalRecursive",
        "while (index",
    )
}
