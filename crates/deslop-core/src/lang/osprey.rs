//! [LANG-CAND-OSPREY] Native Osprey grammar and Type-2 normalization.

use crate::{
    ast::NormalizedNode,
    error::CoreError,
    lang::{
        shared::{
            build_normalised_root, file_root, normalise_kind_with, normalise_node, parse_source,
        },
        LanguageParser,
    },
    state::FileId,
};
use tree_sitter::{Node, Tree};

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
/// Nesting depth of a file's top-level declarations below `__file__`.
const TOP_LEVEL_DEPTH: usize = 2;

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
        if self.syntax == Syntax::Ml {
            return intact_root(&tree, source, file_id, self.id());
        }
        build_normalised_root(&tree, file_id, normalise_kind, self.id())
    }
}

/// [LANG-CAND-OSPREY-RECOVERY] The file root over only the top-level
/// declarations tree-sitter parsed without error. An unfinished
/// declaration drops out whole, so no match is ever invented inside
/// erroneous syntax, while every intact neighbor keeps its findings. A
/// file with errors and no intact declaration stays a visible skip
/// ([LANG-CAND-OSPREY-UNPARSED]).
fn intact_root(
    tree: &Tree,
    source: &[u8],
    file_id: FileId,
    language: &'static str,
) -> Result<NormalizedNode, CoreError> {
    let root = tree.root_node();
    let mut cursor = root.walk();
    let children = root
        .named_children(&mut cursor)
        .filter(|declaration| !declaration.has_error())
        .map(|declaration| intact_declaration(declaration, source, file_id, language))
        .filter_map(Result::transpose)
        .collect::<Result<Vec<_>, _>>()?;
    if children.is_empty() && root.has_error() {
        return Err(CoreError::ParseFailed { language });
    }
    Ok(file_root(root, children, file_id))
}

/// One error-free top-level declaration, ending where its own text ends.
fn intact_declaration(
    declaration: Node<'_>,
    source: &[u8],
    file_id: FileId,
    language: &'static str,
) -> Result<Option<NormalizedNode>, CoreError> {
    let normalised = normalise_node(
        declaration,
        file_id,
        normalise_kind,
        language,
        TOP_LEVEL_DEPTH,
    )?;
    Ok(normalised.map(|mut node| {
        node.byte_range.end = declaration_end(declaration, source);
        node
    }))
}

/// [LANG-CAND-OSPREY-RECOVERY] The line break closing a declaration's last
/// token. The ML grammar's layout tokens are zero-width and sit after every
/// blank line that follows, so the parse's own end swallows those blank
/// lines and moves whenever a declaration is added below.
fn declaration_end(declaration: Node<'_>, source: &[u8]) -> usize {
    let last_token = last_token_end(declaration);
    source
        .get(last_token..declaration.end_byte())
        .and_then(|layout| layout.iter().position(|byte| *byte == b'\n'))
        .map_or(declaration.end_byte(), |offset| {
            last_token.saturating_add(offset).saturating_add(1)
        })
}

/// The end of the last token under `node` that spans any source.
fn last_token_end(node: Node<'_>) -> usize {
    let mut current = node;
    while let Some(child) = (0..current.child_count())
        .rev()
        .filter_map(|index| {
            u32::try_from(index)
                .ok()
                .and_then(|index| current.child(index))
        })
        .find(|child| child.end_byte() > child.start_byte())
    {
        current = child;
    }
    current.end_byte()
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
