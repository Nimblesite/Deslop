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
        shared::{build_normalised_root, intern_kind, parse_source, IDENTIFIER_KIND, LITERAL_KIND},
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
    match raw {
        // Drop trivia — [PIPELINE-NORMALIZE-AST]
        "comment" | "doc_comment" => None,
        // Identifier-like tokens — collapse for Type-2 renamed-clone detection
        "name" => Some(IDENTIFIER_KIND),
        // Literals — collapse so constant edits do not perturb fingerprints
        "integer" | "float" | "boolean" | "null" | "string" | "encapsed_string" | "heredoc"
        | "nowdoc" => Some(LITERAL_KIND),
        // All structural kinds pass through unchanged
        other => Some(intern_kind(other)),
    }
}
