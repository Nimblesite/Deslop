//! Eligibility checks for the verbatim extract action
//! ([AUTOFIX-EXTRACT-PRECONDITIONS]).
//!
//! Every check answers `Option` — `None` means "silently not offered",
//! never an error. Rule 1 (proven Type-1) is re-verified on the exact
//! occurrence slices because the report-level byte-equivalence upgrade
//! can prove equivalence of contained C# methods rather than of the
//! raw slices ([CLONE-BUCKETS-IDENTICAL]).

use std::collections::HashSet;

use tree_sitter::Node;

use crate::{
    ast::ByteRange,
    buckets::{classify, ClusterKind},
    cluster_filters::enclosing_kind,
    lang::LanguageParser,
    refactor::{free_vars, tables::ScopeKinds},
    report::ReportCluster,
    report_render::canonicalise_whitespace,
};

/// One occurrence's aligned statement run and its enclosing scopes,
/// resolved during precondition checks and reused by the emitter.
#[derive(Debug, Clone)]
pub struct OccurrenceScope<'t> {
    /// The contiguous statement nodes the occurrence covers, in source
    /// order (never empty).
    pub run: Vec<Node<'t>>,
    /// Nearest function-like ancestor; `None` only for module-top-level
    /// occurrences in languages that allow them (rule 4).
    pub function: Option<Node<'t>>,
    /// The shared parent one level up (C#: containing class; Rust:
    /// `impl`/module; Python: class or module) — identical across all
    /// occurrences by rule 4.
    pub shared_parent: Node<'t>,
}

impl OccurrenceScope<'_> {
    /// Byte span covered by the statement run.
    #[must_use]
    pub fn span(&self) -> ByteRange {
        let start = self.run.first().map_or(0, tree_sitter::Node::start_byte);
        let end = self.run.last().map_or(0, tree_sitter::Node::end_byte);
        ByteRange { start, end }
    }
}

/// Exact-structural buckets a verbatim extract may come from. The
/// bucket is only a pre-filter — the authoritative Type-1 gate is the
/// byte-equivalence proof on the effective spans
/// ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 1), because the nested-cluster
/// collapse keeps the outer Type-2 view of the renamed-methods case
/// ([PIPELINE-CLUSTER-EXACT] #50).
const EXACT_BUCKETS: [ClusterKind; 3] = [
    ClusterKind::Identical,
    ClusterKind::NearlyIdentical,
    ClusterKind::StructuralOnly,
];

/// Applies rules 2–3 of [AUTOFIX-EXTRACT-PRECONDITIONS] to the cluster
/// record plus the bucket pre-filter: an exact-structural bucket, at
/// least two visible occurrences, all occurrences in one file, no wire
/// truncation (an unseen site could not be rewritten atomically), and
/// non-overlapping ranges. Returns occurrence byte ranges in ascending
/// order. Rule 1's byte-equivalence proof runs later, on the effective
/// spans ([`slices_equivalent`]).
#[must_use]
pub fn eligible_ranges(cluster: &ReportCluster) -> Option<Vec<ByteRange>> {
    if !EXACT_BUCKETS.contains(&classify(cluster)) || cluster.occurrences_truncated {
        return None;
    }
    let visible: Vec<_> = cluster
        .occurrences
        .iter()
        .filter(|occurrence| !occurrence.hidden)
        .collect();
    let (first, rest) = visible.split_first()?;
    if rest.is_empty() || rest.iter().any(|occurrence| occurrence.path != first.path) {
        return None;
    }
    let mut ranges: Vec<ByteRange> = visible
        .iter()
        .map(|occurrence| ByteRange {
            start: occurrence.start_byte,
            end: occurrence.end_byte,
        })
        .collect();
    ranges.sort_unstable_by_key(|range| (range.start, range.end));
    ranges_are_disjoint(&ranges).then_some(ranges)
}

