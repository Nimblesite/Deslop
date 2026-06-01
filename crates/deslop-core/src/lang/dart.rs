//! Dart language plugin.
//!
//! Implements [PIPELINE-LANG-TRAIT] for Dart using the
//! `tree-sitter-dart` grammar (nielsenko fork, Dart 3.x: records,
//! patterns, class modifiers, extension types, null-aware elements).
//! Normalisation follows the same Type-2-invariance principle as the
//! other plug-ins ([CLONE-TYPE-TAXONOMY]):
//!
//! - `identifier`, `identifier_dollar_escaped`, `type_identifier` →
//!   `"__ident__"` so renamed variables / type names hash identically.
//!   Structural wrappers (`type`, `extension_type_name`, `typed_identifier`,
//!   `type_parameter`) pass through so generic / annotation shape survives.
//! - Every numeric / boolean / `null` / symbol literal and every string
//!   form (single / double / multiline / raw quote variants and their
//!   `template_chars_*` text chunks) → `"__literal__"`. Collapsing the
//!   outer string node makes `'x'` and `"x"` fingerprint identically while
//!   `template_substitution` interpolation expressions stay structural.
//! - `comment`, `block_comment`, `documentation_block_comment` are dropped.
//! - All other named node kinds pass through with their grammar name.
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

/// Stable language identifier reported by [`DartParser::id`].
const LANGUAGE_ID: &str = "dart";

/// Dart implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct DartParser;

impl DartParser {
    /// Creates a new parser. Stateless — safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for DartParser {
    fn id(&self) -> &'static str {
        LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["dart"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_dart::LANGUAGE.into()
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

/// Maps a tree-sitter Dart node kind to its normalised form. Returns
/// `None` when the node should be dropped entirely (pure trivia). The
/// returned `&'static str` comes from a fixed placeholder set or is
/// interned on first sight so downstream hashing is cheap and stable.
fn normalise_kind(raw: &str) -> Option<&'static str> {
    match raw {
        "comment" | "block_comment" | "documentation_block_comment" => None,
        "identifier" | "identifier_dollar_escaped" | "type_identifier" => Some(IDENTIFIER_KIND),
        raw if is_literal_kind(raw) => Some(LITERAL_KIND),
        other => Some(intern_kind(other)),
    }
}

/// Returns true when `raw` is a Dart literal node collapsed by
/// normalisation — every numeric / boolean / `null` / symbol scalar and
/// every string form (including the `template_chars_*` text chunks).
/// Collapsing the outer string node makes `'x'` and `"x"` fingerprint
/// identically regardless of quote style, escaping, or value, while
/// `template_substitution` interpolation expressions stay structural.
#[must_use]
pub(crate) fn is_literal_kind(raw: &str) -> bool {
    is_scalar_literal_kind(raw) || is_string_literal_kind(raw)
}

/// Returns true for Dart numeric, boolean, `null`, and symbol literal
/// node kinds.
fn is_scalar_literal_kind(raw: &str) -> bool {
    matches!(
        raw,
        "decimal_integer_literal"
            | "hex_integer_literal"
            | "decimal_floating_point_literal"
            | "true"
            | "false"
            | "null_literal"
            | "symbol_literal"
    )
}

/// Returns true for every Dart string node kind — all quote/raw/multiline
/// variants and their `template_chars_*` text chunks.
fn is_string_literal_kind(raw: &str) -> bool {
    matches!(
        raw,
        "string_literal"
            | "string_literal_single_quotes"
            | "string_literal_single_quotes_multiple"
            | "string_literal_double_quotes"
            | "string_literal_double_quotes_multiple"
            | "raw_string_literal_single_quotes"
            | "raw_string_literal_single_quotes_multiple"
            | "raw_string_literal_double_quotes"
            | "raw_string_literal_double_quotes_multiple"
            | "template_chars_single"
            | "template_chars_single_single"
            | "template_chars_double"
            | "template_chars_double_single"
            | "template_chars_raw_slash"
    )
}
