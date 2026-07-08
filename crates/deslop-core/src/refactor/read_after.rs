//! Rule 6's read-after-span dataflow ([AUTOFIX-EXTRACT-PRECONDITIONS],
//! issue #278) — also [AUTOFIX-MERGE-SAFETY] check B's dataflow half.
//!
//! No name bound inside an occurrence's effective span may be read
//! after that span at *runtime*: positionally within the enclosing
//! function, and — in late-binding languages — from any deferred body
//! (Python `def`/`lambda`) anywhere in the horizon, because such
//! bodies resolve names at call time regardless of where they sit in
//! the source.

use std::collections::HashSet;

use tree_sitter::Node;

use crate::{
    ast::ByteRange,
    lang::LanguageParser,
    refactor::{
        free_vars,
        preconditions::{named_children, node_text, OccurrenceScope},
        tables::ScopeKinds,
    },
};

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
        let scan = ReadAfterScan {
            span: scope.span(),
            all_spans: &all_spans,
            source,
            parser,
            scope_kinds,
            bound: &bound,
        };
        if let Some(name) = read_after_span(horizon, &scan) {
            return Err(format!(
                "local `{name}` declared inside the span is read after it"
            ));
        }
    }
    Ok(())
}

/// Rule 7 ([AUTOFIX-EXTRACT-PRECONDITIONS], issue #280): no free
/// variable of the span may be an assignment target inside it — the
/// helper would mutate its own parameter copy and the caller's
/// variable would silently keep its old value, the mutation loss the
/// type-safety backstop cannot catch. Merge check D runs the same
/// dataflow per hole ([AUTOFIX-MERGE-SAFETY]).
pub(crate) fn write_in_span_check(
    scopes: &[OccurrenceScope<'_>],
    free_variables: &[String],
    source: &[u8],
    scope_kinds: &'static ScopeKinds,
) -> Result<(), String> {
    for scope in scopes {
        for name in free_variables {
            if written_in_span(scope, name, source, scope_kinds.write_kinds) {
                return Err(format!(
                    "free `{name}` is written inside the span — extracting would lose the mutation"
                ));
            }
        }
    }
    Ok(())
}

/// True when `name` is a bare-identifier assignment target anywhere
/// inside the occurrence span. Subscript/member targets do not match:
/// they mutate the object a parameter copy still shares
/// ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 7, [AUTOFIX-MERGE-SAFETY] D).
pub(crate) fn written_in_span(
    scope: &OccurrenceScope<'_>,
    name: &str,
    source: &[u8],
    write_kinds: &'static [(&'static str, &'static str)],
) -> bool {
    scope.run.iter().any(|node| {
        let mut stack = vec![*node];
        while let Some(current) = stack.pop() {
            if let Some((_, field)) = write_kinds.iter().find(|(kind, _)| *kind == current.kind())
            {
                let target = current
                    .child_by_field_name(*field)
                    .and_then(|child| node_text(child, source));
                if target.as_deref() == Some(name) {
                    return true;
                }
            }
            stack.extend(named_children(current));
        }
        false
    })
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
        free_vars::WalkTables::for_language(parser, scope_kinds),
    )
}

/// Everything rule 6's read-after scan needs about one occurrence's
/// surroundings: the effective span, every sibling occurrence span,
/// the language tables, and the names the span binds.
struct ReadAfterScan<'a> {
    /// The occurrence's effective span.
    span: ByteRange,
    /// Every occurrence span in the cluster (pruned during the scan).
    all_spans: &'a [ByteRange],
    /// The file's source bytes.
    source: &'a [u8],
    /// The language plugin supplying reference tables.
    parser: &'a dyn LanguageParser,
    /// The language's scope tables.
    scope_kinds: &'static ScopeKinds,
    /// Names bound at the top level of the span's statement run.
    bound: &'a HashSet<String>,
}

