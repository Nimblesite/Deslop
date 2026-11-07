//! C language plugin.
//!
//! Implements [PIPELINE-LANG-TRAIT] for C using the `tree-sitter-c`
//! grammar ([LANG-CAND-C]). Normalisation follows the Type-2-invariance
//! principle ([CLONE-TYPE-TAXONOMY]) with the same identifier / literal /
//! trivia collapse the other languages use:
//!
//! - Identifier leaves — `identifier`, `field_identifier`,
//!   `type_identifier`, `statement_identifier` (goto labels) and
//!   `primitive_type` — collapse to `"__ident__"`, so a renamed function,
//!   struct member, typedef or label, or an `int` swapped for a `long`,
//!   fingerprints identically (parity with C#'s `predefined_type` and
//!   Rust's `primitive_type`).
//! - Constant leaves collapse to `"__literal__"`: `number_literal`,
//!   `character`, `string_content`, `escape_sequence`, `system_lib_string`
//!   and the `true` / `false` / `null` value leaves. The `string_literal`,
//!   `char_literal` and `concatenated_string` wrappers stay structural so
//!   adjacent-string concatenation keeps a shape distinct from one literal.
//! - `comment` is dropped as trivia.
//! - Everything else, including every preprocessor node, passes through
//!   unchanged. `#include` and `#define` are then kept out of clone
//!   evidence by the boilerplate filter ([LANG-CAND-C-PREPROC]) because
//!   the grammar never parses a macro body.
//!
//! Shared walking / interning plumbing lives in [`super::shared`].

use crate::{
    ast::NormalizedNode,
    error::CoreError,
    lang::{
        shared::{build_normalised_root, normalise_kind_with, parse_source},
        LanguageParser,
    },
    state::FileId,
};

/// Stable language identifier reported by [`CParser::id`].
pub const LANGUAGE_ID: &str = "c";

/// C implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct CParser;

impl CParser {
    /// Creates a new parser. Stateless — safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for CParser {
    fn id(&self) -> &'static str {
        LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["c", "h"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_c::LANGUAGE.into()
    }

    fn is_comment_kind(&self, kind: &str) -> bool {
        is_comment_kind(kind)
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

/// Maps a tree-sitter C node kind to its normalised form. Covers the
/// identifier / literal / trivia families emitted by `tree-sitter-c`
/// 0.24.x ([LANG-CAND-C]).
fn normalise_kind(raw: &str) -> Option<&'static str> {
    normalise_kind_with(raw, is_comment_kind, is_identifier_kind, is_literal_kind)
}

/// C trivia.
fn is_comment_kind(raw: &str) -> bool {
    matches!(raw, "comment")
}

/// C identifier leaves, collapsed for Type-2 renamed-clone detection.
fn is_identifier_kind(raw: &str) -> bool {
    matches!(
        raw,
        "identifier"
            | "field_identifier"
            | "type_identifier"
            | "statement_identifier"
            | "primitive_type"
    )
}

/// C literal leaves, collapsed so constant edits do not perturb
/// fingerprints.
fn is_literal_kind(raw: &str) -> bool {
    matches!(
        raw,
        "number_literal"
            | "character"
            | "string_content"
            | "escape_sequence"
            | "system_lib_string"
            | "true"
            | "false"
            | "null"
    )
}
