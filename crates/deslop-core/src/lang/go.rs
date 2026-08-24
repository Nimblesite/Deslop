//! Go language plugin.
//!
//! Implements [PIPELINE-LANG-TRAIT] for Go using the `tree-sitter-go`
//! grammar ([LANG-CAND-GO]). Normalisation follows the Type-2-invariance
//! principle ([CLONE-TYPE-TAXONOMY]) and the same identifier / literal /
//! trivia collapse the other languages use:
//!
//! - The grammar's identifier leaves — `identifier`, `field_identifier`,
//!   `type_identifier`, `package_identifier`, `blank_identifier`, and
//!   `label_name` — collapse to `"__ident__"` so renamed clones fingerprint
//!   identically. Qualified names stay structural wrappers over those
//!   leaves, so `pkg.Name` keeps a shape distinct from a bare identifier
//!   while each segment collapses — parity with the Python / TypeScript
//!   member-access handling.
//! - Every constant leaf collapses to `"__literal__"`: `int_literal`,
//!   `float_literal`, `imaginary_literal`, `rune_literal`, both string
//!   forms (`interpreted_string_literal`, `raw_string_literal`) together
//!   with their `*_content` bodies and `escape_sequence` (parity with the
//!   ECMAScript string handling), and the predeclared value leaves `true`,
//!   `false`, `nil`, and `iota`. Composite literals (`composite_literal`,
//!   `literal_value`, `literal_element`) and closures (`func_literal`)
//!   stay structural — their shape is the clone signal.
//! - `comment` is dropped as trivia.
//! - All other named node kinds pass through unchanged.
//!
//! Shared walking / interning plumbing lives in [`super::shared`].

use crate::{
    ast::NormalizedNode,
    error::CoreError,
    lang::{
        shared::{build_normalised_root, intern_kind, parse_source, IDENTIFIER_KIND, LITERAL_KIND},
        LanguageParser,
    },
    state::FileId,
};

/// Stable language identifier reported by [`GoParser::id`].
const LANGUAGE_ID: &str = "go";

/// Go implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct GoParser;

impl GoParser {
    /// Creates a new parser. Stateless — safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for GoParser {
    fn id(&self) -> &'static str {
        LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["go"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_go::LANGUAGE.into()
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

/// Maps a tree-sitter Go node kind to its normalised form. Covers the
/// identifier / literal / trivia families emitted by `tree-sitter-go`
/// 0.25.x ([LANG-CAND-GO]).
fn normalise_kind(raw: &str) -> Option<&'static str> {
    match raw {
        // Drop trivia — [PIPELINE-NORMALIZE-AST]
        "comment" => None,
        // Identifier leaves — collapse for Type-2 renamed-clone detection
        "identifier" | "field_identifier" | "type_identifier" | "package_identifier"
        | "blank_identifier" | "label_name" => Some(IDENTIFIER_KIND),
        // Literals — collapse so constant edits do not perturb fingerprints
        "int_literal"
        | "float_literal"
        | "imaginary_literal"
        | "rune_literal"
        | "interpreted_string_literal"
        | "interpreted_string_literal_content"
        | "raw_string_literal"
        | "raw_string_literal_content"
        | "escape_sequence"
        | "true"
        | "false"
        | "nil"
        | "iota" => Some(LITERAL_KIND),
        // All structural kinds pass through unchanged
        other => Some(intern_kind(other)),
    }
}
