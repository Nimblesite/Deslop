//! JavaScript language plugin.
//!
//! Implements [PIPELINE-LANG-TRAIT] for JavaScript and JSX using the
//! `tree-sitter-javascript` grammar. Normalisation is shared with
//! TypeScript in [`super::ecmascript`] so JS and TS differ only by the
//! grammar entry point and extension set ([LANG-CAND-JAVASCRIPT]).

use crate::{
    ast::NormalizedNode,
    error::CoreError,
    lang::{ecmascript::parse_and_normalise, LanguageParser},
    state::FileId,
};

/// Stable language identifier reported by [`JavaScriptParser::id`].
const LANGUAGE_ID: &str = "javascript";

/// JavaScript implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct JavaScriptParser;

impl JavaScriptParser {
    /// Creates a new parser. Stateless, so it is safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for JavaScriptParser {
    fn id(&self) -> &'static str {
        LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["js", "mjs", "cjs", "jsx"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    fn parse_and_normalize(
        &self,
        source: &[u8],
        file_id: FileId,
    ) -> Result<NormalizedNode, CoreError> {
        parse_and_normalise(LANGUAGE_ID, &self.grammar(), source, file_id)
    }
}
