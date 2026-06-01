//! C# language plugin.
//!
//! Implements [PIPELINE-LANG-TRAIT] for C# using the `tree-sitter-c-sharp`
//! grammar. Normalisation rules (mapping to [PIPELINE-NORMALIZE-AST]):
//!
//! - `identifier`, `predefined_type`, `type_parameter` → collapsed to
//!   `"__ident__"` so renamed variables / type names hash identically
//!   (Type-2 invariance per [CLONE-TYPE-TAXONOMY]).
//! - String / verbatim-string / interpolated-string / integer / real /
//!   character / boolean / null literals → collapsed to `"__literal__"`
//!   so changed constants do not perturb the fingerprint.
//! - `comment` nodes are dropped.
//! - All other named node kinds pass through with their grammar name
//!   preserved.
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

/// Stable language identifier reported by [`CSharpParser::id`].
const LANGUAGE_ID: &str = "csharp";

/// C# implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct CSharpParser;

impl CSharpParser {
    /// Creates a new parser. Stateless — safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for CSharpParser {
    fn id(&self) -> &'static str {
        LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["cs"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_c_sharp::LANGUAGE.into()
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

/// Maps a tree-sitter C# node kind to its normalised form. Returns `None`
/// when the node should be dropped entirely (pure trivia). The returned
/// `&'static str` comes from a fixed placeholder set or is interned on
/// first sight so downstream hashing is cheap and stable.
fn normalise_kind(raw: &str) -> Option<&'static str> {
    match raw {
        "comment" => None,
        "identifier" | "predefined_type" | "type_parameter" => Some(IDENTIFIER_KIND),
        raw if is_literal_kind(raw) => Some(LITERAL_KIND),
        other => Some(intern_kind(other)),
    }
}

/// Returns true when `raw` is a C# literal node collapsed by normalisation.
#[must_use]
pub(crate) fn is_literal_kind(raw: &str) -> bool {
    matches!(
        raw,
        "string_literal"
            | "verbatim_string_literal"
            | "interpolated_string_text"
            | "integer_literal"
            | "real_literal"
            | "character_literal"
            | "boolean_literal"
            | "null_literal"
    )
}
