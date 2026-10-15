//! [FUSED-CONTENT-GATE-CALL-TARGET-CONTRACT] A receiver's copied type contract owns its vocabulary.

use super::{call_receiver, declaration_name, selector_identifier};
use crate::{
    ast::{ByteRange, NormalizedNode},
    lang::shared::IDENTIFIER_KIND,
};

mod bindings;
#[cfg(test)]
mod tests;

/// A positive conjunction establishes each of its constituent type guards.
const CONJUNCTION_KIND: &str = "__op__&&";
/// The explicit TypeScript cast production supplying a named receiver contract.
const AS_EXPRESSION_KIND: &str = "as_expression";

/// Contract declarations are indexed once per matched region, never once per call.
pub(super) struct ReceiverContracts<'nodes, 'source> {
    /// Only nodes inside the region can establish ownership.
    nodes: &'nodes [&'nodes NormalizedNode],
    /// Authored identifier bytes, selected exclusively by AST ranges.
    source: &'source [u8],
    /// Only copied declarations with a type predicate can qualify a positive guard.
    predicates: Vec<&'nodes NormalizedNode>,
}

impl<'nodes, 'source> ReceiverContracts<'nodes, 'source> {
    /// Index positive type predicates once; other languages have none.
    pub(super) fn new(nodes: &'nodes [&'nodes NormalizedNode], source: &'source [u8]) -> Self {
        let predicates: Vec<_> = nodes
            .iter()
            .copied()
            .filter(|node| {
                node.children
                    .iter()
                    .any(|child| child.kind == "type_predicate_annotation")
            })
            .collect();
        Self {
            nodes,
            source,
            predicates,
        }
    }
}

/// An explicit cast or a positive, locally declared type guard qualifying this call.
pub(super) fn receiver_contract(
    call: &NormalizedNode,
    contracts: &ReceiverContracts<'_, '_>,
) -> Option<ByteRange> {
    let receiver = call.children.first()?.children.first()?;
    cast_contract(receiver).or_else(|| guarded_contract(call, contracts))
}

/// Only a named contract is evidence; parentheses preserve its ownership.
fn cast_contract(receiver: &NormalizedNode) -> Option<ByteRange> {
    let mut receiver = receiver;
    while receiver.kind == "parenthesized_expression" {
        receiver = receiver.children.first()?;
    }
    (receiver.kind == AS_EXPRESSION_KIND)
        .then(|| selector_identifier(receiver.children.last()?).map(|node| node.byte_range))
        .flatten()
}

/// A guard qualifies calls in its consequent, never calls in the alternative branch.
fn guarded_contract(
    call: &NormalizedNode,
    contracts: &ReceiverContracts<'_, '_>,
) -> Option<ByteRange> {
    if contracts.predicates.is_empty() {
        return None;
    }
    let receiver = call_receiver(call)?;
    contracts
        .nodes
        .iter()
        .filter(|node| node.kind == "if_statement")
        .find_map(|node| consequent_contract(node, call, receiver, contracts))
}

/// The selected call must lie in the branch the positive condition establishes.
fn consequent_contract(
    node: &NormalizedNode,
    call: &NormalizedNode,
    receiver: &NormalizedNode,
    contracts: &ReceiverContracts<'_, '_>,
) -> Option<ByteRange> {
    let [condition, consequent, ..] = node.children.as_slice() else {
        return None;
    };
    if !consequent.byte_range.covers(call.byte_range)
        || !bindings::preserves_receiver(node, call, receiver, contracts.source)
    {
        return None;
    }
    positive_contract(condition, receiver, contracts)
}

/// Negation, disjunction, comparisons and unknown conditions prove no positive guard.
fn positive_contract(
    condition: &NormalizedNode,
    receiver: &NormalizedNode,
    contracts: &ReceiverContracts<'_, '_>,
) -> Option<ByteRange> {
    let mut pending = vec![condition];
    while let Some(condition) = pending.pop() {
        if condition.kind == "call_expression" {
            if let Some(contract) = called_contract(condition, receiver, contracts) {
                return Some(contract);
            }
        } else {
            extend_positive_conditions(condition, &mut pending);
        }
    }
    None
}

