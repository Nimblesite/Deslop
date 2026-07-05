//! F# language plugin.
//!
//! Implements [PIPELINE-LANG-TRAIT] for F# using the `tree-sitter-fsharp`
//! source grammar (`LANGUAGE_FSHARP`, covering `.fs` implementation files
//! and `.fsx` scripts). Normalisation follows the Type-2-invariance
//! principle ([CLONE-TYPE-TAXONOMY]) and the same identifier / literal /
//! trivia collapse the other languages use ([PARSE-FSHARP-NORMALIZE]):
//!
//! - `identifier` and `op_identifier` are the grammar's identifier leaves,
//!   so both collapse to `"__ident__"` and renamed clones fingerprint
//!   identically. Compound names (`long_identifier`, `long_identifier_or_op`,
//!   `identifier_pattern`) are structural wrappers over those leaves, so a
//!   dotted path such as `A.B.C` stays structural while each segment
//!   collapses — parity with the Python / TypeScript member-access handling.
//! - Every constant leaf collapses to `"__literal__"`: `int` and `xint`
//!   (decimal and hex/octal/binary), `float`, `char`, `bool`, and the unit
//!   value `unit` (`()`); plus every string form — `string`,
//!   `triple_quoted_string`, their `format_string` /
//!   `format_triple_quoted_string` bodies, and `verbatim_string` — so that
//!   `"x"`, `"""x"""`, and `@"x"` fingerprint alike and constant edits do
//!   not perturb the fingerprint. The interpolation-hole container
//!   `format_string_eval` stays structural, so `$"{a}"` and `$"{b}"` still
//!   match through the collapsed identifiers while a plain string reduces to
//!   a constant `__literal__` subtree (parity with the Dart string
//!   treatment, [LANG-CAND-DART-RESULT]).
//! - `line_comment`, `block_comment`, and `xml_doc` are dropped as trivia.
//! - All other named node kinds pass through unchanged.
//!
//! The F# idioms the normaliser deliberately keeps structural — active
//! patterns, computation expressions (`async`/`seq`/`task`), pipelines
//! (`|>`), units of measure, and quotations — are catalogued in
//! `docs/plans/LANG-ROADMAP.md` under `[LANG-CAND-FSHARP]`, cross-checked
//! against the F# language reference
//! (<https://learn.microsoft.com/en-us/dotnet/fsharp/language-reference/>)
//! and the grammar's `node-types.json`.
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

/// Stable language identifier reported by [`FSharpParser::id`].
const LANGUAGE_ID: &str = "fsharp";

/// F# implementation of [`LanguageParser`].
#[derive(Debug, Default)]
pub struct FSharpParser;

impl FSharpParser {
    /// Creates a new parser. Stateless — safe to share across threads.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageParser for FSharpParser {
    fn id(&self) -> &'static str {
        LANGUAGE_ID
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["fs", "fsx"]
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_fsharp::LANGUAGE_FSHARP.into()
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

/// Maps a tree-sitter F# node kind to its normalised form. Covers the
/// identifier / literal / trivia families emitted by `tree-sitter-fsharp`
/// 0.3.x ([PARSE-FSHARP-NORMALIZE]).
fn normalise_kind(raw: &str) -> Option<&'static str> {
    match raw {
        // Drop trivia — [PIPELINE-NORMALIZE-AST]
        "line_comment" | "block_comment" | "xml_doc" => None,
        // Identifier leaves — collapse for Type-2 renamed-clone detection
        "identifier" | "op_identifier" => Some(IDENTIFIER_KIND),
        // Literals — collapse so constant edits do not perturb fingerprints
        "int" | "xint" | "float" | "char" | "bool" | "unit" | "string" | "format_string"
        | "triple_quoted_string" | "format_triple_quoted_string" | "verbatim_string" => {
            Some(LITERAL_KIND)
        }
        // All structural kinds pass through unchanged
        other => Some(intern_kind(other)),
    }
}
