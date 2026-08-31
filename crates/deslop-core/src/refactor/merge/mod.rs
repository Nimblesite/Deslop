//! The mechanical call-site merge ([AUTOFIX-MERGE]).
//!
//! Anti-unifies a leaf-gap Type-2 / constrained Type-3 cluster into
//! one parameterised helper and rewrites every site, entirely without
//! AI. Refusals are first-class: every gate or safety failure produces
//! a [`MergePlan`] with an `ai_or_human` verdict and a human-readable
//! reason — never an error, never a partial edit.

pub mod gate;
pub mod naming;
pub mod safety;

use std::path::Path;

use crate::{
    ast::{ByteRange, NormalizedNode},
    lang::{shared::parse_source, LanguageParser},
    refactor::{
        emit::PlannedEdit,
        preconditions::{self, OccurrenceScope},
        wire_edit, RefactorError,
    },
    report::ReportCluster,
    wire_generated::{MergeParameter, MergePlan, MergeVerdict},
};

/// Everything a language plugin needs to emit the merged helper
/// ([AUTOFIX-MERGE-NAMES] typed parameters, no placeholder types).
#[derive(Debug)]
pub struct MergeEmitRequest<'t, 'a> {
    /// Full source bytes of the (single) file being rewritten.
    pub source: &'a [u8],
    /// Stable cluster id; the deterministic helper name derives from
    /// its first six characters.
    pub cluster_id: &'a str,
    /// Anti-unified helper body with parameter names spliced in.
    pub helper_body: &'a str,
    /// Typed, named, defaulted parameters in slot order.
    pub parameters: &'a [MergeParameter],
    /// Per-occurrence statement runs and scopes, ascending by offset.
    pub scopes: &'a [OccurrenceScope<'t>],
}

/// A language emitter's merge result: the helper text plus one call
/// per site (arguments differ per site).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeEmitOutcome {
    /// Deterministic helper name.
    pub helper_name: String,
    /// Byte offset at which `insertion_text` is inserted.
    pub insertion_offset: usize,
    /// Helper declaration text.
    pub insertion_text: String,
    /// Call-site text per occurrence, in occurrence order.
    pub call_texts: Vec<String>,
}

/// Computes the mechanical merge plan for one cluster over its single
/// file. Refusals return an `ai_or_human` plan with the reason —
/// errors are reserved for a missing parse tree, mirroring
/// [`crate::refactor::compute_plan`].
///
/// # Errors
///
/// Returns [`RefactorError::Core`] when the source fails to parse.
pub fn compute_merge_plan(
    cluster: &ReportCluster,
    source: &[u8],
    file_root: &NormalizedNode,
    absolute_path: &Path,
    parser: &dyn LanguageParser,
) -> Result<MergePlan, RefactorError> {
    match mechanical_plan(cluster, source, file_root, absolute_path, parser)? {
        Ok(plan) => Ok(plan),
        Err(reason) => Ok(refusal(cluster, parser, reason)),
    }
}

/// The mechanical pipeline; the inner `Err` is the routing reason.
fn mechanical_plan(
    cluster: &ReportCluster,
    source: &[u8],
    file_root: &NormalizedNode,
    absolute_path: &Path,
    parser: &dyn LanguageParser,
) -> Result<Result<MergePlan, String>, RefactorError> {
    let Some(tables) = parser.merge_tables() else {
        return Ok(Err(format!(
            "{} has no mechanical merge support yet",
            parser.id()
        )));
    };
    let Some(scope_kinds) = parser.extract_scope_kinds() else {
        return Ok(Err(format!("{} has no refactor scope tables", parser.id())));
    };
    let Some(ranges) = preconditions::eligible_ranges(cluster) else {
        return Ok(Err(pre_screen_refusal(cluster)));
    };
    let tree = parse_source(parser.id(), &parser.grammar(), source)?;
    let Some(scopes) = preconditions::occurrence_scopes(tree.root_node(), &ranges, scope_kinds)
    else {
        return Ok(Err(
            "occurrences are not statement-aligned within one shared scope".to_owned(),
        ));
    };
    let spans: Vec<ByteRange> = scopes.iter().map(OccurrenceScope::span).collect();
    if preconditions::slices_equivalent(source, &spans) {
        return Ok(Err(type1_refusal(&scopes, source, parser, scope_kinds)));
    }
    let context = MergeContext {
        cluster,
        source,
        absolute_path,
        parser,
        tables,
        scope_kinds,
        scopes: &scopes,
        spans: &spans,
    };
    Ok(build_plan(&context, file_root))
}

