//! Eligibility checks for the verbatim extract action
//! ([AUTOFIX-EXTRACT-PRECONDITIONS]).
//!
//! Every check answers `Option` — `None` means "silently not offered",
//! never an error. Rule 1 (proven Type-1) is re-verified on the exact
//! occurrence slices because the report-level byte-equivalence upgrade
//! can prove equivalence of contained C# methods rather than of the
//! raw slices ([CLONE-BUCKETS-IDENTICAL]).
//!
//! Rule 1 also has a content half ([`content_refusal`]): a shape match
//! is not evidence of duplication, so a cluster whose measured content
//! evidence does not vouch for it never reaches any of these actions
//! ([FUSED-CONTENT-GATE], gh #344).

use tree_sitter::Node;

use crate::{
    ast::{named_children, ByteRange},
    buckets::{classify, lacks_content_support, ClusterKind},
    cluster_filters::enclosing_kind,
    refactor::tables::ScopeKinds,
    render::signals::unvouched_content_reason,
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
/// ([PIPELINE-CLUSTER-EXACT]).
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
    let visible = visible_exact_occurrences(cluster)?;
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
/// silently offering nothing.
#[must_use]
pub fn consolidation_candidate(cluster: &ReportCluster) -> bool {
    visible_exact_occurrences(cluster).is_some()
        && crate::report::distinct_visible_path_count(cluster) >= 2
}

/// The content-evidence half of rule 1
/// ([AUTOFIX-EXTRACT-PRECONDITIONS], [FUSED-CONTENT-GATE]): the reason
/// this cluster's shape match is not evidence of duplication, or `None`
/// when the measured evidence vouches for it.
///
/// An exact-structural bucket says the *shapes* matched. It cannot say
/// the code matched: `structural` and `token_jaccard` are two views of
/// one normalised representation, so an anchor-poor scaffolding family
/// and a corroborated Type-2 rename both render `1.00 / 1.00`. Every
/// action behind this module folds N sites into one shared definition,
/// so acting on the first rewrites unrelated code — the merge engine
/// anti-unifies it, and the consolidation offer deletes all but one
/// copy outright. The measured content evidence is the only signal that
/// separates the two, so it decides here rather than downstream.
///
/// [`ClusterKind::Identical`] is exempt: [CLONE-BUCKETS-IDENTICAL]
/// awarded that bucket on raw-source byte equality, which is strictly
/// stronger evidence than the collapsed-leaf measurement — the same
/// exemption [`crate::buckets::content_gated_signals`] makes.
#[must_use]
pub fn content_refusal(cluster: &ReportCluster) -> Option<String> {
    let unvouched =
        classify(cluster) != ClusterKind::Identical && lacks_content_support(cluster.signals);
    unvouched.then(|| unvouched_content_reason(cluster.signals))
}

/// The visible occurrences of an exact-structural, untruncated cluster
/// whose measured content evidence vouches for it — the pre-screen
/// [`eligible_ranges`] and [`consolidation_candidate`] share. `None`
/// when the bucket, wire truncation, or [`content_refusal`]
/// disqualifies the cluster outright.
fn visible_exact_occurrences(
    cluster: &ReportCluster,
) -> Option<Vec<&crate::report::ReportOccurrence>> {
    if !EXACT_BUCKETS.contains(&classify(cluster))
        || cluster.occurrences_truncated
        || content_refusal(cluster).is_some()
    {
        return None;
    }
    Some(
        cluster
            .occurrences
            .iter()
            .filter(|occurrence| !occurrence.hidden)
            .collect(),
    )
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
        return exact_node_run(covering, scopes)
            .or_else(|| exact_node_run(widest_at_extent(covering), scopes));
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

/// The outermost ancestor sharing `node`'s exact byte extent — Python's
/// `expression_statement` wraps its expression at identical extent, so
/// the deepest-node lookup lands below the statement and rule 5 must
/// hop back up to align on the statement container's child.
fn widest_at_extent(node: Node<'_>) -> Node<'_> {
    let mut current = node;
    while let Some(parent) = current.parent() {
        if parent.start_byte() != current.start_byte() || parent.end_byte() != current.end_byte() {
            break;
        }
        current = parent;
    }
    current
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