/// Only parentheses and conjunctions preserve positive guard ownership.
fn extend_positive_conditions<'node>(
    condition: &'node NormalizedNode,
    pending: &mut Vec<&'node NormalizedNode>,
) {
    match (condition.kind, condition.children.as_slice()) {
        ("parenthesized_expression", [inner]) => pending.push(inner),
        ("binary_expression", [left, operator, right]) if operator.kind == CONJUNCTION_KIND => {
            pending.extend([right, left]);
        }
        _ => {}
    }
}

/// The local predicate must guard exactly the receiver selected on by this call.
fn called_contract(
    call: &NormalizedNode,
    receiver: &NormalizedNode,
    contracts: &ReceiverContracts<'_, '_>,
) -> Option<ByteRange> {
    unique_contract(
        guarded_callee(call, receiver, contracts.source)?,
        call,
        contracts,
    )
}

/// A direct predicate call on exactly the selected receiver, without argument substitution.
fn guarded_callee<'call>(
    call: &'call NormalizedNode,
    receiver: &NormalizedNode,
    source: &[u8],
) -> Option<&'call NormalizedNode> {
    let [callee, arguments] = call.children.as_slice() else {
        return None;
    };
    let [argument] = arguments.children.as_slice() else {
        return None;
    };
    let argument = selector_identifier(argument)?;
    if bytes(argument, source)? != bytes(receiver, source)? {
        return None;
    }
    selector_identifier(callee)
}

/// Ambiguous declarations cannot certify a receiver contract.
fn unique_contract(
    callee: &NormalizedNode,
    call: &NormalizedNode,
    contracts: &ReceiverContracts<'_, '_>,
) -> Option<ByteRange> {
    let mut matching = contracts
        .predicates
        .iter()
        .filter(|declaration| visible_predicate(declaration, callee, call, contracts))
        .filter_map(|node| matching_contract(node, callee, contracts.source));
    let contract = matching.next()?;
    matching.next().is_none().then_some(contract)
}

/// A same-named declaration elsewhere cannot qualify a shadowed or out-of-scope call.
fn visible_predicate(
    declaration: &NormalizedNode,
    callee: &NormalizedNode,
    call: &NormalizedNode,
    contracts: &ReceiverContracts<'_, '_>,
) -> bool {
    bindings::declaration_reaches_call(declaration, call, contracts.nodes)
        && contracts.nodes.iter().all(|node| {
            node.byte_range == declaration.byte_range
                || !bindings::names_binding(node, callee, contracts.source)
        })
}

/// A copied declaration binds this predicate name to a named contract.
fn matching_contract(
    declaration: &NormalizedNode,
    callee: &NormalizedNode,
    source: &[u8],
) -> Option<ByteRange> {
    let name = declaration_name(declaration)?;
    (bytes(name, source)? == bytes(callee, source)?)
        .then(|| predicate_contract(declaration, source))
        .flatten()
}

/// A copied TypeScript predicate explicitly names the receiver's contract.
fn predicate_contract(declaration: &NormalizedNode, source: &[u8]) -> Option<ByteRange> {
    let annotation = declaration
        .children
        .iter()
        .find(|node| node.kind == "type_predicate_annotation")?;
    let predicate = annotation.children.first()?;
    let [parameter, contract] = predicate.children.as_slice() else {
        return None;
    };
    (parameter.kind == IDENTIFIER_KIND
        && contract.kind == IDENTIFIER_KIND
        && bytes(parameter, source)? == bytes(guard_parameter(declaration)?, source)?)
    .then_some(contract.byte_range)
}

/// The single guarded parameter is the single argument the caller supplied.
fn guard_parameter(declaration: &NormalizedNode) -> Option<&NormalizedNode> {
    let parameters = declaration
        .children
        .iter()
        .find(|node| node.kind == "formal_parameters")?;
    let [parameter] = parameters.children.as_slice() else {
        return None;
    };
    selector_identifier(parameter.children.first()?)
}

/// Source bytes at an AST-selected identifier, never a text search.
fn bytes<'source>(node: &NormalizedNode, source: &'source [u8]) -> Option<&'source [u8]> {
    source.get(node.byte_range.start..node.byte_range.end)
}
