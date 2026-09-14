//! Forwarding-declaration proof ([RANK-STRUCTURAL-ONLY-FORWARDING]).
//!
//! A sibling-declaration family is not defined by how many declarations
//! a fingerprint window happens to span. The meilisearch settings
//! surface — the family the `structural_only` demotion exists for — is
//! a run of one-statement API wrappers, and a window over one of them
//! covers exactly one declaration while still being family noise.
//! What makes it noise is the *data flow of its body*: one client call,
//! optionally bound and immediately returned, with nothing computed on
//! the way through. That shape is what this module proves.
//!
//! # The proof
//!
//! A member forwards when its body is one of three shapes — `return
//! <expr>;`, `<binding> = <expr>; return <reads binding>;`, or a bare
//! expression (arrow) body — every named node in those expressions
//! comes from a closed *declarative* allowlist, and **every call the
//! body makes is transport** — at least one delegating to a
//! collaborator, the rest consuming only what a delegation produced
//! ([`delegation`]). Branches, loops,
//! arithmetic, comparisons, mutation and every unknown node kind are
//! absent from the allowlist, so any body that computes anything —
//! including a parse `ERROR` — fails the proof and the cluster stays
//! visible. Failing open is the point: this predicate hides findings,
//! so it may only speak when the AST proves the member has nothing to
//! lift.
//!
//! # Why the calls must reach collaborator state
//!
//! "Contains a call" is not forwarding. `standardPrice(order) =>
//! computePrice(order, "standard", 100)` beside `premiumPrice(order) =>
//! computePrice(order, "premium", 250)` is one allowlisted call per
//! body and differs only in literals — and it is exactly the
//! parameterisable duplication this tool exists to report, because the
//! call goes back into the class's own logic. What separates the
//! meilisearch wrappers is *where their data goes*: `http.getMethod(
//! '/indexes/$uid/settings')` hands the request to the injected client
//! the class holds. Receiver resolution is the AST fact that tells the
//! two apart; a bare identifier callee, a `this.method(...)` self-call
//! and a static `Type.factory(...)` all fail it, and each of those
//! failures keeps a liftable pair on the report. Pinned by
//! `dart_forwarding_fail_open::same_class_helper_calls_are_not_forwarding`.
//!
//! One such call is not a licence for the rest of the body, either:
//! [`delegation`] holds the whole-body argument and the two fixtures
//! that pin it.
//!
//! # Why a statement count cannot stand in for this
//!
//! "One or two statements" convicts `TotalWithTax` — a loop, arithmetic
//! and an accumulator compressed into a short body — and acquits a
//! `getX()` wrapper that spends its two statements on a temporary
//! response. The allowlist asks what the statements *do*, not how many
//! there are, which is the same distinction that separates the
//! meilisearch wrappers from `csharp-merge-drift`'s `ApplyStandard` /
//! `ApplyPremium`: those bodies bind two locals, call five collaborators
//! in sequence and branch on a ceiling, so no window over them can ever
//! prove forwarding.
//!
//! Languages without a wired grammar table return `false` for every
//! member — their single-declaration windows are never suppressed.

use tree_sitter::Node;

use crate::ast::named_children;

mod delegation;

/// Per-language grammar facts the forwarding walk needs. The
/// `declarative` list is a closed allowlist: a named node kind outside
/// it — control flow, arithmetic, mutation, `ERROR` — disproves
/// forwarding, so an incomplete list can only surface more, never hide
/// more.
struct ForwardingGrammar {
    /// Outermost body node of a member declaration.
    body: &'static str,
    /// Statement-sequence node inside a braced body.
    block: &'static str,
    /// Local-binding statement (`final response = ...`).
    binding: &'static str,
    /// Return statement.
    ret: &'static str,
    /// Expression statement (a bare forwarded call, no return value).
    statement: &'static str,
    /// Call node — every one in a forwarding body must be transport.
    call: &'static str,
    /// Argument-list node of a call.
    arguments: &'static str,
    /// Identifier node, for matching the binding against the return.
    identifier: &'static str,
    /// Member-access callee kinds that carry a receiver.
    member_access: &'static [&'static str],
    /// Node kinds whose first identifier declares instance state: plain
    /// field declarations and `this.x` constructor parameters.
    field_names: &'static [&'static str],
    /// Formal-parameter list node of a member signature.
    parameters: &'static str,
    /// One formal parameter; its first identifier is the name.
    parameter: &'static str,
    /// Every named node kind a declarative body may contain.
    declarative: &'static [&'static str],
}

/// tree-sitter-dart grammar facts. Dart names class members generically
/// — there is no `method_declaration` — so the body node is the anchor
/// and the allowlist is what separates a wrapper from a computation.
const DART: ForwardingGrammar = ForwardingGrammar {
    body: "function_body",
    block: "block",
    binding: "local_variable_declaration",
    ret: "return_statement",
    statement: "expression_statement",
    call: "call_expression",
    arguments: "arguments",
    identifier: "identifier",
    member_access: &["member_expression", "null_aware_member_expression"],
    field_names: &["initialized_identifier", "constructor_param"],
    parameters: "formal_parameter_list",
    parameter: "formal_parameter",
    declarative: &[
        "return_statement",
        "expression_statement",
        "local_variable_declaration",
        "initialized_variable_definition",
        "call_expression",
        "member_expression",
        "null_aware_member_expression",
        "instantiation_expression",
        "arguments",
        "named_argument",
        "label",
        "identifier",
        "null_assertion_expression",
        "parenthesized_expression",
        "await_expression",
        "unary_expression",
        "throw_expression",
        "type_cast_expression",
        "type_cast",
        "as_operator",
        "type",
        "type_arguments",
        "type_identifier",
        "function_expression",
        "function_expression_body",
        "formal_parameter_list",
        "formal_parameter",
        "optional_formal_parameters",
        "string_literal",
        "string_literal_single_quotes",
        "string_literal_double_quotes",
        "template_chars_single_single",
        "template_chars_double_single",
        "template_substitution",
        "identifier_dollar_escaped",
        "decimal_integer_literal",
        "decimal_floating_point_literal",
        "true",
        "false",
        "null_literal",
        "list_literal",
        "set_or_map_literal",
        "pair",
        "comment",
    ],
};

