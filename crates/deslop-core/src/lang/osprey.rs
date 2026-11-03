//! [LANG-CAND-OSPREY] Native Osprey grammar and Type-2 normalization.

use crate::{
    ast::NormalizedNode,
    error::CoreError,
    lang::{
        shared::{build_normalised_root, normalise_kind_with, parse_source},
        LanguageParser,
    },
    state::FileId,
};

/// Stable Osprey language identifier.
pub(crate) const LANGUAGE_ID: &str = "osprey";
/// Python-flavor identity, separate from ordinary Python.
pub(crate) const PYTHON_LANGUAGE_ID: &str = "osprey-python";
/// Native layout-flavor identity.
pub(crate) const ML_LANGUAGE_ID: &str = "osprey-ml";
/// Default-flavor extensions supported by the pinned Osprey grammar.
const EXTENSIONS: &[&str] = &["osp"];
/// Native layout-flavor extension.
const ML_EXTENSIONS: &[&str] = &["ospml"];

/// A source surface with its own native CST and cache identity.
#[derive(Debug, Default, PartialEq, Eq)]
enum Syntax {
    /// Default braces syntax.
    #[default]
    Braces,
    /// ML layout syntax.
    Ml,
    /// Python-looking Osprey syntax.
    Python,
}

/// Osprey's default braces syntax, including effects and value handlers.
#[derive(Debug, Default)]
pub struct OspreyParser {
    /// Select the native grammar without enabling another language's refactors.
    syntax: Syntax,
}

impl OspreyParser {
    /// Creates the stateless Osprey parser.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            syntax: Syntax::Braces,
        }
    }

    /// Uses the same Python CST as Osprey's own Python frontend.
    #[must_use]
    pub const fn python() -> Self {
        Self {
            syntax: Syntax::Python,
        }
    }

    /// Uses native ML layout parsing with original source ranges.
    #[must_use]
    pub const fn ml() -> Self {
        Self { syntax: Syntax::Ml }
    }
}

impl LanguageParser for OspreyParser {
    fn id(&self) -> &'static str {
        match self.syntax {
            Syntax::Braces => LANGUAGE_ID,
            Syntax::Ml => ML_LANGUAGE_ID,
            Syntax::Python => PYTHON_LANGUAGE_ID,
        }
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        match self.syntax {
            Syntax::Braces => EXTENSIONS,
            Syntax::Ml => ML_EXTENSIONS,
            Syntax::Python => &[],
        }
    }

    fn grammar(&self) -> tree_sitter::Language {
        match self.syntax {
            Syntax::Braces => tree_sitter_osprey::LANGUAGE.into(),
            Syntax::Ml => tree_sitter_osprey_ml::LANGUAGE.into(),
            Syntax::Python => tree_sitter_python::LANGUAGE.into(),
        }
    }

    fn is_comment_kind(&self, kind: &str) -> bool {
        if self.syntax == Syntax::Python {
            kind == "comment"
        } else {
            is_comment_kind(kind)
        }
    }

    fn parse_and_normalize(
        &self,
        source: &[u8],
        file_id: FileId,
    ) -> Result<NormalizedNode, CoreError> {
        if self.syntax == Syntax::Python {
            return super::python::PythonParser::new().parse_and_normalize(source, file_id);
        }
        let tree = parse_source(self.id(), &self.grammar(), source)?;
        match (&self.syntax, tree.root_node().has_error()) {
            (Syntax::Ml, true) => {
                // [LANG-CAND-OSPREY-RECOVERY] Rejecting the entire file discarded
                // intact neighboring clones during incomplete edits.
                #[allow(clippy::panic)]
                {
                    panic!("accuracy quarantine: Osprey ML whole-file error rejection removed because it hides intact duplicates; implement verified grammar recovery");
                }
            }
            _ => build_normalised_root(&tree, file_id, normalise_kind, self.id()),
        }
    }
}

/// Preserve expression descendants, including those inside interpolation.
fn normalise_kind(raw: &str) -> Option<&'static str> {
    normalise_kind_with(raw, is_comment_kind, is_identifier_kind, is_literal_kind)
}

/// Value names collapse; qualified paths and type wrappers stay structural.
fn is_identifier_kind(raw: &str) -> bool {
    raw == "identifier"
}

/// Text and escapes normalize while embedded expression children survive.
fn is_literal_kind(raw: &str) -> bool {
    matches!(
        raw,
        "integer"
            | "float"
            | "boolean"
            | "string"
            | "interpolated_string"
            | "string_content"
            | "escape_sequence"
    )
}

/// Native line, documentation and ML block comments are trivia.
fn is_comment_kind(raw: &str) -> bool {
    matches!(
        raw,
        "line_comment" | "doc_comment" | "inner_doc_comment" | "block_comment"
    )
}
