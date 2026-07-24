//! Per-language node-kind tables driving the refactor engine.
//!
//! The free-variable walk ([AUTOFIX-EXTRACT-FREE-VARS]) and the
//! precondition checks ([AUTOFIX-EXTRACT-PRECONDITIONS]) are
//! language-agnostic; everything language-specific is declared through
//! these tables on the [`crate::lang::LanguageParser`] trait — the same
//! single extension point as parsing.

/// One binding-introducing node pattern for the free-variable walk
/// ([AUTOFIX-EXTRACT-FREE-VARS] step 3).
///
/// When the walk enters a node of `node_kind`, the names bound are the
/// identifier-kind nodes inside the `name_field` child subtree (or the
/// whole node when `name_field` is `None`). Value-side subtrees are
/// walked as references *before* the names bind, matching runtime
/// evaluation order (`x = x + 1` reads the outer `x`).
#[derive(Debug, Clone, Copy)]
pub struct BindingKind {
    /// Tree-sitter node kind that introduces the binding.
    pub node_kind: &'static str,
    /// Child field holding the bound name(s); `None` binds identifiers
    /// from the whole node.
    pub name_field: Option<&'static str>,
    /// Child fields walked *after* the names bind, in addition to the
    /// walk's global late fields — a Rust match arm's `value` runs with
    /// its pattern in scope, unlike an assignment's `value`.
    pub late_fields: &'static [&'static str],
}

/// Scope-frame node pattern for the free-variable walk. Frames open at
/// nested function-like constructs so their parameters and locals do
/// not leak into the enclosing block's free-variable list.
#[derive(Debug, Clone, Copy)]
pub struct FrameKind {
    /// Tree-sitter node kind that opens a new scope frame.
    pub node_kind: &'static str,
    /// Child field whose identifiers bind *inside* the new frame
    /// (lambda / closure parameter lists).
    pub bind_inside_field: Option<&'static str>,
    /// Child field whose identifiers bind in the *enclosing* frame
    /// (a nested function's own name).
    pub bind_outside_field: Option<&'static str>,
    /// Child kinds walked *before* the frame's remaining children —
    /// Python comprehension clauses appear textually after the body
    /// but bind first (`[x for x in xs]` binds `x` before the body
    /// reads it).
    pub bind_first_kinds: &'static [&'static str],
}

/// Identifier-reference recognition table for the free-variable walk
/// ([AUTOFIX-EXTRACT-FREE-VARS] step 4). Declares which node kinds are
/// variable references and which syntactic positions are *not*
/// references (member names, type positions, call targets that resolve
/// as methods).
#[derive(Debug, Clone, Copy)]
pub struct ReferenceTable {
    /// Node kinds that read or write a variable by name.
    pub reference_kinds: &'static [&'static str],
    /// Extra leaf kinds that bind a name without ever being a
    /// reference (C#'s `implicit_parameter` in `order => …`).
    pub bindable_kinds: &'static [&'static str],
    /// Parent node kinds under which an identifier is never a variable
    /// reference (e.g. `generic_name`, `scoped_identifier`).
    pub skip_parent_kinds: &'static [&'static str],
    /// `(parent_kind, child_field)` pairs whose identifier child is not
    /// a variable reference (e.g. `("member_access_expression", "name")`).
    pub skip_parent_fields: &'static [(&'static str, &'static str)],
    /// Child field names that always hold non-reference identifiers
    /// regardless of parent kind (e.g. `"type"`).
    pub skip_fields: &'static [&'static str],
}

/// Shared empty table returned by the [`crate::lang::LanguageParser`]
/// default implementation — languages without refactor support
/// recognise no references, so every walk yields an empty free list.
pub const EMPTY_REFERENCE_TABLE: ReferenceTable = ReferenceTable {
    reference_kinds: &[],
    bindable_kinds: &[],
    skip_parent_kinds: &[],
    skip_parent_fields: &[],
    skip_fields: &[],
};

/// A binding node kind whose names bind past *transparent* frames into
/// the nearest enclosing opaque frame — PEP 572's walrus inside a
/// comprehension binds in the containing function or module scope, not
/// the comprehension's own frame ([AUTOFIX-EXTRACT-FREE-VARS]).
#[derive(Debug, Clone, Copy)]
pub struct HoistRule {
    /// Tree-sitter node kind of the hoisting binding (`named_expression`).
    pub binding_kind: &'static str,
    /// Frame node kinds the binding hoists past (comprehension kinds).
    pub transparent_frame_kinds: &'static [&'static str],
}