/// Cheap cross-file consolidation screen for the LSP offer
/// ([AUTOFIX-CONSOLIDATE-SURFACE]): an exact-structural bucket with ≥2
/// visible, untruncated occurrences spanning ≥2 files. The
/// consolidation engine's gates decide the rest at resolve time, so a
/// candidate that ultimately refuses surfaces its reason instead of
/// silently offering nothing (issue #277).
#[must_use]
pub fn consolidation_candidate(cluster: &ReportCluster) -> bool {
    if !EXACT_BUCKETS.contains(&classify(cluster)) || cluster.occurrences_truncated {
        return false;
    }
    let visible: Vec<_> = cluster
        .occurrences
        .iter()
        .filter(|occurrence| !occurrence.hidden)
        .collect();
    let distinct: std::collections::HashSet<_> = visible
        .iter()
        .map(|occurrence| &occurrence.path)
        .collect();
    visible.len() >= 2 && distinct.len() >= 2
}

/// True when every range ends before the next begins.
fn ranges_are_disjoint(ranges: &[ByteRange]) -> bool {
    ranges
        .windows(2)
        .all(|pair| matches!(pair, [left, right] if left.end <= right.start))
}

/// Rule 1's authoritative Type-1 proof, run on the **effective spans**
/// the action will rewrite: every span's source bytes must be
/// whitespace-canonically equal — the same proof
/// [CLONE-BUCKETS-IDENTICAL] uses for the `Identical` bucket.
#[must_use]
pub fn slices_equivalent(source: &[u8], ranges: &[ByteRange]) -> bool {
    let slices: Option<Vec<&[u8]>> = ranges
        .iter()
        .map(|range| source.get(range.start..range.end))
        .collect();
    slices.is_some_and(|slices| raw_slices_equivalent(&slices))
}

/// Whitespace-canonical equality across raw slices — the same Type-1
/// proof, reused by consolidation across files
/// ([AUTOFIX-CONSOLIDATE-GATE]).
#[must_use]
pub(crate) fn raw_slices_equivalent(slices: &[&[u8]]) -> bool {
    let canonical: Vec<Vec<u8>> = slices
        .iter()
        .map(|slice| canonicalise_whitespace(slice))
        .collect();
    canonical
        .windows(2)
        .all(|pair| matches!(pair, [left, right] if left == right))
}

/// Resolves rules 4–5 for every occurrence: statement-boundary
/// alignment, function-like enclosing scope, and one shared parent.
/// `None` when any occurrence is misaligned, out of scope, or the
/// occurrences do not share the same parent node.
#[must_use]
pub fn occurrence_scopes<'t>(
    root: Node<'t>,
    ranges: &[ByteRange],
    scopes: &'static ScopeKinds,
) -> Option<Vec<OccurrenceScope<'t>>> {
    let resolved: Option<Vec<_>> = ranges
        .iter()
        .map(|range| occurrence_scope(root, *range, scopes))
        .collect();
    let resolved = resolved?;
    let first = resolved.first()?;
    resolved
        .iter()
        .all(|scope| same_node(scope.shared_parent, first.shared_parent))
        .then_some(resolved)
}

/// Resolves one occurrence's statement run and enclosing scopes.
fn occurrence_scope<'t>(
    root: Node<'t>,
    range: ByteRange,
    scopes: &'static ScopeKinds,
) -> Option<OccurrenceScope<'t>> {
    let (parent, run) = statement_run(root, range, scopes)?;
    let function = enclosing_kind(root, parent_range(parent), scopes.function_kinds);
    if function.is_none() && !module_top_level_allowed(parent, root, scopes) {
        return None;
    }
    let shared_parent = shared_parent_for(root, function, parent, scopes)?;
    Some(OccurrenceScope {
        run,
        function,
        shared_parent,
    })
}

/// The byte range of a node, used to seed ancestor lookups.
fn parent_range(node: Node<'_>) -> ByteRange {
    ByteRange {
        start: node.start_byte(),
        end: node.end_byte(),
    }
}

/// Rule 4's module-top-level escape hatch (Python only): the statement
/// container must be the parse root itself.
fn module_top_level_allowed(parent: Node<'_>, root: Node<'_>, scopes: &'static ScopeKinds) -> bool {
    scopes.allow_module_top_level && same_node(parent, root)
}

/// Nearest shared-parent ancestor (class / `impl` / module) of the
/// occurrence's function — or the parse root for module-level
/// languages when no named ancestor matches.
fn shared_parent_for<'t>(
    root: Node<'t>,
    function: Option<Node<'t>>,
    parent: Node<'t>,
    scopes: &'static ScopeKinds,
) -> Option<Node<'t>> {
    let anchor = function.unwrap_or(parent);
    let above = ancestor_of_kinds(anchor, scopes.shared_parent_kinds);
    match above {
        Some(node) => Some(node),
        None => scopes
            .shared_parent_kinds
            .contains(&root.kind())
            .then_some(root),
    }
}

