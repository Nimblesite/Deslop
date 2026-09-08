//! [FUSED-CONTENT-GATE-CALL-TARGET] Member-call selectors name behavior, not local variables.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    frontier::{frontiers_aligned, identities_substitute, population, MemberContent, Population},
    rename::corroborated_substitution,
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

/// An external selector and the selected collaborator through which it is called.
pub(super) struct CallTarget {
    /// Frontier position of a receiver property, excluding plain receiver variables.
    collaborator: Option<usize>,
}

impl CallTarget {
    /// The same target inside a frontier that begins `offset` positions
    /// later — how a span's targets join a concatenated core
    /// ([FUSED-SHARED-SUBTREE-CORE]).
    pub(super) fn shifted(&self, offset: usize) -> Self {
        Self {
            collaborator: self
                .collaborator
                .map(|position| position.saturating_add(offset)),
        }
    }

    /// The same target inside the stretch of `len` positions starting
    /// at `first`; a collaborator outside the stretch no longer
    /// qualifies the selector ([FUSED-SHARED-SUBTREE-CORE]).
    pub(super) fn within(&self, first: usize, len: usize) -> Self {
        Self {
            collaborator: self
                .collaborator
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

/// Locates a receiver property in the same collapsed frontier as its selector.
fn target_for_receiver(receiver: Option<ByteRange>, leaves: &[ByteRange]) -> CallTarget {
    CallTarget {
        collaborator: receiver.and_then(|range| leaves.iter().position(|leaf| *leaf == range)),
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

/// A copied collaborator property can rename the operation selected through it.
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
    let receiver = collaborator_property(call).map(|node| node.byte_range);
    Some(((selector.start, selector.end), receiver))
}

/// External selectors obey the same positional/substitution rule as behavior-bearing operators.
pub(super) fn contradicts(left: &MemberContent, right: &MemberContent) -> bool {
    if !frontiers_aligned(left, right) {
        return identities_substitute(external_identities(left), external_identities(right));
    }
    let identifiers = population(&left.keys, &right.keys, Population::Identifier);
    left.keys
        .iter()
        .zip(&right.keys)
        .zip(left.external_calls.iter().zip(&right.external_calls))
        .any(|((left_key, right_key), (left_target, right_target))| {
            left_key.key != right_key.key
                && left_target
                    .as_ref()
                    .zip(right_target.as_ref())
                    .is_some_and(|targets| {
                        !collaborators_rename(left, right, targets, &identifiers)
                    })
        })
}

/// Matching receiver-property positions must demonstrate a repeated bijective substitution.
fn collaborators_rename(
    left: &MemberContent,
    right: &MemberContent,
    targets: (&CallTarget, &CallTarget),
    identifiers: &[(u64, u64)],
) -> bool {
    let Some((left_index, right_index)) = targets.0.collaborator.zip(targets.1.collaborator) else {
        return false;
    };
    left_index == right_index
        && left
            .keys
            .get(left_index)
            .zip(right.keys.get(right_index))
            .is_some_and(|(left, right)| {
                corroborated_substitution(identifiers, (left.key, right.key))
            })
}

/// The fixed call-target identities, excluding receivers and authored callable declarations.
fn external_identities(member: &MemberContent) -> impl Iterator<Item = u64> + '_ {
    member
        .keys
        .iter()
        .zip(&member.external_calls)
        .filter(|(_, external)| external.is_some())
        .map(|(key, _)| key.key)
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

/// A receiver property is a collaborator; a plain variable is only its local handle.
fn collaborator_property(call: &NormalizedNode) -> Option<&NormalizedNode> {
    let callee = call.children.first()?;
    let _selector = member_selector(callee)?;
    member_selector(callee.children.first()?)
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