/// Why the cluster never reached the merge machinery
/// ([`preconditions::eligible_ranges`]). A cluster the measured content
/// gate refused states that evidence in its own numbers
/// ([FUSED-CONTENT-GATE], gh #344) — telling a user their two methods
/// are "not mergeable" when the engine actually found 18% raw-content
/// agreement hides the one fact that would let them judge the verdict.
/// Every other pre-screen failure is a shape fact about the cluster
/// record, and the wording enumerates them.
fn pre_screen_refusal(cluster: &ReportCluster) -> String {
    preconditions::content_refusal(cluster).unwrap_or_else(|| {
        "cluster shape not mergeable (bucket, truncation, multi-file, or overlap)".to_owned()
    })
}

/// The refusal reason for a byte-identical Type-1 candidate. Routing
/// the user to the verbatim extract action is only honest when that
/// action's dataflow rules would accept the span — the extract tier
/// refuses such spans silently, so the advice would dead-end. A failing
/// rule therefore surfaces its own safety reason ([AUTOFIX-MERGE-SAFETY]).
fn type1_refusal(
    scopes: &[OccurrenceScope<'_>],
    source: &[u8],
    parser: &dyn LanguageParser,
    scope_kinds: &'static crate::refactor::tables::ScopeKinds,
) -> String {
    let free_variables = scopes.first().map_or_else(Vec::new, |scope| {
        crate::refactor::free_variables_of_run(&scope.run, source, parser, scope_kinds)
    });
    crate::refactor::dataflow_refusal(scopes, &free_variables, source, parser, scope_kinds)
        .map_or_else(
            |reason| reason,
            |()| {
                "occurrences are byte-identical Type-1 — use the verbatim extract action".to_owned()
            },
        )
}

/// Everything the gate → safety → naming → emission pipeline consumes.
struct MergeContext<'t, 'a> {
    /// Cluster being merged.
    cluster: &'a ReportCluster,
    /// Full source bytes of the single file.
    source: &'a [u8],
    /// Absolute path of that file (for the wire `WorkspaceEdit` URI).
    absolute_path: &'a Path,
    /// Language plugin driving tables and emission.
    parser: &'a dyn LanguageParser,
    /// Per-language merge tables.
    tables: &'static crate::refactor::tables::MergeTables,
    /// Per-language scope tables.
    scope_kinds: &'static crate::refactor::tables::ScopeKinds,
    /// Per-occurrence statement runs, ascending.
    scopes: &'a [OccurrenceScope<'t>],
    /// Effective rewrite spans, ascending.
    spans: &'a [ByteRange],
}

/// Gate → safety → naming → emission, all refusal-driven.
fn build_plan(
    context: &MergeContext<'_, '_>,
    file_root: &NormalizedNode,
) -> Result<MergePlan, String> {
    let forests: Option<Vec<Vec<&NormalizedNode>>> = context
        .spans
        .iter()
        .map(|span| effective_forest(file_root, *span))
        .collect();
    let Some(forests) = forests else {
        return Err("normalised subtrees unavailable for the occurrences".to_owned());
    };
    let gate = gate::evaluate(&forests, context.source, context.spans)?;
    let roles = safety::evaluate(
        context.scopes,
        context.source,
        &gate.holes,
        context.parser,
        context.tables,
        context.scope_kinds,
    )?;
    gate::budget_guard(
        gate.matched_nodes,
        distinct_substitutions(&gate.holes, &roles),
    )?;
    let slots = naming::derive(
        &gate.holes,
        &roles,
        context.tables.supports_default_parameters,
    )?;
    let mut parameters = free_value_parameters(context, &gate.holes, &slots)?;
    parameters.extend(slots.iter().map(|slot| slot.parameter.clone()));
    let helper_body = spliced_body(context.source, context.spans, &gate.holes, &slots)
        .ok_or_else(|| "helper body could not be rendered".to_owned())?;
    let request = MergeEmitRequest {
        source: context.source,
        cluster_id: &context.cluster.id,
        helper_body: &helper_body,
        parameters: &parameters,
        scopes: context.scopes,
    };
    let outcome = context
        .parser
        .emit_merge_method(&request)
        .ok_or_else(|| format!("{} has no merge emitter yet", context.parser.id()))?;
    Ok(assemble(context, parameters, helper_body, outcome))
}

