//! [FUSED-CONTENT-GATE-CALL-TARGET] Member-call selectors name behavior, not local variables.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    frontier::{frontiers_aligned, population, MemberContent, Population},
    rename::Corroboration,
};
use crate::{
    ast::{ByteRange, NormalizedNode},
    lang::shared::IDENTIFIER_KIND,
};

/// A callable name and its following parameter node.
const DECLARATION_NEIGHBORS: usize = 2;
/// Callable declarations whose names can be renamed together with copied callers.
const CALLABLE_DECLARATIONS: &[&str] = &[
    "method_definition",
    "method_declaration",
    "function_definition",
    "function_declaration",
    "function_item",
    "function_signature",
    "function_declaration_left",
    "method_or_prop_defn",
];

/// An external selector and the collaborator it is selected on.
pub(super) struct CallTarget {
    /// Frontier position of the receiver the call is made on — the
    /// property or the handle at the head of the callee chain.
    receiver: Option<usize>,
}

impl CallTarget {
    /// The same target inside a frontier that begins `offset` positions
    /// later — how a span's targets join a concatenated core
    /// ([FUSED-SHARED-SUBTREE-CORE]).
    pub(super) fn shifted(&self, offset: usize) -> Self {
        Self {
            receiver: self
                .receiver
                .map(|position| position.saturating_add(offset)),
        }
    }

    /// The same target inside the stretch of `len` positions starting
    /// at `first`; a receiver outside the stretch no longer qualifies
    /// the selector ([FUSED-SHARED-SUBTREE-CORE]).
    pub(super) fn within(&self, first: usize, len: usize) -> Self {
        Self {
            receiver: self
                .receiver
                .filter(|position| *position >= first && position.saturating_sub(first) < len)
                .map(|position| position.saturating_sub(first)),
        }
    }
}

/// Marks frontier leaves that select methods declared outside the matched region.
pub(super) fn external_targets(
    root: &NormalizedNode,
    range: ByteRange,
    source: &[u8],
    leaves: &[ByteRange],
) -> Vec<Option<CallTarget>> {
    let nodes = covered_nodes(root, range);
    let declarations = declared_names(&nodes, source);
    let targets = external_selector_ranges(&nodes, &declarations, source);
    leaves
        .iter()
        .map(|leaf| {
            targets
                .get(&(leaf.start, leaf.end))
                .map(|receiver| target_for_receiver(*receiver, leaves))
        })
        .collect()
}

/// Locates a call's receiver in the same collapsed frontier as its selector.
fn target_for_receiver(receiver: Option<ByteRange>, leaves: &[ByteRange]) -> CallTarget {
    CallTarget {
        receiver: receiver.and_then(|range| leaves.iter().position(|leaf| *leaf == range)),
    }
}

/// Selector spans whose names are not declared in the matched region.
fn external_selector_ranges(
    nodes: &[&NormalizedNode],
    declarations: &BTreeSet<&[u8]>,
    source: &[u8],
) -> BTreeMap<(usize, usize), Option<ByteRange>> {
    nodes
        .iter()
        .filter_map(|node| external_call_target(node, declarations, source))
        .collect()
}

/// A copied collaborator can rename the operation selected on it.
fn external_call_target(
    call: &NormalizedNode,
    declarations: &BTreeSet<&[u8]>,
    source: &[u8],
) -> Option<((usize, usize), Option<ByteRange>)> {
    let selector = call_selector(call)?.byte_range;
    let name = source.get(selector.start..selector.end)?;
    if declarations.contains(name) {
        return None;
    }
    let receiver = call_receiver(call).map(|node| node.byte_range);
    Some(((selector.start, selector.end), receiver))
}

/// [FUSED-CONTENT-GATE-CALL-TARGET] Whether the two members select
/// different behaviour through the calls they make outside the matched
/// region.
///
/// A method declared elsewhere is a fixed name. The copy did not author
/// it and cannot rename it at will, so `toHaveText` standing where its
/// partner says `toContainText` is a changed operation, not a copy, and
/// it contradicts the pair's content.
///
/// [`selector_renames`] is the one reading that excuses a changed
/// selector: evidence that the *name* changed rather than the behaviour
/// ([CLONE-BUCKETS-NORTH-STAR], [TECH-PMATCH-BAKER]).
///
/// The rule reads positions, so frontiers that do not align carry no
/// verdict here. A near-miss pair is read over its aligned core instead
/// ([FUSED-SHARED-SUBTREE-CORE]): comparing the two members' method names
/// without positions cannot see a receiver, and rejected a renamed
/// near-miss copy its unrenamed twin reported at 100%.
///
/// Pinned by `type2_rename_call_targets`,
/// `js_literal_variation_calls::member_targets` and
/// `cluster_extent_statement_runs`.
pub(super) fn contradicts(left: &MemberContent, right: &MemberContent) -> bool {
    if !frontiers_aligned(left, right) {
        return false;
    }
    let corroboration =
        Corroboration::over(&population(&left.keys, &right.keys, Population::Identifier));
    left.keys
        .iter()
        .zip(&right.keys)
        .zip(left.external_calls.iter().zip(&right.external_calls))
        .any(|((left_key, right_key), (left_target, right_target))| {
            left_key.key != right_key.key
                && left_target
                    .as_ref()
                    .zip(right_target.as_ref())
                    .is_some_and(|targets| !selector_renames(left, right, targets, &corroboration))
        })
}

