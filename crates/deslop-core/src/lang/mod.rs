//! Language parser plugins.
//!
//! Implements [PIPELINE-LANG-TRAIT]. Each language implementation provides a
//! tree-sitter grammar, a file-extension filter, and per-language
//! normalisation rules that flatten identifier / literal / trivia nodes into
//! their structural kind. The trait output — [`NormalizedNode`] — is
//! identical across languages so downstream stages never branch on language.

use tree_sitter::Language;

use crate::{
    ast::NormalizedNode,
    error::CoreError,
    refactor::{
        emit::{EmitOutcome, EmitRequest},
        merge::{MergeEmitOutcome, MergeEmitRequest},
        tables::{BindingKind, MergeTables, ReferenceTable, ScopeKinds, EMPTY_REFERENCE_TABLE},
    },
    state::FileId,
};

pub mod csharp;
mod csharp_merge;
pub mod dart;
pub(crate) mod ecmascript;
pub mod fsharp;
pub mod go;
pub mod javascript;
pub mod php;
pub mod python;
pub mod rust_lang;
pub mod shared;
pub mod typescript;

/// [PIPELINE-LANG-TRAIT] The registered parser that claims `path`'s
/// extension — matched lowercase and without the leading `.` — or `None`
/// when no parser does. The one place a file is mapped to its language,
/// so a report consumer and the engine cannot disagree about which
/// grammar a path belongs to.
#[must_use]
pub fn parser_for_path<'parsers>(
    parsers: &'parsers [Box<dyn LanguageParser>],
    path: &std::path::Path,
) -> Option<&'parsers dyn LanguageParser> {
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_lowercase)?;
    parsers
        .iter()
        .map(Box::as_ref)
        .find(|parser| parser.file_extensions().contains(&extension.as_str()))
}

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

    /// Binding-introducing node table for the refactor free-variable
    /// walk ([AUTOFIX-EXTRACT-FREE-VARS]). The default (empty) leaves
    /// the language without extract-method support.
    fn binding_node_kinds(&self) -> &'static [BindingKind] {
        &[]
    }

    /// Identifier-reference recognition table for the refactor
    /// free-variable walk ([AUTOFIX-EXTRACT-FREE-VARS]). The default
    /// recognises no references.
    fn identifier_reference_kinds(&self) -> &'static ReferenceTable {
        &EMPTY_REFERENCE_TABLE
    }

    /// Container/scope kinds for the extract-method preconditions
    /// ([AUTOFIX-EXTRACT-PRECONDITIONS] rules 4–5). `None` (the
    /// default) means the verbatim extract action is not offered for
    /// this language.
    fn extract_scope_kinds(&self) -> Option<&'static ScopeKinds> {
        None
    }

    /// Emits the language-shaped extract-method declaration and call
    /// text ([AUTOFIX-EXTRACT-EMITTER]). `None` (the default) means the
    /// language has free-variable tables but no emitter yet, so no
    /// action is offered.
    fn emit_extract_method(&self, request: &EmitRequest<'_, '_>) -> Option<EmitOutcome> {
        let _ = request;
        None
    }

    /// Per-language tables for the mechanical merge
    /// ([AUTOFIX-MERGE-GATE], [AUTOFIX-MERGE-SAFETY]). `None` (the
    /// default) routes every merge to `ai_or_human`.
    fn merge_tables(&self) -> Option<&'static MergeTables> {
        None
    }

    /// Syntactic declared-type lookup for an identifier hole
    /// ([AUTOFIX-MERGE-SAFETY] D): the explicit type text of `name`'s
    /// declaration inside `function`, or `None` when no explicit
    /// declaration is visible — which refuses the merge.
    fn declared_type_of(
        &self,
        function: tree_sitter::Node<'_>,
        name: &str,
        source: &[u8],
    ) -> Option<String> {
        let _ = (function, name, source);
        None
    }

    /// Emits the language-shaped merged helper and per-site call texts
    /// ([AUTOFIX-MERGE-NAMES] typed parameters, no placeholders).
    /// `None` (the default) routes the merge to `ai_or_human`.
    fn emit_merge_method(&self, request: &MergeEmitRequest<'_, '_>) -> Option<MergeEmitOutcome> {
        let _ = request;
        None
    }
}
