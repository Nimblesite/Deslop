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
}

/// How one language spells a merged helper.
pub(super) struct HelperDialect {
    /// Prefix of the deterministic helper name, before the cluster id.
    pub(super) name_prefix: &'static str,
    /// One indent level in this language.
    pub(super) indent_step: &'static str,
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

    /// The full helper text: declaration line, indented body, closing
    /// brace, and the blank line that separates it from what follows.
    fn helper_text(&self, request: &MergeEmitRequest<'_, '_>, indent: &str, name: &str) -> String {
        let statement_indent = format!("{indent}{}", self.indent_step);
        let signature = (self.signature)(name, &self.parameter_list(request));
        format!(
            "{indent}{signature} {{\n{statement_indent}{}\n{indent}}}\n\n",
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
        insertion_text: dialect.helper_text(request, &placement.indent, &helper_name),
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
