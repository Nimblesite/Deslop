//! Accuracy quarantine for [CLONE-NOISE-LITERAL-VARIATION-CALLS] data flow.
//!
//! The deleted walkers missed Rust `let` bindings and plain method receivers.
//! Registry construction therefore appeared unrelated to later registrations,
//! and varying path payloads were published as duplicate implementation.
//! The CLI test `registry_call_payload_variation_keeps_only_authored_control`
//! pins the false positive beside a visible, fully asserted authored clone.
//! AGENTS.md requires these panics rather than a repair or silent fallback.

use tree_sitter::Node;

/// Quarantined result-binding lookup; its allowlist omitted Rust let patterns.
#[allow(clippy::panic, reason = "mandated accuracy quarantine")]
pub(super) fn assigned_binding(_call: Node<'_>, _source: &[u8]) -> Option<Vec<u8>> {
    panic!("accuracy quarantine: call-result bindings omitted Rust let declarations")
}

/// Quarantined consumption lookup; argument-only traversal lost method receivers.
#[allow(clippy::panic, reason = "mandated accuracy quarantine")]
pub(super) fn consumed_identifiers(
    _call: Node<'_>,
    _source: &[u8],
    _kinds: &[&str],
) -> Vec<Vec<u8>> {
    panic!("accuracy quarantine: call consumption omitted plain method receivers")
}
