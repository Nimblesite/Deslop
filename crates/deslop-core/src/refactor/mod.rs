//! Mechanical (zero-risk) deduplication refactors ([AUTOFIX-EXTRACT]).
//!
//! Tier 1: the verbatim extract-method action on proven Type-1
//! clusters — one shared helper, every occurrence rewritten as a call,
//! all inside one file. The engine is language-agnostic; everything
//! language-specific flows through the [`crate::lang::LanguageParser`]
//! trait's refactor methods (node-kind tables + emitter), the same
//! single extension point as parsing.
//!
//! The LSP layer never walks ASTs or emits code — it forwards clusters
//! here and maps the returned [`ExtractMethodPlan`] onto a
//! `WorkspaceEdit` ([AUTOFIX-EXTRACT-CODE-ACTION]).

pub mod consolidate;
pub mod emit;
pub mod merge;
pub mod position;
pub mod preconditions;
pub(crate) mod read_after;
pub mod tables;

mod free_vars;
pub(crate) mod wire_edit;

use std::path::Path;

use free_vars::WalkTables;

use crate::{
    error::CoreError,
    lang::{shared::parse_source, LanguageParser},
    pipeline::{default_parsers, language_for_path},
    refactor::emit::EmitRequest,
    report::ReportCluster,
};

pub use emit::{ExtractMethodPlan, PlannedEdit};

/// Refactor-computation failure. Reserved for "we tried to compute and
/// the parse tree was missing" — failed preconditions are `Ok(None)`,
/// never an error.
#[derive(Debug, thiserror::Error)]
pub enum RefactorError {
    /// The occurrence file could not be parsed.
    #[error(transparent)]
    Core(#[from] CoreError),
}

/// Computes the verbatim extract-method plan for one cluster, or
/// `Ok(None)` when any [AUTOFIX-EXTRACT-PRECONDITIONS] rule fails.
/// `source` is the full byte content of the single file every
/// occurrence lives in.
///
/// # Errors
///
/// Returns [`RefactorError::Core`] when the source fails to parse.
pub fn compute_plan(
    cluster: &ReportCluster,
    source: &[u8],
    parser: &dyn LanguageParser,
) -> Result<Option<ExtractMethodPlan>, RefactorError> {
    let Some(scope_kinds) = parser.extract_scope_kinds() else {
        return Ok(None);
    };
    let Some(ranges) = preconditions::eligible_ranges(cluster) else {
        return Ok(None);
    };
    let tree = parse_source(parser.id(), &parser.grammar(), source)?;
    let Some(scopes) = preconditions::occurrence_scopes(tree.root_node(), &ranges, scope_kinds)
    else {
        return Ok(None);
    };
    let effective_spans: Vec<_> = scopes
        .iter()
        .map(preconditions::OccurrenceScope::span)
        .collect();
    if !preconditions::slices_equivalent(source, &effective_spans) {
        return Ok(None);
    }
    // Rule 6 ([AUTOFIX-EXTRACT-PRECONDITIONS], issue #278): a span
    // whose own bindings are read after it would corrupt the enclosing
    // code when rewritten as a call. Silent refusal, reason discarded.
    if read_after::read_after_check(&scopes, source, parser, scope_kinds).is_err() {
        return Ok(None);
    }
    let tables = WalkTables::for_language(parser, scope_kinds);
    let free_variables = scopes.first().map_or_else(Vec::new, |scope| {
        free_vars::free_variables(&scope.run, source, tables)
    });
    let request = EmitRequest {
        source,
        cluster_id: &cluster.id,
        free_variables: &free_variables,
        scopes: &scopes,
    };
    Ok(parser
        .emit_extract_method(&request)
        .map(|outcome| emit::assemble_plan(outcome, &scopes, free_variables)))
}

/// Free variables of a raw statement run in first-reference order —
/// shared by the verbatim extract and the merge engine's context
/// parameters ([AUTOFIX-EXTRACT-FREE-VARS], [AUTOFIX-MERGE-SAFETY]).
pub(crate) fn free_variables_of_run(
    run: &[tree_sitter::Node<'_>],
    source: &[u8],
    parser: &dyn LanguageParser,
    scope_kinds: &'static tables::ScopeKinds,
) -> Vec<String> {
    free_vars::free_variables(run, source, WalkTables::for_language(parser, scope_kinds))
}

/// Resolves the language plugin responsible for `path`, when the
/// extension maps to a registered language. Used by the LSP layer to
/// hand [`compute_plan`] the right parser without touching the
/// pipeline session.
#[must_use]
pub fn parser_for_path(path: &Path) -> Option<Box<dyn LanguageParser>> {
    let language = language_for_path(path);
    default_parsers()
        .into_iter()
        .find(|parser| parser.id() == language)
}
