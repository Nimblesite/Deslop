//! PHP language plugin.
//!
//! Implements [PIPELINE-LANG-TRAIT] for PHP using the `tree-sitter-php`
//! grammar. Normalisation follows the Type-2-invariance principle
//! ([CLONE-TYPE-TAXONOMY]):
//!
//! - `name` → `"__ident__"` so renamed functions, classes, and bare
//!   identifiers hash identically. `variable_name` is structural
//!   (wraps a `name` child via `$ { ... }`), so the `$` sigil is
//!   anonymous and does not appear in named children — `name` alone
//!   carries the identifier.
//! - `integer`, `float`, `boolean`, `null`, `string`,
//!   `encapsed_string`, `heredoc`, `nowdoc` → `"__literal__"` so
//!   constant edits do not perturb fingerprints. The outer literal
//!   node is renamed; child nodes (e.g. `string_value`, interpolation
//!   expressions) are still walked and normalised.
//! - `comment`, `doc_comment` are dropped.
//! - All other named node kinds pass through unchanged.
//!
//! Shared walking / interning plumbing lives in [`super::shared`].

use crate::{
    ast::NormalizedNode,
    error::CoreError,
    lang::{
        shared::{build_normalised_root, normalise_kind_with, parse_source},
        LanguageParser,
    },
    state::FileId,
};

/// Stable language identifier reported by [`PhpParser::id`].
const LANGUAGE_ID: &str = "php";

/// PHP implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct PhpParser;

impl PhpParser {
    /// Creates a new parser. Stateless — safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for PhpParser {
    fn id(&self) -> &'static str {
        LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["php"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_php::LANGUAGE_PHP.into()
    }

    fn parse_and_normalize(
        &self,
        source: &[u8],
        file_id: FileId,
    ) -> Result<NormalizedNode, CoreError> {
        let tree = parse_source(LANGUAGE_ID, &self.grammar(), source)?;
        build_normalised_root(&tree, file_id, normalise_kind, LANGUAGE_ID)
    }
}

/// Maps a tree-sitter PHP node kind to its normalised form. Covers the
/// identifier / literal / trivia families emitted by `tree-sitter-php`
/// 0.24.x ([PARSE-PHP-NORMALIZE]).
fn normalise_kind(raw: &str) -> Option<&'static str> {
    normalise_kind_with(raw, is_comment_kind, is_identifier_kind, is_literal_kind)
}

/// PHP trivia.
fn is_comment_kind(raw: &str) -> bool {
    matches!(raw, "comment" | "doc_comment")
}

/// PHP identifier-like tokens, collapsed for Type-2 renamed-clone
/// detection.
fn is_identifier_kind(raw: &str) -> bool {
    matches!(raw, "name")
}

/// PHP literal leaves, collapsed so constant edits do not perturb
/// fingerprints.
fn is_literal_kind(raw: &str) -> bool {
    matches!(
        raw,
        "integer"
            | "float"
            | "boolean"
            | "null"
            | "string"
            | "encapsed_string"
            | "heredoc"
            | "nowdoc"
    )
}
