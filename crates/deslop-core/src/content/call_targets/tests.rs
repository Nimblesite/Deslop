//! [FUSED-CONTENT-GATE-CALL-TARGET] Conventional async selector edits
//! stay measured edits; an unrelated operation on the same store remains
//! a contradiction.

use std::collections::HashMap;

use super::{changed, CallTarget, CallTargetChange};
use crate::{
    ast::ByteRange,
    content::frontier::{LeafKey, MemberContent, Population},
    state::{FileId, FileRegistry},
};

const LEFT_PATH: &str = "blocking.cs";
const RIGHT_PATH: &str = "async.cs";
const RECEIVER: &str = "store";
const FOR: &str = "For";
const ASYNC_FOR: &str = "AsyncFor";
const CLEAR: &str = "Clear";
const RECEIVER_KEY: u64 = 1;
const LEFT_SELECTOR_KEY: u64 = 2;
const RIGHT_SELECTOR_KEY: u64 = 3;
const SHAPE_SIZE: usize = 32;
const SOURCE_START: usize = 0;
const SELECTOR_SEPARATOR_SIZE: usize = 1;
const RECEIVER_INDEX: usize = 0;
const EMPTY_HASH_BYTE: u8 = 0;

#[test]
fn leading_async_affix_is_a_measured_selector_edit() {
    let (left, right, sources) = target_pair(FOR, ASYNC_FOR);
    assert!(
        matches!(
            changed(&left, &right, &sources),
            Some(CallTargetChange::AsyncEdit)
        ),
        "For → AsyncFor on the same store is an async edit, not an unrelated operation"
    );
}

#[test]
fn unrelated_selector_on_the_same_receiver_is_a_contradiction() {
    let (left, right, sources) = target_pair(FOR, CLEAR);
    assert!(
        matches!(
            changed(&left, &right, &sources),
            Some(CallTargetChange::Other)
        ),
        "For → Clear on the same store changes the operation"
    );
}

fn target_pair(
    left_selector: &str,
    right_selector: &str,
) -> (MemberContent, MemberContent, HashMap<FileId, Vec<u8>>) {
    let mut registry = FileRegistry::new();
    let left_file = registry.register(LEFT_PATH.into());
    let right_file = registry.register(RIGHT_PATH.into());
    let sources = HashMap::from([
        (left_file, source(left_selector)),
        (right_file, source(right_selector)),
    ]);
    let left = member(left_file, left_selector, LEFT_SELECTOR_KEY);
    let right = member(right_file, right_selector, RIGHT_SELECTOR_KEY);
    (left, right, sources)
}

fn source(selector: &str) -> Vec<u8> {
    format!("{RECEIVER}.{selector}").into_bytes()
}

fn member(file: FileId, selector: &str, selector_key: u64) -> MemberContent {
    let selector_start = RECEIVER.len().saturating_add(SELECTOR_SEPARATOR_SIZE);
    MemberContent {
        file,
        shape: [EMPTY_HASH_BYTE; SHAPE_SIZE],
        keys: vec![key(RECEIVER_KEY), key(selector_key)],
        ranges: vec![
            ByteRange {
                start: SOURCE_START,
                end: RECEIVER.len(),
            },
            ByteRange {
                start: selector_start,
                end: selector_start.saturating_add(selector.len()),
            },
        ],
        external_calls: vec![
            None,
            Some(CallTarget {
                receiver: Some(RECEIVER_INDEX),
            }),
        ],
    }
}

fn key(value: u64) -> LeafKey {
    LeafKey {
        population: Population::Identifier,
        key: value,
        literal_group: None,
    }
}
