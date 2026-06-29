//! TypeScript and TSX language plugins.
//!
//! Implements [PIPELINE-LANG-TRAIT] for TypeScript and TSX using the
//! two grammar entry points exposed by `tree-sitter-typescript`.
//! Normalisation is shared with JavaScript in [`super::ecmascript`],
//! keeping the JS/TS surface deliberately small
//! ([LANG-CAND-TYPESCRIPT], [LANG-CAND-JAVASCRIPT]).

use crate::{
    ast::NormalizedNode,
    error::CoreError,
    lang::{ecmascript::parse_and_normalise, LanguageParser},
    state::FileId,
};

/// Stable language identifier reported by [`TypeScriptParser::id`].
const TYPESCRIPT_LANGUAGE_ID: &str = "typescript";
/// Stable language identifier reported by [`TsxParser::id`].
const TSX_LANGUAGE_ID: &str = "tsx";

/// TypeScript implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct TypeScriptParser;

impl TypeScriptParser {
    /// Creates a new parser. Stateless, so it is safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for TypeScriptParser {
    fn id(&self) -> &'static str {
        TYPESCRIPT_LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["ts"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn parse_and_normalize(
        &self,
        source: &[u8],
        file_id: FileId,
    ) -> Result<NormalizedNode, CoreError> {
        parse_and_normalise(TYPESCRIPT_LANGUAGE_ID, &self.grammar(), source, file_id)
    }
}

/// TSX implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct TsxParser;

impl TsxParser {
    /// Creates a new parser. Stateless, so it is safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for TsxParser {
    fn id(&self) -> &'static str {
        TSX_LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["tsx"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    }

    fn parse_and_normalize(
        &self,
        source: &[u8],
        file_id: FileId,
    ) -> Result<NormalizedNode, CoreError> {
        parse_and_normalise(TSX_LANGUAGE_ID, &self.grammar(), source, file_id)
    }
}
