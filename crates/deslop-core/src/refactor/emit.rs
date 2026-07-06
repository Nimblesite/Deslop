//! Language-agnostic assembly of the extract-method edit plan
//! ([AUTOFIX-EXTRACT-WORKSPACE-EDIT]).
//!
//! Per-language emitters produce the textual method declaration and
//! call form; this module turns them into one deterministic,
//! descending-ordered edit list the LSP layer maps 1:1 onto a
//! `WorkspaceEdit`. Emitter output is byte-for-byte deterministic per
//! cluster id — required for golden tests.

use tree_sitter::Node;

use crate::refactor::preconditions::OccurrenceScope;

/// Everything a language plugin needs to emit the extract-method text
/// ([AUTOFIX-EXTRACT-EMITTER]).
#[derive(Debug)]
pub struct EmitRequest<'t, 'a> {
    /// Full source bytes of the (single) file being rewritten.
    pub source: &'a [u8],
    /// Stable cluster id; the deterministic method name derives from
    /// its first six characters.
    pub cluster_id: &'a str,
    /// Free variables of the occurrence block, in first-reference
    /// order ([AUTOFIX-EXTRACT-FREE-VARS]).
    pub free_variables: &'a [String],
    /// Per-occurrence statement runs and scopes, ascending by offset.
    pub scopes: &'a [OccurrenceScope<'t>],
}

/// A language emitter's textual result: where the helper goes, its
/// full text, and the call that replaces every occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitOutcome {
    /// Deterministic helper name ([AUTOFIX-EXTRACT-EMITTER] naming).
    pub method_name: String,
    /// Byte offset at which `insertion_text` is inserted
    /// ([AUTOFIX-EXTRACT-DESTINATION]).
    pub insertion_offset: usize,
    /// Helper declaration text (plus any required type alias).
    pub insertion_text: String,
    /// Call-site text replacing each occurrence's statement span.
    pub call_text: String,
}

/// One planned text replacement, byte-addressed. Insertions carry
/// `start_byte == end_byte`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedEdit {
    /// Inclusive start of the replaced span.
    pub start_byte: usize,
    /// Exclusive end of the replaced span.
    pub end_byte: usize,
    /// Replacement text.
    pub new_text: String,
}

/// The complete, apply-ready extract-method refactor for one cluster
/// ([AUTOFIX-EXTRACT]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractMethodPlan {
    /// Deterministic helper name (contains the cluster-id prefix).
    pub method_name: String,
    /// Free variables that became the parameter list.
    pub free_variables: Vec<String>,
    /// Edits in descending start order so earlier edits never shift
    /// later offsets ([AUTOFIX-EXTRACT-WORKSPACE-EDIT]).
    pub edits: Vec<PlannedEdit>,
}

impl ExtractMethodPlan {
    /// Applies the plan to `source`, returning the rewritten bytes.
    /// Used by golden tests and preview surfaces; the LSP applies the
    /// same edits through `workspace/applyEdit` instead.
    #[must_use]
    pub fn apply_to(&self, source: &[u8]) -> Vec<u8> {
        let mut output = source.to_vec();
        for edit in &self.edits {
            let Some(tail) = output.get(edit.end_byte..).map(<[u8]>::to_vec) else {
                continue;
            };
            output.truncate(edit.start_byte);
            output.extend_from_slice(edit.new_text.as_bytes());
            output.extend_from_slice(&tail);
        }
        output
    }
}

/// Assembles the final plan from an emitter outcome: one insertion plus
/// one call-site replacement per occurrence, sorted descending.
#[must_use]
pub fn assemble_plan(
    outcome: EmitOutcome,
    scopes: &[OccurrenceScope<'_>],
    free_variables: Vec<String>,
) -> ExtractMethodPlan {
    let mut edits: Vec<PlannedEdit> = scopes
        .iter()
        .map(|scope| {
            let span = scope.span();
            PlannedEdit {
                start_byte: span.start,
                end_byte: span.end,
                new_text: outcome.call_text.clone(),
            }
        })
        .collect();
    edits.push(PlannedEdit {
        start_byte: outcome.insertion_offset,
        end_byte: outcome.insertion_offset,
        new_text: outcome.insertion_text,
    });
    edits.sort_unstable_by_key(|edit| std::cmp::Reverse(edit.start_byte));
    ExtractMethodPlan {
        method_name: outcome.method_name,
        free_variables,
        edits,
    }
}

/// Leading whitespace of the line containing `offset` — the base
/// indentation for destination formatting.
#[must_use]
pub fn line_indent_at(source: &[u8], offset: usize) -> String {
    let head = source.get(..offset).unwrap_or_default();
    let line_start = head
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map_or(0, |position| position.saturating_add(1));
    source
        .get(line_start..)
        .unwrap_or_default()
        .iter()
        .take_while(|&&byte| byte == b' ' || byte == b'\t')
        .map(|&byte| char::from(byte))
        .collect()
}

/// Byte offset of the start of the line containing `offset` — used to
/// insert a helper immediately above its destination anchor.
#[must_use]
pub fn line_start_at(source: &[u8], offset: usize) -> usize {
    source
        .get(..offset)
        .unwrap_or_default()
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map_or(0, |position| position.saturating_add(1))
}

/// The verbatim source slice covered by an occurrence's statement run.
#[must_use]
pub fn run_text<'a>(source: &'a [u8], scope: &OccurrenceScope<'_>) -> &'a str {
    let span = scope.span();
    source
        .get(span.start..span.end)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .unwrap_or_default()
}

/// Deterministic six-character cluster-id prefix used in helper names.
#[must_use]
pub fn cluster_id_prefix(cluster_id: &str) -> &str {
    cluster_id.get(..6).unwrap_or(cluster_id)
}

/// True when any node in the runs contains a value-producing `return`
/// or `yield` in its own scope — nested function frames are not
/// descended ([AUTOFIX-EXTRACT-EMITTER-CSHARP] return-type rule).
#[must_use]
pub fn runs_produce_value(
    scopes: &[OccurrenceScope<'_>],
    value_return_kinds: &[&str],
    frame_kinds: &[&str],
) -> bool {
    scopes.iter().any(|scope| {
        scope
            .run
            .iter()
            .any(|node| subtree_produces_value(*node, value_return_kinds, frame_kinds))
    })
}

/// Recursive kind search for value-producing exits, stopping at nested
/// scope frames.
fn subtree_produces_value(
    node: Node<'_>,
    value_return_kinds: &[&str],
    frame_kinds: &[&str],
) -> bool {
    if frame_kinds.contains(&node.kind()) {
        return false;
    }
    if value_return_kinds.contains(&node.kind()) && node.named_child_count() > 0 {
        return true;
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .any(|child| subtree_produces_value(child, value_return_kinds, frame_kinds));
    found
}