/// Grammar table lookup. Only Dart is wired: the sibling-family false
/// positive this proof repairs is the Dart REST-wrapper surface, and an
/// unwired language fails open — its single-declaration windows are
/// never suppressed as a family.
const fn forwarding_grammar(language: &str) -> Option<&'static ForwardingGrammar> {
    match language.as_bytes() {
        b"dart" => Some(&DART),
        _ => None,
    }
}

/// Returns the member's **body bytes** when its body proves the
/// forwarding shape: declarative nodes only, one of the three body
/// forms, and every call proven transport — `container` supplies the
/// instance-field names delegation resolves against
/// ([RANK-STRUCTURAL-ONLY-FORWARDING]). `None` when the member is not
/// proven to forward.
///
/// The bytes are returned rather than a bare `bool` because the caller
/// must still compare wrappers against each other. Two wrappers in one
/// class that forward to the *same* route are a copy-paste bug — one of
/// the two calls is dead — and their declarations differ only in the
/// method name, so the reported windows are never byte-equal even when
/// the duplication is exact. Only the bodies can show it.
pub(super) fn forwarding_body<'a>(
    member: Node<'_>,
    container: Node<'_>,
    language: &str,
    source: &'a [u8],
) -> Option<&'a [u8]> {
    let grammar = forwarding_grammar(language)?;
    let body = outermost_kind(member, grammar.body)?;
    if !body_shape_forwards(body, grammar, source) {
        return None;
    }
    if !delegation::body_is_pure_delegation(body, container, member, grammar, source) {
        return None;
    }
    source.get(body.start_byte()..body.end_byte())
}

/// Finds the outermost descendant of `kind` — the member's own body,
/// never the body of a lambda nested inside it.
fn outermost_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    if node.kind() == kind {
        return Some(node);
    }
    named_children(node)
        .into_iter()
        .find_map(|child| outermost_kind(child, kind))
}

/// A body forwards as either a single braced block of forwarding
/// statements or a bare (arrow) declarative expression.
fn body_shape_forwards(body: Node<'_>, grammar: &ForwardingGrammar, source: &[u8]) -> bool {
    let content = named_non_comment_children(body);
    match content.as_slice() {
        [block] if block.kind() == grammar.block => block_shape_forwards(*block, grammar, source),
        [expression] => subtree_is_declarative(*expression, grammar),
        _ => false,
    }
}

/// The braced forms: `return <expr>;`, a bare forwarded call, or
/// `<binding>; return <reads binding>;`. Anything longer has room for
/// logic the proof has not read, so it fails open.
fn block_shape_forwards(block: Node<'_>, grammar: &ForwardingGrammar, source: &[u8]) -> bool {
    let statements = named_non_comment_children(block);
    match statements.as_slice() {
        [only] if only.kind() == grammar.ret || only.kind() == grammar.statement => {
            subtree_is_declarative(*only, grammar)
        }
        [binding, ret] if binding.kind() == grammar.binding && ret.kind() == grammar.ret => {
            subtree_is_declarative(*binding, grammar)
                && subtree_is_declarative(*ret, grammar)
                && return_reads_binding(*binding, *ret, grammar, source)
        }
        _ => false,
    }
}

/// True when the returned expression mentions the bound name — the
/// binding exists only to hold the forwarded call's response.
fn return_reads_binding(
    binding: Node<'_>,
    ret: Node<'_>,
    grammar: &ForwardingGrammar,
    source: &[u8],
) -> bool {
    let Some(name) = first_identifier_bytes(binding, grammar, source) else {
        return false;
    };
    subtree_mentions_identifier(ret, grammar, source, name)
}

/// The declared name is the first identifier inside the binding
/// statement — type positions use distinct type-identifier kinds.
fn first_identifier_bytes<'a>(
    node: Node<'_>,
    grammar: &ForwardingGrammar,
    source: &'a [u8],
) -> Option<&'a [u8]> {
    let identifier = outermost_kind(node, grammar.identifier)?;
    source.get(identifier.start_byte()..identifier.end_byte())
}

/// True when any identifier in `node`'s subtree equals `name`.
fn subtree_mentions_identifier(
    node: Node<'_>,
    grammar: &ForwardingGrammar,
    source: &[u8],
    name: &[u8],
) -> bool {
    if node.kind() == grammar.identifier
        && source.get(node.start_byte()..node.end_byte()) == Some(name)
    {
        return true;
    }
    named_children(node)
        .into_iter()
        .any(|child| subtree_mentions_identifier(child, grammar, source, name))
}

/// True when every named node in the subtree is on the declarative
/// allowlist. Unknown kinds — including `ERROR` — disprove forwarding.
fn subtree_is_declarative(node: Node<'_>, grammar: &ForwardingGrammar) -> bool {
    if !grammar.declarative.contains(&node.kind()) {
        return false;
    }
    named_children(node)
        .into_iter()
        .all(|child| subtree_is_declarative(child, grammar))
}

/// Named children with comments removed — comments never carry logic.
fn named_non_comment_children(node: Node<'_>) -> Vec<Node<'_>> {
    named_children(node)
        .into_iter()
        .filter(|child| child.kind() != "comment")
        .collect()
}
