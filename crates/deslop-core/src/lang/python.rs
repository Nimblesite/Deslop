//! Python language plugin.
//!
//! Implements [PIPELINE-LANG-TRAIT] for Python using the
//! `tree-sitter-python` grammar. Normalisation rules match the
//! Type-2-invariance principle from the other plug-ins
//! ([CLONE-TYPE-TAXONOMY]): identifiers collapse to `__ident__`,
//! literals (including the `true` / `false` / `none` keywords that
//! tree-sitter exposes as named leaf nodes) collapse to `__literal__`,
//! and `comment` nodes are dropped. Shared walking / interning plumbing
//! lives in [`super::shared`].

use crate::{
    ast::NormalizedNode,
    error::CoreError,
    lang::{
        shared::{build_normalised_root, intern_kind, parse_source, IDENTIFIER_KIND, LITERAL_KIND},
        LanguageParser,
    },
    state::FileId,
};

/// Stable language identifier reported by [`PythonParser::id`].
const LANGUAGE_ID: &str = "python";

/// Python implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct PythonParser;

impl PythonParser {
    /// Creates a new parser. Stateless — safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for PythonParser {
    fn id(&self) -> &'static str {
        LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["py"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_python::LANGUAGE.into()
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

/// Maps a tree-sitter Python node kind to its normalised form. Covers
/// the identifier / literal / trivia families emitted by
/// `tree-sitter-python` 0.25.x.
fn normalise_kind(raw: &str) -> Option<&'static str> {
    match raw {
        "comment" => None,
        "identifier" | "type" => Some(IDENTIFIER_KIND),
        "string"
        | "concatenated_string"
        | "integer"
        | "float"
        | "true"
        | "false"
        | "none"
        | "ellipsis" => Some(LITERAL_KIND),
        other => Some(intern_kind(other)),
    }
}
