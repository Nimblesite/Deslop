//! Language parser plugins.
//!
//! Implements [PIPELINE-LANG-TRAIT]. Each language implementation provides a
//! tree-sitter grammar, a file-extension filter, and per-language
//! normalisation rules that flatten identifier / literal / trivia nodes into
//! their structural kind. The trait output — [`NormalizedNode`] — is
//! identical across languages so downstream stages never branch on language.

use tree_sitter::Language;

use crate::{ast::NormalizedNode, error::CoreError, state::FileId};

pub mod csharp;
pub mod python;
pub mod rust_lang;
pub mod shared;

/// A language plugin. One instance per language per pipeline run.
pub trait LanguageParser: std::fmt::Debug + Send + Sync {
    /// Stable identifier for this language (`"csharp"`, `"rust"`, ...).
    fn id(&self) -> &'static str;

    /// Source file extensions handled by this parser, lowercase and without
    /// leading `.` (e.g. `&["cs"]` for C#).
    fn file_extensions(&self) -> &'static [&'static str];

    /// Returns the tree-sitter grammar used by [`Self::parse_and_normalize`].
    fn grammar(&self) -> Language;

    /// Parses `source` and returns a normalised AST rooted at the file. All
    /// subtrees carry `file_id`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::GrammarLoad`] if the tree-sitter grammar cannot
    /// be applied to the parser, and [`CoreError::ParseFailed`] if the
    /// parser did not produce a tree.
    fn parse_and_normalize(
        &self,
        source: &[u8],
        file_id: FileId,
    ) -> Result<NormalizedNode, CoreError>;
}