/// First in-span-declared name referenced after the span within
/// `horizon`, if any. Subtrees inside any of the cluster's occurrence
/// spans are pruned — they are rewritten away with their occurrence,
/// so a sibling occurrence re-binding the same names is not a read.
/// In late-binding languages the scan also descends *before* the span,
/// because a deferred body defined there still reads the span's
/// bindings at call time ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 6).
fn read_after_span(horizon: Node<'_>, scan: &ReadAfterScan<'_>) -> Option<String> {
    let mut stack = vec![horizon];
    while let Some(node) = stack.pop() {
        if inside_any_span(node, scan.all_spans) {
            continue;
        }
        if is_deferred_body(node, scan) {
            if let Some(name) = deferred_read(node, scan) {
                return Some(name);
            }
            continue;
        }
        if let Some(name) = positional_read(node, scan) {
            return Some(name);
        }
        stack.extend(scannable_children(node, scan));
    }
    None
}

/// True for a deferred body disjoint from the span — its reads happen
/// at call time and route through [`deferred_read`]; a node containing
/// the span keeps the normal descent.
fn is_deferred_body(node: Node<'_>, scan: &ReadAfterScan<'_>) -> bool {
    disjoint(node, scan.span) && scan.scope_kinds.deferred_frame_kinds.contains(&node.kind())
}

/// Children the scan must visit: everything in late-binding languages
/// (a deferred body may sit anywhere in the horizon), otherwise only
/// nodes reaching past the span's end.
fn scannable_children<'t>(node: Node<'t>, scan: &ReadAfterScan<'_>) -> Vec<Node<'t>> {
    let scan_before_span = !scan.scope_kinds.deferred_frame_kinds.is_empty();
    named_children(node)
        .into_iter()
        .filter(|child| scan_before_span || child.end_byte() > scan.span.end)
        .collect()
}

/// A bound name read at a position at or after the span's end. Skip
/// rules apply exactly as in the free-variable walk, so attribute and
/// keyword-argument names never count as reads.
fn positional_read(node: Node<'_>, scan: &ReadAfterScan<'_>) -> Option<String> {
    let references = scan.parser.identifier_reference_kinds();
    if node.start_byte() < scan.span.end
        || !references.reference_kinds.contains(&node.kind())
        || free_vars::reference_is_skipped(node, references)
    {
        return None;
    }
    node_text(node, scan.source).filter(|text| scan.bound.contains(text))
}

/// A span-bound name a deferred body reads when called: free in the
/// body per the frame-aware walk, or declared `global`/`nonlocal`
/// inside it — either way the read resolves in the scope the span's
/// bindings would vacate ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 6).
fn deferred_read(body: Node<'_>, scan: &ReadAfterScan<'_>) -> Option<String> {
    let tables = free_vars::WalkTables::for_language(scan.parser, scan.scope_kinds);
    free_vars::free_variables(&[body], scan.source, tables)
        .into_iter()
        .find(|name| scan.bound.contains(name))
        .or_else(|| escape_read(body, scan))
}

/// A span-bound name declared `global`/`nonlocal` anywhere under
/// `body` — the declaration re-binds its reads to an enclosing scope,
/// so the body's own frames cannot hide them from rule 6.
fn escape_read(body: Node<'_>, scan: &ReadAfterScan<'_>) -> Option<String> {
    let mut stack = vec![body];
    while let Some(node) = stack.pop() {
        if scan.scope_kinds.scope_escape_kinds.contains(&node.kind()) {
            let escaped = named_children(node)
                .into_iter()
                .filter_map(|child| node_text(child, scan.source))
                .find(|name| scan.bound.contains(name));
            if escaped.is_some() {
                return escaped;
            }
        }
        stack.extend(named_children(node));
    }
    None
}

/// True when `node` shares no bytes with `span`.
fn disjoint(node: Node<'_>, span: ByteRange) -> bool {
    node.end_byte() <= span.start || node.start_byte() >= span.end
}

/// True when `node` sits fully inside one of the occurrence spans.
fn inside_any_span(node: Node<'_>, spans: &[ByteRange]) -> bool {
    spans
        .iter()
        .any(|span| span.start <= node.start_byte() && node.end_byte() <= span.end)
}
