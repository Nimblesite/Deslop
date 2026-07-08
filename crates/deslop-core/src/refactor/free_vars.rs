//! Language-agnostic free-variable walk ([AUTOFIX-EXTRACT-FREE-VARS]).
//!
//! Walks the raw tree-sitter subtree of an occurrence, pushing bound
//! names into scope frames and recording identifier references that no
//! frame declares. The walk is purely syntactic: no symbol tables, no
//! member resolution, no type inference. Per-language node-kind tables
//! ([`crate::refactor::tables`]) declare what binds, what references,
//! and where nested scope frames open.

use std::collections::HashSet;

use tree_sitter::Node;

use crate::{
    lang::LanguageParser,
    refactor::{
        preconditions::node_text,
        tables::{BindingKind, FrameKind, ReferenceTable, ScopeKinds},
    },
};

/// Child fields whose subtrees never contribute bound names when a
/// binding node's names are collected: type annotations, default
/// values, and return types describe the binding without being bound.
const BINDING_SKIP_FIELDS: &[&str] = &["type", "value", "default", "return_type"];

/// Node kinds that make an assignment target opaque — assigning through
/// a member or index expression binds nothing (`self.cache = x`).
const BINDING_OPAQUE_KINDS: &[&str] = &["attribute", "subscript", "member_access_expression"];

/// Child fields walked *after* a binding node's names bind, so loop
/// variables are in scope inside their own body.
const LATE_FIELDS: &[&str] = &["body"];

/// Bundled per-language tables consumed by the walk.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WalkTables {
    /// Binding-introducing node patterns.
    pub bindings: &'static [BindingKind],
    /// Identifier-reference recognition rules.
    pub references: &'static ReferenceTable,
    /// Scope kinds (only `frame_kinds` is consulted here).
    pub scopes: &'static ScopeKinds,
}

impl WalkTables {
    /// Bundles one language's walk tables from its parser plus scope
    /// kinds — the single construction point for every walk consumer.
    pub(crate) fn for_language(parser: &dyn LanguageParser, scopes: &'static ScopeKinds) -> Self {
        Self {
            bindings: parser.binding_node_kinds(),
            references: parser.identifier_reference_kinds(),
            scopes,
        }
    }
}

/// One scope frame: the kind of the node that opened it (`None` for
/// the walk's root frame) and the names it binds. The kind lets hoisted
/// bindings skip transparent frames ([`crate::refactor::tables::HoistRule`]).
struct Frame {
    /// Node kind that opened the frame; `None` at the root.
    kind: Option<&'static str>,
    /// Names bound in this frame so far.
    names: HashSet<String>,
}

impl Frame {
    /// An empty frame opened by a node of `kind`.
    fn new(kind: Option<&'static str>) -> Self {
        Self {
            kind,
            names: HashSet::new(),
        }
    }
}

/// Mutable walk state: the scope-frame stack and the ordered,
/// deduplicated free-name list ([AUTOFIX-EXTRACT-FREE-VARS] step 5).
struct WalkState {
    /// Innermost frame last; names bound so far during the walk.
    frames: Vec<Frame>,
    /// Free names in first-reference textual order.
    free: Vec<String>,
    /// Frame kinds the *current* binding's names hoist past — set for
    /// the duration of one binding node's name collection, empty
    /// otherwise ([AUTOFIX-EXTRACT-FREE-VARS] PEP 572 walrus).
    hoist_past: &'static [&'static str],
}

impl WalkState {
    /// A fresh state with one root frame.
    fn new() -> Self {
        Self {
            frames: vec![Frame::new(None)],
            free: Vec::new(),
            hoist_past: &[],
        }
    }
}

/// Collects the free-variable list for a run of sibling statement
/// nodes, in first-reference textual order, deduplicated.
pub(crate) fn free_variables(run: &[Node<'_>], source: &[u8], tables: WalkTables) -> Vec<String> {
    let mut state = WalkState::new();
    for node in run {
        walk(*node, source, tables, &mut state);
    }
    state.free
}

/// Names bound at the top level of a statement run — its root scope
/// frame after a full walk. The merge safety checks scan the enclosing
/// function for these after the span ([AUTOFIX-MERGE-SAFETY] B).
pub(crate) fn bound_names(run: &[Node<'_>], source: &[u8], tables: WalkTables) -> HashSet<String> {
    let mut state = WalkState::new();
    for node in run {
        walk(*node, source, tables, &mut state);
    }
    state.frames.pop().map(|frame| frame.names).unwrap_or_default()
}

/// Dispatches one node to the frame / binding / reference / descend
/// branch. Pre-order: bindings earlier in the walk shadow later
/// references ([AUTOFIX-EXTRACT-FREE-VARS] steps 2–4).
fn walk(node: Node<'_>, source: &[u8], tables: WalkTables, state: &mut WalkState) {
    if let Some(frame) = frame_kind_for(node, tables.scopes) {
        walk_frame(node, source, tables, state, frame);
    } else if let Some(binding) = binding_kind_for(node, tables.bindings) {
        walk_binding(node, source, tables, state, binding);
    } else if tables.references.reference_kinds.contains(&node.kind()) {
        record_reference(node, source, tables.references, state);
    } else {
        walk_children(node, source, tables, state);
    }
}

/// Walks every named child of `node` in grammar order.
fn walk_children(node: Node<'_>, source: &[u8], tables: WalkTables, state: &mut WalkState) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk(child, source, tables, state);
    }
}

/// Returns the frame spec matching `node`, if any.
fn frame_kind_for(node: Node<'_>, scopes: &'static ScopeKinds) -> Option<&'static FrameKind> {
    scopes
        .frame_kinds
        .iter()
        .find(|frame| frame.node_kind == node.kind())
}