/// Whether a changed external selector is a rename rather than a
/// changed operation: the **receiver it is selected on** must itself be
/// a corroborated rename — the same handle or property mapped the same
/// way more than once, inside a mapping nothing contradicts
/// ([TECH-PMATCH-BAKER]) — at the same frontier position in both
/// members.
///
/// An operation swapped on the *same* collaborator changed what the code
/// does: `client.startJob` against `client.cancelJob` renames nothing
/// else and starts what it used to cancel. An operation carried to a
/// renamed collaborator is that collaborator's own vocabulary coming
/// with it: `ledger.postEntry` against `journal.writeRecord`, in a
/// function whose receiver, parameters, locals and types all rename
/// together, is one function written twice.
///
/// A call made on something that is not a name at all —
/// `expect(page.locator(..)).toHaveText(..)` — has no receiver to
/// corroborate and is never excused.
fn selector_renames(
    left: &MemberContent,
    right: &MemberContent,
    targets: (&CallTarget, &CallTarget),
    corroboration: &Corroboration,
) -> bool {
    let Some((left_index, right_index)) = targets.0.receiver.zip(targets.1.receiver) else {
        return false;
    };
    left_index == right_index
        && left
            .keys
            .get(left_index)
            .zip(right.keys.get(right_index))
            .is_some_and(|(left, right)| corroboration.admits((left.key, right.key)))
}

/// Nodes wholly covered by the reported bytes; enclosing callback headers never enter this view.
fn covered_nodes(root: &NormalizedNode, range: ByteRange) -> Vec<&NormalizedNode> {
    let mut stack = vec![root];
    let mut nodes = Vec::new();
    while let Some(node) = stack.pop() {
        if node.byte_range.start >= range.end || node.byte_range.end <= range.start {
            continue;
        }
        if range.covers(node.byte_range) {
            nodes.push(node);
        }
        stack.extend(node.children.iter().rev());
    }
    nodes
}

/// Only the terminal member selector is fixed; receivers and direct function names remain renameable.
fn call_selector(call: &NormalizedNode) -> Option<&NormalizedNode> {
    match call.kind {
        "call" | "call_expression" | "invocation_expression" | "application_expression" => {
            member_selector(call.children.first()?)
        }
        "member_call_expression" | "nullsafe_member_call_expression" | "scoped_call_expression" => {
            call.children.iter().rev().find_map(selector_identifier)
        }
        "cascade_call_expression" => call.children.iter().find_map(member_selector),
        _ => None,
    }
}

/// The collaborator a member call is made on: the property, or the
/// handle, at the head of the callee chain. A call whose head is not a
/// name at all — `expect(page.locator(..)).toHaveText(..)` — has no
/// receiver here, so nothing can excuse a changed operation on it.
fn call_receiver(call: &NormalizedNode) -> Option<&NormalizedNode> {
    let callee = call.children.first()?;
    let _selector = member_selector(callee)?;
    let head = callee.children.first()?;
    member_selector(head).or_else(|| selector_identifier(head))
}

/// The member-access productions preserved by the supported normalizers.
fn member_selector(member: &NormalizedNode) -> Option<&NormalizedNode> {
    match member.kind {
        "member_expression"
        | "null_aware_member_expression"
        | "cascade_member_expression"
        | "cascade_null_aware_member_expression"
        | "member_access_expression"
        | "field_expression"
        | "attribute"
        | "selector_expression"
        | "dot_expression" => member.children.last().and_then(selector_identifier),
        _ => None,
    }
}

/// A generic method's type arguments are not its name; a qualified selector ends at its last name.
fn selector_identifier(node: &NormalizedNode) -> Option<&NormalizedNode> {
    match node.kind {
        IDENTIFIER_KIND => Some(node),
        "generic_name" => node.children.first().and_then(selector_identifier),
        "long_identifier" | "long_identifier_or_op" | "property_or_ident" => {
            node.children.last().and_then(selector_identifier)
        }
        _ => None,
    }
}

/// Names whose declarations are themselves copied may be renamed along with their callers.
fn declared_names<'src>(nodes: &[&NormalizedNode], source: &'src [u8]) -> BTreeSet<&'src [u8]> {
    nodes
        .iter()
        .filter_map(|node| declaration_name(node))
        .filter_map(|name| source.get(name.byte_range.start..name.byte_range.end))
        .collect()
}

/// Declaration names precede their parameter list, independent of return types and modifiers.
fn declaration_name(node: &NormalizedNode) -> Option<&NormalizedNode> {
    if !CALLABLE_DECLARATIONS.contains(&node.kind) {
        return None;
    }
    node.children
        .windows(DECLARATION_NEIGHBORS)
        .find_map(|pair| {
            if parameter_list(pair.last()?.kind) {
                selector_identifier(pair.first()?)
            } else {
                None
            }
        })
}

/// Parameter and generic-parameter productions adjacent to an authored callable name.
fn parameter_list(kind: &str) -> bool {
    matches!(
        kind,
        "parameters"
            | "parameter_list"
            | "formal_parameters"
            | "formal_parameter_list"
            | "type_parameters"
            | "type_parameter_list"
            | "argument_patterns"
            | "type_arguments"
    )
}
