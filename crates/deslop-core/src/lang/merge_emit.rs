//! The merged-helper emission shape shared by every language emitter
//! ([AUTOFIX-MERGE-NAMES], [AUTOFIX-MERGE-DEFAULTS]).
//!
//! Each language decides three things: where the helper is inserted and
//! at what indent, how one typed parameter and the declaration line are
//! spelled, and how a call site is rendered. Everything else — the
//! deterministic helper name, the per-site call list, and the assembly
//! of declaration + body + blank line — is identical across C#, Dart and
//! Rust, so it lives here once.

use crate::refactor::{
    emit::cluster_id_prefix,
    merge::{plain_call_text, MergeEmitOutcome, MergeEmitRequest},
};
use crate::wire_generated::MergeParameter;

/// Where a language wants its merged helper written.
pub(super) struct HelperPlacement {
    /// Byte offset the helper text is inserted at.
    pub(super) insertion_offset: usize,
    /// Leading whitespace every line of the helper carries.
    pub(super) indent: String,
    /// What the insertion offset sits after, which decides the line
    /// breaks that join the helper to the text around it.
    pub(super) point: InsertionPoint,
}

/// What a helper's insertion offset sits after.
#[derive(Clone, Copy, Debug)]
pub(super) enum InsertionPoint {
    /// Directly after a container's opening brace, mid-line (C#: the
    /// class body's `{`). A line break opens the helper, and the brace's
    /// own line break — now after the helper — leaves the blank line
    /// before whatever follows.
    AfterOpeningBrace,
    /// At the start of a line (Dart, Rust: the first occurrence's
    /// function). Nothing precedes the helper on its line, so the
    /// helper supplies the blank line that separates it from the
    /// declaration it was written above.
    LineStart,
}

impl InsertionPoint {
    /// The line breaks written before and after the helper text.
    const fn line_breaks(self) -> (&'static str, &'static str) {
        match self {
            Self::AfterOpeningBrace => ("\n", "\n"),
            Self::LineStart => ("", "\n\n"),
        }
    }
}

/// Where a language puts the brace that opens the helper's body.
#[derive(Clone, Copy, Debug)]
pub(super) enum BraceStyle {
    /// At the end of the declaration line (Dart, Rust).
    SameLine,
    /// On its own line at the helper's indent (C#, Allman style).
    OwnLine,
}

impl BraceStyle {
    /// The text between the declaration line and the body's first line.
    fn opening(self, indent: &str) -> String {
        match self {
            Self::SameLine => " {".to_owned(),
            Self::OwnLine => format!("\n{indent}{{"),
        }
    }
}

/// How one language spells a merged helper.
pub(super) struct HelperDialect {
    /// Prefix of the deterministic helper name, before the cluster id.
    pub(super) name_prefix: &'static str,
    /// One indent level in this language.
    pub(super) indent_step: &'static str,
    /// Where the brace opening the helper's body goes.
    pub(super) brace: BraceStyle,
    /// Renders one typed parameter in a declaration list.
    pub(super) parameter: fn(&MergeParameter) -> String,
    /// Renders the declaration line, given the helper name and the
    /// already-joined parameter list — everything before the `{`.
    pub(super) signature: fn(&str, &str) -> String,
    /// Renders one site's call statement.
    pub(super) call: fn(&MergeEmitRequest<'_, '_>, &str, usize) -> String,
}

impl HelperDialect {
    /// The declaration list, rendered parameter by parameter.
    fn parameter_list(&self, request: &MergeEmitRequest<'_, '_>) -> String {
        request
            .parameters
            .iter()
            .map(self.parameter)
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The full helper text: declaration line, the brace where the
    /// dialect puts it, indented body, closing brace, and the line
    /// breaks the placement needs to sit between its neighbours.
    fn helper_text(
        &self,
        request: &MergeEmitRequest<'_, '_>,
        placement: &HelperPlacement,
        name: &str,
    ) -> String {
        let indent = placement.indent.as_str();
        let statement_indent = format!("{indent}{}", self.indent_step);
        let signature = (self.signature)(name, &self.parameter_list(request));
        let brace = self.brace.opening(indent);
        let (leading, trailing) = placement.point.line_breaks();
        format!(
            "{leading}{indent}{signature}{brace}\n{statement_indent}{}\n{indent}}}{trailing}",
            request.helper_body
        )
    }
}

/// Emits the merged helper and one call per occurrence.
///
/// The helper name is derived from the cluster id, so the same cluster
/// always produces the same name no matter which language emitted it.
pub(super) fn emit_merge_helper(
    request: &MergeEmitRequest<'_, '_>,
    placement: &HelperPlacement,
    dialect: &HelperDialect,
) -> MergeEmitOutcome {
    let helper_name = format!(
        "{}{}",
        dialect.name_prefix,
        cluster_id_prefix(request.cluster_id)
    );
    let call_texts = (0..request.scopes.len())
        .map(|site| (dialect.call)(request, &helper_name, site))
        .collect();
    MergeEmitOutcome {
        insertion_text: dialect.helper_text(request, placement, &helper_name),
        insertion_offset: placement.insertion_offset,
        helper_name,
        call_texts,
    }
}

/// The call renderer for languages with no argument elision: every
/// parameter is passed at every site.
pub(super) fn plain_call(
    request: &MergeEmitRequest<'_, '_>,
    helper_name: &str,
    site: usize,
) -> String {
    plain_call_text(request.parameters, helper_name, site)
}