/// Returns the binding spec matching `node`, if any.
fn binding_kind_for(
    node: Node<'_>,
    bindings: &'static [BindingKind],
) -> Option<&'static BindingKind> {
    bindings
        .iter()
        .find(|binding| binding.node_kind == node.kind())
}

/// Enters a nested scope: binds the frame's own name outward (nested
/// function names), pushes a frame, binds parameters inward, walks the
/// remaining children, pops the frame.
fn walk_frame(
    node: Node<'_>,
    source: &[u8],
    tables: WalkTables,
    state: &mut WalkState,
    frame: &'static FrameKind,
) {
    if let Some(field) = frame.bind_outside_field {
        bind_field(node, field, source, tables, state);
    }
    state.frames.push(Frame::new(Some(frame.node_kind)));
    if let Some(field) = frame.bind_inside_field {
        bind_field(node, field, source, tables, state);
    }
    walk_frame_children(node, source, tables, state, frame);
    let _closed = state.frames.pop();
}

/// Walks a frame node's children — binding clauses first (Python
/// comprehensions bind before their textually-earlier body evaluates),
/// then the rest — skipping the fields already consumed by the frame's
/// inward/outward name binding.
fn walk_frame_children(
    node: Node<'_>,
    source: &[u8],
    tables: WalkTables,
    state: &mut WalkState,
    frame: &'static FrameKind,
) {
    let consumed = [frame.bind_inside_field, frame.bind_outside_field];
    let children: Vec<_> = named_children_with_fields(node)
        .into_iter()
        .filter(|(_, field)| !consumed.iter().flatten().any(|used| Some(*used) == *field))
        .collect();
    for (child, _) in children
        .iter()
        .filter(|(child, _)| frame.bind_first_kinds.contains(&child.kind()))
    {
        walk(*child, source, tables, state);
    }
    for (child, _) in children
        .iter()
        .filter(|(child, _)| !frame.bind_first_kinds.contains(&child.kind()))
    {
        walk(*child, source, tables, state);
    }
}

/// Processes a binding node in evaluation order: value-side children
/// first (as references), then the bound names, then late fields such
/// as loop bodies — so `x = x + 1` reads the outer `x` and a loop
/// variable is in scope inside its own body. Whole-node bindings
/// (`name_field: None`) route everything through the bound-name
/// collector, which walks describing fields as references itself.
fn walk_binding(
    node: Node<'_>,
    source: &[u8],
    tables: WalkTables,
    state: &mut WalkState,
    binding: &'static BindingKind,
) {
    let late = walk_early_children(node, source, tables, state, binding);
    let enclosing_hoist = state.hoist_past;
    state.hoist_past = hoist_past_for(binding.node_kind, tables.scopes);
    bind_field_or_node(node, binding.name_field, source, tables, state);
    state.hoist_past = enclosing_hoist;
    for child in late {
        walk(child, source, tables, state);
    }
}

/// Walks a binding node's value-side children as references and
/// returns the late-field children to walk after the names bind.
fn walk_early_children<'t>(
    node: Node<'t>,
    source: &[u8],
    tables: WalkTables,
    state: &mut WalkState,
    binding: &'static BindingKind,
) -> Vec<Node<'t>> {
    let mut late = Vec::new();
    for (child, field) in named_children_with_fields(node) {
        if binding.name_field.is_none() || field == binding.name_field {
            continue;
        }
        let is_late = field
            .is_some_and(|name| LATE_FIELDS.contains(&name) || binding.late_fields.contains(&name));
        if is_late {
            late.push(child);
        } else {
            walk(child, source, tables, state);
        }
    }
    late
}

/// Frame kinds the names of a `binding_kind` node hoist past — empty
/// unless the language declares a matching
/// [`crate::refactor::tables::HoistRule`] (PEP 572 walrus).
fn hoist_past_for(
    binding_kind: &'static str,
    scopes: &'static ScopeKinds,
) -> &'static [&'static str] {
    scopes
        .hoist_rules
        .iter()
        .find(|rule| rule.binding_kind == binding_kind)
        .map_or(&[], |rule| rule.transparent_frame_kinds)
}

