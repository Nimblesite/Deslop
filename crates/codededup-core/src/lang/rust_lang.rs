//! Rust language plugin.
//!
//! Implements [PIPELINE-LANG-TRAIT] for Rust using the
//! `tree-sitter-rust` grammar. Normalisation rules follow the same
//! Type-2-invariance principle as the C# plug-in ([CLONE-TYPE-TAXONOMY]):
//! every identifier flavour collapses to `__ident__`, every literal
//! flavour collapses to `__literal__`, line and block comments are
//! dropped. Shared walking / interning plumbing lives in
//! [`super::shared`].

use crate::{
    ast::NormalizedNode,
    error::CoreError,
    lang::{
        shared::{build_normalised_root, intern_kind, parse_source, IDENTIFIER_KIND, LITERAL_KIND},
        LanguageParser,
    },
    state::FileId,
};

/// Stable language identifier reported by [`RustParser::id`].
const LANGUAGE_ID: &str = "rust";

/// Rust implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct RustParser;

impl RustParser {
    /// Creates a new parser. Stateless — safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for RustParser {
    fn id(&self) -> &'static str {
        LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["rs"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_rust::language()
    }

    fn parse_and_normalize(
        &self,
        source: &[u8],
        file_id: FileId,
    ) -> Result<NormalizedNode, CoreError> {
        let tree = parse_source(LANGUAGE_ID, &self.grammar(), source)?;
        Ok(build_normalised_root(&tree, file_id, normalise_kind))
    }
}

/// Maps a tree-sitter Rust node kind to its normalised form. Covers the
/// identifier / literal / trivia families emitted by `tree-sitter-rust`
/// 0.21.x. Every other named kind passes through interned so the hash
/// stays stable across runs.
fn normalise_kind(raw: &str) -> Option<&'static str> {
    match raw {
        "line_comment" | "block_comment" => None,
        "identifier"
        | "type_identifier"
        | "field_identifier"
        | "shorthand_field_identifier"
        | "primitive_type"
        | "scoped_identifier"
        | "scoped_type_identifier"
        | "metavariable" => Some(IDENTIFIER_KIND),
        "string_literal" | "raw_string_literal" | "char_literal" | "integer_literal"
        | "float_literal" | "boolean_literal" => Some(LITERAL_KIND),
        other => Some(intern_kind(other)),
    }
}