/// Walks up from `node` (exclusive) to the nearest ancestor whose kind
/// is in `kinds`.
fn ancestor_of_kinds<'t>(node: Node<'t>, kinds: &[&str]) -> Option<Node<'t>> {
    let mut current = node.parent();
    while let Some(candidate) = current {
        if kinds.contains(&candidate.kind()) {
            return Some(candidate);
        }
        current = candidate.parent();
    }
    None
}

/// Node identity — tree-sitter node ids are unique within a tree.
fn same_node(left: Node<'_>, right: Node<'_>) -> bool {
    left.id() == right.id()
}

/// Rule 5: the occurrence must cover a contiguous run of named
/// children of one statement container — start and end sitting exactly
/// on child boundaries. Mid-expression ranges resolve to a non-container
/// parent and are silently skipped.
fn statement_run<'t>(
    root: Node<'t>,
    range: ByteRange,
    scopes: &'static ScopeKinds,
) -> Option<(Node<'t>, Vec<Node<'t>>)> {
    let covering = root.named_descendant_for_byte_range(range.start, range.end)?;
    if covering.start_byte() == range.start && covering.end_byte() == range.end {
        return exact_node_run(covering, scopes);
    }
    if scopes.function_kinds.contains(&covering.kind()) {
        return covered_function_body_run(covering, range, scopes);
    }
    child_run(covering, range, scopes)
}

/// A sibling-window occurrence over a function declaration narrows to
/// the body statements when the window covers the whole body — the
/// pipeline's sibling pass emits windows that start mid-signature, and
/// the signature is never part of the effective span. Windows stopping
/// mid-body are rejected.
fn covered_function_body_run<'t>(
    function: Node<'t>,
    range: ByteRange,
    scopes: &'static ScopeKinds,
) -> Option<(Node<'t>, Vec<Node<'t>>)> {
    let body = function.child_by_field_name("body")?;
    (range.start <= body.start_byte() && body.end_byte() <= range.end)
        .then(|| function_body_run(function, scopes))
        .flatten()
}

/// An occurrence that is exactly one node: a whole statement container
/// extracts its children; a whole function/method declaration narrows
/// to its body's statements ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 5 —
/// the renamed-methods-with-identical-bodies case); a single statement
/// extracts itself.
fn exact_node_run<'t>(
    node: Node<'t>,
    scopes: &'static ScopeKinds,
) -> Option<(Node<'t>, Vec<Node<'t>>)> {
    if scopes.statement_container_kinds.contains(&node.kind()) {
        let run = named_children(node);
        return (!run.is_empty()).then_some((node, run));
    }
    if scopes.function_kinds.contains(&node.kind()) {
        return function_body_run(node, scopes);
    }
    let parent = node.parent()?;
    scopes
        .statement_container_kinds
        .contains(&parent.kind())
        .then_some((parent, vec![node]))
}

/// Narrows a whole function/method occurrence to its body statements —
/// the effective span rewritten by the action. The signature stays
/// untouched.
fn function_body_run<'t>(
    function: Node<'t>,
    scopes: &'static ScopeKinds,
) -> Option<(Node<'t>, Vec<Node<'t>>)> {
    let body = container_body(function, scopes)?;
    let run = named_children(body);
    (!run.is_empty()).then_some((body, run))
}

/// The statement container behind a function's `body` field, descending
/// through single-child wrappers (Dart's `function_body` wraps the
/// `block`). Bounded so a malformed tree cannot loop.
fn container_body<'t>(function: Node<'t>, scopes: &'static ScopeKinds) -> Option<Node<'t>> {
    let mut body = function.child_by_field_name("body")?;
    for _ in 0..3_u8 {
        if scopes.statement_container_kinds.contains(&body.kind()) {
            return Some(body);
        }
        match named_children(body).as_slice() {
            [only] => body = *only,
            _ => return None,
        }
    }
    None
}