/// Builds the final mechanical plan with its wire `WorkspaceEdit`.
fn assemble(
    context: &MergeContext<'_, '_>,
    parameters: Vec<MergeParameter>,
    helper_body: String,
    outcome: MergeEmitOutcome,
) -> MergePlan {
    let mut edits: Vec<PlannedEdit> = context
        .spans
        .iter()
        .zip(&outcome.call_texts)
        .map(|(span, call)| PlannedEdit {
            start_byte: span.start,
            end_byte: span.end,
            new_text: call.clone(),
        })
        .collect();
    edits.push(PlannedEdit {
        start_byte: outcome.insertion_offset,
        end_byte: outcome.insertion_offset,
        new_text: outcome.insertion_text,
    });
    edits.sort_unstable_by_key(|edit| std::cmp::Reverse(edit.start_byte));
    let file = wire_edit::FileEdits {
        absolute_path: context.absolute_path.to_path_buf(),
        source: context.source,
        edits,
    };
    MergePlan {
        cluster_id: context.cluster.id.clone(),
        language: context.parser.id().to_owned(),
        verdict: MergeVerdict::Mechanical,
        helper_name: outcome.helper_name,
        helper_body,
        parameters,
        workspace_edit: wire_edit::workspace_edit_json(
            &[file],
            MERGE_ANNOTATION_ID,
            MERGE_ANNOTATION_LABEL,
        ),
    }
}

/// An `ai_or_human` plan carrying the routing reason
/// ([AUTOFIX-MERGE-GATE]).
fn refusal(cluster: &ReportCluster, parser: &dyn LanguageParser, reason: String) -> MergePlan {
    MergePlan {
        cluster_id: cluster.id.clone(),
        language: parser.id().to_owned(),
        verdict: MergeVerdict::AiOrHuman { reason },
        helper_name: String::new(),
        helper_body: String::new(),
        parameters: Vec::new(),
        workspace_edit: None,
    }
}

/// Context parameters ([AUTOFIX-MERGE-SAFETY] C/E): every free
/// variable of the canonical run becomes a typed value parameter — the
/// residual byte proof guarantees the same names appear at every site.
/// A free name with no explicit declared type refuses the merge (no
/// `object` guessing, [AUTOFIX-MERGE-NAMES]).
fn free_value_parameters(
    context: &MergeContext<'_, '_>,
    holes: &[gate::Hole],
    slots: &[naming::ParameterSlot],
) -> Result<Vec<MergeParameter>, String> {
    let canonical = context
        .scopes
        .first()
        .ok_or_else(|| "no canonical site".to_owned())?;
    let hole_names: std::collections::HashSet<&str> = slots
        .iter()
        .flat_map(|slot| slot.hole_indexes.iter())
        .filter_map(|index| holes.get(*index))
        .filter_map(|hole| hole.per_site.first())
        .map(|site| site.text.as_str())
        .collect();
    let free = crate::refactor::free_variables_of_run(
        &canonical.run,
        context.source,
        context.parser,
        context.scope_kinds,
    );
    let site_count = context.scopes.len();
    free.into_iter()
        .filter(|name| !hole_names.contains(name.as_str()))
        .map(|name| typed_context_parameter(context, canonical, name, site_count))
        .collect()
}

