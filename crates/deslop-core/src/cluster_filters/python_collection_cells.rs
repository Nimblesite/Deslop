//! [CLONE-NOISE-PY-COLLECTION-SIBLING-CELLS] — sibling entries of one
//! collection literal reported as duplication (gh #421). At a
//! permissive `--min-nodes`, two entries of a single dict/list/set/
//! tuple literal — `"name": name` and `"arguments": arguments` on one
//! line — admit as a structural pair: the entry shape saturates while
//! the distinguishing vocabulary normalises away. Cells of one record
//! are that record's fields, not something a reader can extract.

use tree_sitter::Node;

use super::{enclosing_kind, node_contains_kind, parse_for, raw_snippet_texts_differ, Snippet};
use crate::state::FileId;

/// Python collection-literal node kinds a sibling-cell pair can
/// inhabit. Python is the one language measured (gh #421); other
/// languages fall through unchanged, per the additive filter contract.
const PYTHON_COLLECTION_KINDS: &[&str] = &["dictionary", "list", "set", "tuple"];

/// Detects a cluster whose every member is a fragment of one collection
/// literal *instance* — same file, same literal node. Suppressed only
/// when at least two members differ in raw bytes (a byte-identical
/// repeated entry is a real copy and still surfaces) and no member
/// carries a lambda (logic inside an element is extractable and keeps
/// clustering).
pub(super) fn is_collection_sibling_cell_cluster(snippets: &[Snippet<'_>]) -> bool {
    if snippets.len() < 2 {
        return false;
    }
    let homes: Option<Vec<(FileId, usize, usize)>> = snippets.iter().map(collection_home).collect();
    let Some(homes) = homes else { return false };
    let Some(first) = homes.first() else {
        return false;
    };
    homes.iter().all(|home| home == first)
        && !snippets.iter().any(carries_lambda)
        && raw_snippet_texts_differ(snippets)
}

/// The smallest collection literal containing the member's whole range,
/// when one does, as `(file, literal start byte, literal end byte)` —
/// the identity that makes two members siblings of one instance.
fn collection_home(snippet: &Snippet<'_>) -> Option<(FileId, usize, usize)> {
    let tree = parse_for(snippet)?;
    let collection = enclosing_kind(tree.root_node(), snippet.range, PYTHON_COLLECTION_KINDS)?;
    Some((
        snippet.file_id,
        collection.start_byte(),
        collection.end_byte(),
    ))
}

/// True when the member's own subtree carries a lambda.
fn carries_lambda(snippet: &Snippet<'_>) -> bool {
    let Some(tree) = parse_for(snippet) else {
        return false;
    };
    let root: Node<'_> = tree.root_node();
    let Some(member) = root.named_descendant_for_byte_range(snippet.range.start, snippet.range.end)
    else {
        return false;
    };
    node_contains_kind(member, "lambda")
}