/// An occurrence spanning several siblings: the covering node must be a
/// statement container and the range must align exactly with a
/// contiguous child run.
fn child_run<'t>(
    covering: Node<'t>,
    range: ByteRange,
    scopes: &'static ScopeKinds,
) -> Option<(Node<'t>, Vec<Node<'t>>)> {
    if !scopes.statement_container_kinds.contains(&covering.kind()) {
        return None;
    }
    let children = named_children(covering);
    let start = children
        .iter()
        .position(|child| child.start_byte() == range.start)?;
    let end = children
        .iter()
        .position(|child| child.end_byte() == range.end)?;
    (start <= end).then(|| (covering, children.get(start..=end).unwrap_or(&[]).to_vec()))
}

/// Rule 6 ([AUTOFIX-EXTRACT-PRECONDITIONS], issue #278) — also
/// [AUTOFIX-MERGE-SAFETY] check B's dataflow half: no name bound
/// inside any occurrence's span may be read after that span within its
/// enclosing function (or, for module-top-level occurrences, the rest
/// of the shared parent). The `Err` carries the human-readable refusal
/// reason the merge tier surfaces; the extract tier discards it and
/// refuses silently.
pub(crate) fn read_after_check(
    scopes: &[OccurrenceScope<'_>],
    source: &[u8],
    parser: &dyn LanguageParser,
    scope_kinds: &'static ScopeKinds,
) -> Result<(), String> {
    let all_spans: Vec<ByteRange> = scopes.iter().map(OccurrenceScope::span).collect();
    for scope in scopes {
        let bound = run_bound_names(scope, source, parser, scope_kinds);
        let horizon = scope.function.unwrap_or(scope.shared_parent);
        let span = scope.span();
        if let Some(name) = read_after_span(horizon, span, &all_spans, source, parser, &bound) {
            return Err(format!(
                "local `{name}` declared inside the span is read after it"
            ));
        }
    }
    Ok(())
}

/// Names bound at the top level of one occurrence's statement run —
/// shared by [`read_after_check`] and the merge tier's rename lifting.
pub(crate) fn run_bound_names(
    scope: &OccurrenceScope<'_>,
    source: &[u8],
    parser: &dyn LanguageParser,
    scope_kinds: &'static ScopeKinds,
) -> HashSet<String> {
    free_vars::bound_names(
        &scope.run,
        source,
        free_vars::WalkTables {
            bindings: parser.binding_node_kinds(),
            references: parser.identifier_reference_kinds(),
            scopes: scope_kinds,
        },
    )
}

/// First in-span-declared name referenced after the span within
/// `horizon`, if any. Subtrees inside any of the cluster's occurrence
/// spans are pruned — they are rewritten away with their occurrence,
/// so a sibling occurrence re-binding the same names is not a read.
fn read_after_span(
    horizon: Node<'_>,
    span: ByteRange,
    all_spans: &[ByteRange],
    source: &[u8],
    parser: &dyn LanguageParser,
    bound: &HashSet<String>,
) -> Option<String> {
    let references = parser.identifier_reference_kinds();
    let mut stack = vec![horizon];
    while let Some(node) = stack.pop() {
        if inside_any_span(node, all_spans) {
            continue;
        }
        if node.start_byte() >= span.end && references.reference_kinds.contains(&node.kind()) {
            if let Some(text) = node_text(node, source) {
                if bound.contains(&text) {
                    return Some(text);
                }
            }
        }
        let children = named_children(node);
        stack.extend(
            children
                .into_iter()
                .filter(|child| child.end_byte() > span.end),
        );
    }
    None
}

/// True when `node` sits fully inside one of the occurrence spans.
fn inside_any_span(node: Node<'_>, spans: &[ByteRange]) -> bool {
    spans
        .iter()
        .any(|span| span.start <= node.start_byte() && node.end_byte() <= span.end)
}

/// UTF-8 text of one raw node — the shared leaf-reading primitive for
/// every refactor walk.
pub(crate) fn node_text(node: Node<'_>, source: &[u8]) -> Option<String> {
    source
        .get(node.byte_range())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(str::to_owned)
}

/// UTF-8 text of one child field of a raw node.
pub(crate) fn field_text(node: Node<'_>, field: &str, source: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|child| node_text(child, source))
}

/// Named children of `node` in source order — shared by the merge
/// engine's raw-tree scans.
pub(crate) fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}
