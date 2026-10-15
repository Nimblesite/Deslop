//! [FUSED-CONTENT-GATE-CALL-TARGET-CONTRACT] A guard belongs to the receiver it checked.

use super::super::{call_selector, covered_nodes};
use super::*;
use crate::{
    error::CoreError,
    lang::{typescript::TypeScriptParser, LanguageParser},
    state::FileRegistry,
};

const FILE_PATH: &str = "dispatcher.ts";
const METHOD: &[u8] = b"work";
const TARGET_MESSAGE: &str = "the parsed fixture contains its selected method call";
const NO_CHANGE: &str = "";
const REBIND: &str = "value = other;";
const SHADOW: &str = "let value = other;";
const DESTRUCTURE: &str = "[value] = [other];";
const OBJECT_REBIND: &str = "({ item: value } = other);";
const COMPOUND_REBIND: &str = "value += other;";
const UPDATE_REBIND: &str = "value++;";
const OTHER_WRITE: &str = "other = value;";
const PROPERTY_WRITE: &str = "value.item = other;";
const SIBLING_SHADOW: &str = "{ let value = other; }";
const UNRELATED_PARAMETER: &str = "const callback = (value: any) => value;";
const DECLARED_SHADOW: &str = "function value() { return other; }";
const STABLE_SCENARIOS: &[(&str, &str)] = &[
    (NO_CHANGE, NO_CHANGE),
    (OTHER_WRITE, NO_CHANGE),
    (PROPERTY_WRITE, NO_CHANGE),
    (SIBLING_SHADOW, NO_CHANGE),
    (UNRELATED_PARAMETER, NO_CHANGE),
    (NO_CHANGE, REBIND),
];
const REPLACED_BINDINGS: &[&str] = &[
    REBIND,
    SHADOW,
    DESTRUCTURE,
    OBJECT_REBIND,
    COMPOUND_REBIND,
    UPDATE_REBIND,
    DECLARED_SHADOW,
];
const PLAIN: (&str, &str) = (NO_CHANGE, NO_CHANGE);
const ASSIGN_RESULT: (&str, &str) = ("value = ", "");
const SHADOWED_CALL: (&str, &str) = ("((value: any) => ", ")(other)");
const UNRELATED_PREDICATE: &str = r"import { isReady } from './external';
function unused() {
  function isReady(value: unknown): value is Ready { return true; }
}
function run(value: any) {
  if (isReady(value)) { value.work(); }
}
";
const SHADOWED_PREDICATE: &str = r"function isReady(value: unknown): value is Ready { return true; }
function run(value: any, isReady: any) {
  if (isReady(value)) { value.work(); }
}
";
const EXPRESSION_SHADOW: &str = r"function isReady(value: unknown): value is Ready { return true; }
const run = function isReady(value: any) {
  if (isReady(value)) { value.work(); }
};
";
const REBOUND_PREDICATE: &str = r"function isReady(value: unknown): value is Ready { return true; }
function run(value: any, other: any) {
  isReady = other;
  if (isReady(value)) { value.work(); }
}
";
const AMBIGUOUS_PREDICATES: &[&str] = &[
    UNRELATED_PREDICATE,
    SHADOWED_PREDICATE,
    EXPRESSION_SHADOW,
    REBOUND_PREDICATE,
];

fn scenario(before: &str, after: &str, wrapper: (&str, &str)) -> String {
    let (prefix, suffix) = wrapper;
    format!(
        r"function isReady(value: unknown): value is Ready {{ return true; }}
function run(value: any, other: any) {{
  if (isReady(value)) {{
    {before}
    {prefix}value.work(){suffix};
    {after}
  }}
}}
"
    )
}

fn normalized_source(source: &str) -> Result<NormalizedNode, CoreError> {
    let mut registry = FileRegistry::new();
    let file = registry.register(FILE_PATH.into());
    TypeScriptParser::new().parse_and_normalize(source.as_bytes(), file)
}

fn selected_call(node: &NormalizedNode, source: &[u8]) -> bool {
    call_selector(node).is_some_and(|selector| {
        source.get(selector.byte_range.start..selector.byte_range.end) == Some(METHOD)
    })
}

fn has_contract(before: &str, after: &str, wrapper: (&str, &str)) -> Result<bool, CoreError> {
    let source = scenario(before, after, wrapper);
    source_has_contract(&source)
}

fn source_has_contract(source: &str) -> Result<bool, CoreError> {
    let root = normalized_source(source)?;
    let nodes = covered_nodes(&root, root.byte_range);
    let contracts = ReceiverContracts::new(&nodes, source.as_bytes());
    let target = nodes
        .iter()
        .find(|node| selected_call(node, source.as_bytes()));
    assert!(target.is_some(), "{TARGET_MESSAGE}");
    Ok(target
        .and_then(|node| receiver_contract(node, &contracts))
        .is_some())
}

#[test]
fn an_unrelated_nested_predicate_cannot_qualify_the_call() -> Result<(), CoreError> {
    for source in AMBIGUOUS_PREDICATES {
        assert!(!source_has_contract(source)?, "{source}");
    }
    Ok(())
}

#[test]
fn positive_guard_keeps_the_checked_receiver_until_it_changes() -> Result<(), CoreError> {
    for (before, after) in STABLE_SCENARIOS.iter().copied() {
        assert!(
            has_contract(before, after, PLAIN)?,
            "before={before:?}, after={after:?}"
        );
    }
    assert!(has_contract(NO_CHANGE, NO_CHANGE, ASSIGN_RESULT)?);
    Ok(())
}

#[test]
fn rebinding_or_shadowing_cannot_borrow_the_old_receiver_contract() -> Result<(), CoreError> {
    for before in REPLACED_BINDINGS {
        assert!(!has_contract(before, NO_CHANGE, PLAIN)?, "{before}");
    }
    assert!(!has_contract(NO_CHANGE, NO_CHANGE, SHADOWED_CALL)?);
    Ok(())
}
