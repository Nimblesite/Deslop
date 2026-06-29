//! Shared JavaScript / TypeScript normalisation.
//!
//! The JavaScript, TypeScript, TSX, and JSX grammars are separate
//! tree-sitter entry points, but their identifier / literal / trivia
//! vocabularies intentionally overlap. This module owns that common
//! mapping so the two language plug-ins differ only by id, extensions,
//! and grammar constant ([LANG-CAND-TYPESCRIPT], [LANG-CAND-JAVASCRIPT],
//! [PIPELINE-LANG-TRAIT]).

use tree_sitter::Language;

use crate::{
    ast::NormalizedNode,
    error::CoreError,
    lang::shared::{
        build_normalised_root, intern_kind, parse_source, IDENTIFIER_KIND, LITERAL_KIND,
    },
    state::FileId,
};

/// Parses an ECMAScript-family source file and applies the shared
/// JavaScript / TypeScript normalisation mapping.
pub(crate) fn parse_and_normalise(
    language_id: &'static str,
    grammar: &Language,
    source: &[u8],
    file_id: FileId,
) -> Result<NormalizedNode, CoreError> {
    let tree = parse_source(language_id, grammar, source)?;
    build_normalised_root(&tree, file_id, normalise_kind, language_id)
}

/// Maps a JavaScript / TypeScript tree-sitter node kind to its
/// normalised form. Comments and hash-bang lines are dropped;
/// identifier-like nodes collapse to `__ident__`; scalar and textual
/// literal nodes collapse to `__literal__`; every other named node stays
/// structural via interning.
pub(crate) fn normalise_kind(raw: &str) -> Option<&'static str> {
    match raw {
        raw if is_comment_kind(raw) => None,
        raw if is_identifier_kind(raw) => Some(IDENTIFIER_KIND),
        raw if is_literal_kind(raw) => Some(LITERAL_KIND),
        other => Some(intern_kind(other)),
    }
}

/// Returns true for trivia nodes emitted by the JS / TS grammars.
fn is_comment_kind(raw: &str) -> bool {
    matches!(raw, "comment" | "html_comment" | "hash_bang_line")
}

/// Returns true for identifier-like JS / TS nodes whose spelling should
/// not affect Type-2 clone detection.
fn is_identifier_kind(raw: &str) -> bool {
    matches!(
        raw,
        "identifier"
            | "nested_identifier"
            | "nested_type_identifier"
            | "private_property_identifier"
            | "property_identifier"
            | "shorthand_property_identifier"
            | "shorthand_property_identifier_pattern"
            | "statement_identifier"
            | "type_identifier"
    )
}

/// Returns true for literal-like JS / TS nodes collapsed by
/// normalisation. Template substitutions stay structural so the
/// interpolated expression shape is still fingerprinted.
fn is_literal_kind(raw: &str) -> bool {
    matches!(
        raw,
        "escape_sequence"
            | "false"
            | "jsx_text"
            | "null"
            | "number"
            | "regex"
            | "regex_flags"
            | "regex_pattern"
            | "string"
            | "string_fragment"
            | "template_string"
            | "true"
            | "undefined"
    )
}