/// One typed pass-through parameter for a free variable.
fn typed_context_parameter(
    context: &MergeContext<'_, '_>,
    canonical: &OccurrenceScope<'_>,
    name: String,
    site_count: usize,
) -> Result<MergeParameter, String> {
    let function = canonical
        .function
        .ok_or_else(|| "free variables need an enclosing function for type lookup".to_owned())?;
    let type_name = context
        .parser
        .declared_type_of(function, &name, context.source)
        .ok_or_else(|| format!("no explicit declared type found for free variable `{name}`"))?;
    Ok(MergeParameter {
        per_site_arguments: vec![name.clone(); site_count],
        name,
        type_name,
        is_thunk: false,
        is_required: true,
        default_value: None,
    })
}

/// Distinct surviving substitutions after rename lifting — the count
/// the gate budgets ([AUTOFIX-MERGE-ANTIUNIFY] store rule).
fn distinct_substitutions(holes: &[gate::Hole], roles: &[safety::HoleRole]) -> usize {
    let tuples: std::collections::HashSet<Vec<&str>> = holes
        .iter()
        .zip(roles)
        .filter(|(_, role)| matches!(role, safety::HoleRole::Parameter { .. }))
        .map(|(hole, _)| {
            hole.per_site
                .iter()
                .map(|site| site.text.as_str())
                .collect()
        })
        .collect();
    tuples.len()
}

/// One site's arguments in slot order — shared by every language's
/// call emitter.
#[must_use]
pub fn site_arguments(parameters: &[MergeParameter], site: usize) -> Vec<&str> {
    parameters
        .iter()
        .filter_map(|parameter| parameter.per_site_arguments.get(site))
        .map(String::as_str)
        .collect()
}

/// One site's plain call statement (`name(a, b);`) — the default call
/// shape for languages without argument elision.
#[must_use]
pub fn plain_call_text(parameters: &[MergeParameter], helper_name: &str, site: usize) -> String {
    format!(
        "{helper_name}({});",
        site_arguments(parameters, site).join(", ")
    )
}

/// The normalised statement forest covering one effective span: the
/// covering node's children inside the span, or the node itself.
fn effective_forest(root: &NormalizedNode, span: ByteRange) -> Option<Vec<&NormalizedNode>> {
    let covering = root.smallest_covering(span)?;
    let inside: Vec<&NormalizedNode> = covering
        .children
        .iter()
        .filter(|child| span.start <= child.byte_range.start && child.byte_range.end <= span.end)
        .collect();
    Some(if inside.is_empty() {
        vec![covering]
    } else {
        inside
    })
}

/// Renders the helper body: the canonical (first) site's span text
/// with each parameter hole spliced to its parameter name; lifted
/// renames keep the canonical names untouched.
fn spliced_body(
    source: &[u8],
    spans: &[ByteRange],
    holes: &[gate::Hole],
    slots: &[naming::ParameterSlot],
) -> Option<String> {
    let span = spans.first()?;
    let mut body = std::str::from_utf8(source.get(span.start..span.end)?)
        .ok()?
        .to_owned();
    let mut splices: Vec<(usize, usize, &str)> = Vec::new();
    for slot in slots {
        for hole_index in &slot.hole_indexes {
            let hole = holes.get(*hole_index)?;
            let site = hole.per_site.first()?;
            let start = site.range.start.checked_sub(span.start)?;
            let end = site.range.end.checked_sub(span.start)?;
            splices.push((start, end, slot.parameter.name.as_str()));
        }
    }
    splices.sort_unstable_by_key(|(start, ..)| std::cmp::Reverse(*start));
    for (start, end, name) in splices {
        if end <= body.len() {
            body.replace_range(start..end, name);
        }
    }
    Some(body)
}

/// Annotation id labelling every merge edit in the preview tree
/// ([AUTOFIX-MERGE-CODE-ACTION] step 3).
const MERGE_ANNOTATION_ID: &str = "deslop.merge";

/// Annotation label shown on the merge preview tree.
const MERGE_ANNOTATION_LABEL: &str = "Deslop: merge duplicates into one parameterised helper";