/// Container/scope kinds for [AUTOFIX-EXTRACT-PRECONDITIONS] rules 4–5
/// and the free-variable walk's frame handling.
#[derive(Debug, Clone, Copy)]
pub struct ScopeKinds {
    /// Node kinds whose named children are statements — an occurrence
    /// must cover a contiguous run of these children (rule 5).
    pub statement_container_kinds: &'static [&'static str],
    /// Function-like enclosing-scope kinds (rule 4).
    pub function_kinds: &'static [&'static str],
    /// Shared-parent kinds one level up (rule 4): C# containing class,
    /// Rust `impl`/module, Python class or module. The parse root
    /// qualifies when its kind is listed (module-level languages).
    pub shared_parent_kinds: &'static [&'static str],
    /// Nested-scope kinds that open a frame during the free-variable
    /// walk (lambdas, closures, comprehensions, local functions).
    pub frame_kinds: &'static [FrameKind],
    /// Whether an occurrence directly at module top level satisfies the
    /// enclosing-scope rule (Python: yes).
    pub allow_module_top_level: bool,
    /// Binding kinds that hoist past transparent frames
    /// ([AUTOFIX-EXTRACT-FREE-VARS] — PEP 572 walrus).
    pub hoist_rules: &'static [HoistRule],
    /// Frame kinds whose bodies run at *call time*, not where they sit
    /// in the source — in late-binding languages (Python) a body
    /// defined before a span can still read the span's bindings after
    /// it, so rule 6 must scan these bodies wherever they appear
    /// ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 6). Empty for languages
    /// whose compilers reject use-before-declaration (C#, Rust, Dart).
    pub deferred_frame_kinds: &'static [&'static str],
    /// Declaration kinds that re-bind names to an *enclosing* scope
    /// (Python `global`/`nonlocal`): inside a deferred body their names
    /// read past the body's own frame, so rule 6 treats them as free.
    pub scope_escape_kinds: &'static [&'static str],
    /// Variable-writing node patterns. Rule 7 refuses extracts whose
    /// free variables are written inside the span — the helper would
    /// mutate its own parameter copy ([AUTOFIX-EXTRACT-PRECONDITIONS],
    /// issue #280) — and merge check D refuses written holes and
    /// context parameters ([AUTOFIX-MERGE-SAFETY]).
    pub write_kinds: &'static [WriteKind],
    /// Statement kinds whose meaning changes when the span relocates
    /// to the emitter's destination scope (Python `nonlocal` — a
    /// module-scope helper has no enclosing function binding). A span
    /// containing one refuses ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 7).
    pub relocation_unsafe_kinds: &'static [&'static str],
}

/// One variable-writing node pattern for rule 7 and merge check D
/// ([AUTOFIX-EXTRACT-PRECONDITIONS] issue #280, [AUTOFIX-MERGE-SAFETY]).
///
/// A node of `node_kind` writes the name(s) in its target — the
/// `target_field` child, or the node itself when `None` (grammars that
/// give the operand no field: C# `total++`, `out total`). When
/// `marker_tokens` is non-empty the node writes only if one of those
/// token kinds appears among its children (`++`/`--` under a unary
/// expression, `ref`/`out` under an argument). A target matches by
/// exact text for bare identifiers, or — when its kind is listed in
/// `destructuring_kinds` — by any named leaf under it (C# tuple
/// deconstruction, Dart pattern assignment). Other composite targets
/// (subscripts, member accesses) never match: they mutate the object a
/// parameter copy still shares.
#[derive(Debug, Clone, Copy)]
pub struct WriteKind {
    /// Tree-sitter node kind that writes a variable.
    pub node_kind: &'static str,
    /// Child field holding the written target; `None` targets the node
    /// itself.
    pub target_field: Option<&'static str>,
    /// Token kinds one of which must appear among the node's children
    /// for it to write. Empty means the node always writes.
    pub marker_tokens: &'static [&'static str],
    /// Target kinds whose named leaves are all written.
    pub destructuring_kinds: &'static [&'static str],
}

/// One boundary-crossing statement pattern for [AUTOFIX-MERGE-SAFETY]
/// check B: a node of `node_kind` inside a merge candidate refuses the
/// merge unless one of `allowed_containers` encloses it *within* the
/// candidate span (a `break` inside its own loop is fine; a `return`
/// never is).
#[derive(Debug, Clone, Copy)]
pub struct BoundaryKind {
    /// Tree-sitter node kind that transfers control.
    pub node_kind: &'static str,
    /// Enclosing kinds that neutralise the transfer when fully inside
    /// the span. Empty means the kind always crosses the boundary.
    pub allowed_containers: &'static [&'static str],
}

/// Per-language tables for the mechanical merge ([AUTOFIX-MERGE]).
#[derive(Debug, Clone, Copy)]
pub struct MergeTables {
    /// Control-transfer patterns for safety check B
    /// ([AUTOFIX-MERGE-SAFETY]).
    pub boundary_kinds: &'static [BoundaryKind],
    /// Raw literal node kind → declared parameter type
    /// ([AUTOFIX-MERGE-NAMES] type backstop).
    pub literal_types: &'static [(&'static str, &'static str)],
    /// Whether the language supports default parameter values
    /// ([AUTOFIX-MERGE-DEFAULTS]).
    pub supports_default_parameters: bool,
}