/// Binds identifiers from the `field` child subtree of `node` into the
/// innermost frame.
fn bind_field(
    node: Node<'_>,
    field: &'static str,
    source: &[u8],
    tables: WalkTables,
    state: &mut WalkState,
) {
    if let Some(subtree) = node.child_by_field_name(field) {
        collect_bound_names(subtree, source, tables, state);
    }
}

/// Binds identifiers from the `field` subtree, or from the whole node
/// when the binding spec has no name field.
fn bind_field_or_node(
    node: Node<'_>,
    field: Option<&'static str>,
    source: &[u8],
    tables: WalkTables,
    state: &mut WalkState,
) {
    match field {
        Some(name) => bind_field(node, name, source, tables, state),
        None => collect_bound_names(node, source, tables, state),
    }
}

/// Recursively collects bindable identifiers under `node` into the
/// innermost frame. Opaque targets (member / index assignment) bind
/// nothing but still read their object expression, and describing
/// fields (types, defaults) are walked as references — `def g(a=b)`
/// binds `a` while reading `b`.
fn collect_bound_names(node: Node<'_>, source: &[u8], tables: WalkTables, state: &mut WalkState) {
    if BINDING_OPAQUE_KINDS.contains(&node.kind()) {
        walk_children(node, source, tables, state);
        return;
    }
    if node.named_child_count() == 0 {
        bind_leaf(node, source, tables, state);
        return;
    }
    for (child, field) in named_children_with_fields(node) {
        if field.is_some_and(|name| BINDING_SKIP_FIELDS.contains(&name)) {
            walk(child, source, tables, state);
        } else {
            collect_bound_names(child, source, tables, state);
        }
    }
}

/// Inserts one leaf identifier's text into the innermost frame the
/// current binding does not hoist past — hoisting bindings (PEP 572
/// walrus) skip transparent comprehension frames and land in the
/// nearest opaque frame. Only identifier-kind leaves bind — keywords,
/// modifiers, and literals inside a binding region are not names.
fn bind_leaf(node: Node<'_>, source: &[u8], tables: WalkTables, state: &mut WalkState) {
    let kind = node.kind();
    if !tables.references.reference_kinds.contains(&kind)
        && !tables.references.bindable_kinds.contains(&kind)
    {
        return;
    }
    let Some(name) = node_text(node, source) else {
        return;
    };
    let hoist_past = state.hoist_past;
    let target = state
        .frames
        .iter_mut()
        .rev()
        .find(|frame| !frame.kind.is_some_and(|kind| hoist_past.contains(&kind)));
    if let Some(frame) = target {
        let _known = frame.names.insert(name);
    }
}

/// Records a reference node as free when no frame declares it and no
/// skip rule marks the position as a non-reference
/// ([AUTOFIX-EXTRACT-FREE-VARS] step 4).
fn record_reference(
    node: Node<'_>,
    source: &[u8],
    references: &'static ReferenceTable,
    state: &mut WalkState,
) {
    if reference_is_skipped(node, references) {
        return;
    }
    let Some(name) = node_text(node, source) else {
        return;
    };
    if state
        .frames
        .iter()
        .any(|frame| frame.names.contains(&name))
    {
        return;
    }
    if !state.free.contains(&name) {
        state.free.push(name);
    }
}

/// Applies the per-language skip rules to a candidate reference node —
/// shared with rule 6's read-after scan so both classify identifier
/// positions identically ([AUTOFIX-EXTRACT-PRECONDITIONS]).
pub(crate) fn reference_is_skipped(node: Node<'_>, references: &'static ReferenceTable) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    if references.skip_parent_kinds.contains(&parent.kind()) {
        return true;
    }
    let field = field_of(node, parent);
    if field.is_some_and(|name| references.skip_fields.contains(&name)) {
        return true;
    }
    references
        .skip_parent_fields
        .iter()
        .any(|(kind, skip_field)| *kind == parent.kind() && field == Some(*skip_field))
}

/// Returns the field name `parent` assigns to `node`, if any.
fn field_of<'t>(node: Node<'t>, parent: Node<'t>) -> Option<&'static str> {
    named_children_with_fields(parent)
        .into_iter()
        .find(|(child, _)| child.id() == node.id())
        .and_then(|(_, field)| field)
}

/// Returns `(child, field_name)` pairs for every named child of `node`
/// in grammar order.
fn named_children_with_fields(node: Node<'_>) -> Vec<(Node<'_>, Option<&'static str>)> {
    let mut pairs = Vec::with_capacity(node.named_child_count());
    let count = u32::try_from(node.child_count()).unwrap_or(u32::MAX);
    for index in 0..count {
        let Some(child) = node.child(index) else {
            continue;
        };
        if child.is_named() {
            pairs.push((child, node.field_name_for_child(index)));
        }
    }
    pairs
}
