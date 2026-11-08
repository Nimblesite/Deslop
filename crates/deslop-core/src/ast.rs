//! Normalised AST representation shared across languages.
//!
//! Implements [PIPELINE-NORMALIZE-AST]: every language parser emits a
//! `NormalizedNode` tree with identical shape and semantics so that downstream
//! pipeline stages ([PIPELINE-FINGERPRINT-MERKLE],
//! [PIPELINE-CLUSTER-EXACT]) are completely language-agnostic.
//!
//! ## Invariants
//!
//! - `kind` is a stable, `'static` string. Two subtrees with the same logical
//!   structure and node kinds hash identically across runs.
//! - `byte_range` is an absolute byte offset into the original source file.
//!   This is the canonical location — line numbers are never stored, only
//!   computed at render time.
//! - Identifier, literal, comment, and whitespace nodes are collapsed by the
//!   per-language normaliser so Type-2 clones (renamed variables) produce
//!   identical trees.
//! - `children` sit in source order and never overlap, as tree-sitter emits
//!   them. Every descent by byte range ([`NormalizedNode::child_covering`],
//!   [`NormalizedNode::children_touching`]) binary-searches on that order.

use tree_sitter::Node;

use crate::state::FileId;

/// Half-open byte range `[start, end)` into a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteRange {
    /// Inclusive start offset in bytes.
    pub start: usize,
    /// Exclusive end offset in bytes.
    pub end: usize,
}

impl ByteRange {
    /// Number of bytes spanned by this range. Returns 0 when
    /// `end <= start` (saturating semantics).
    #[must_use]
    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// True when this range covers `inner` and is wider on at least
    /// one side.
    #[must_use]
    pub const fn strictly_encloses(self, inner: Self) -> bool {
        self.start <= inner.start
            && inner.end <= self.end
            && (self.start < inner.start || inner.end < self.end)
    }

    /// True when `self` covers every byte of `inner`, equal ranges
    /// included.
    #[must_use]
    pub const fn covers(self, inner: Self) -> bool {
        self.start <= inner.start && inner.end <= self.end
    }

    /// True when the two ranges share at least one byte.
    #[must_use]
    pub const fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// True when the two ranges share bytes but neither covers the
    /// other — each one starts or ends inside the other.
    #[must_use]
    pub const fn partially_overlaps(self, other: Self) -> bool {
        self.overlaps(other) && !self.covers(other) && !other.covers(self)
    }

    /// Returns `true` when this range spans no bytes. Paired with
    /// [`Self::len`] so clippy's `len_without_is_empty` passes.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.end <= self.start
    }
}

/// A normalised AST subtree. See the module docs for invariants.
#[derive(Debug, Clone)]
pub struct NormalizedNode {
    /// Structural node kind. Interned as `&'static str` by the language
    /// parser so equality is a pointer comparison in practice.
    pub kind: &'static str,
    /// Ordered children. Order matters for fingerprinting.
    pub children: Vec<NormalizedNode>,
    /// Byte range within the source file.
    pub byte_range: ByteRange,
    /// File this subtree belongs to.
    pub file_id: FileId,
}

impl NormalizedNode {
    /// Descends to the smallest subtree whose byte range contains
    /// `range` — the merge engine's occurrence-narrowing primitive
    /// ([AUTOFIX-MERGE]).
    ///
    /// `range` is clipped to this subtree's extent before the descent,
    /// because callers name a region of the *file* and a file is wider
    /// than its tree. Normalisation drops comments and the trailing
    /// newline, so the root spans only the code it kept
    /// ([PIPELINE-NORMALIZE-AST]); a whole-file request of
    /// `0..source.len()` is therefore contained by no node at all, and
    /// answering `None` makes the merge engine refuse a plan it should
    /// have produced. Clipping asks the question the caller meant: the
    /// smallest subtree covering the code inside `range`.
    ///
    /// A range lying wholly outside the tree clips to nothing and still
    /// yields `None`.
    #[must_use]
    pub fn smallest_covering(&self, range: ByteRange) -> Option<&Self> {
        let clipped = ByteRange {
            start: range.start.max(self.byte_range.start),
            end: range.end.min(self.byte_range.end),
        };
        (!clipped.is_empty()).then(|| self.covering(clipped))?
    }

    /// Strict-containment descent, once `range` is known to lie inside
    /// this subtree.
    fn covering(&self, range: ByteRange) -> Option<&Self> {
        if !self.byte_range.covers(range) {
            return None;
        }
        self.child_covering(range)
            .and_then(|child| child.covering(range))
            .or(Some(self))
    }

    /// The child whose range covers `range`, by binary search: children
    /// sit in source order and never overlap, so at most one can. A
    /// descent through a node with thousands of children — a generated
    /// parse table — costs its depth, not its width.
    #[must_use]
    pub fn child_covering(&self, range: ByteRange) -> Option<&Self> {
        let index = self
            .children
            .partition_point(|child| child.byte_range.end <= range.start);
        self.children
            .get(index)
            .filter(|child| child.byte_range.covers(range))
    }

    /// The children sharing at least one byte with `range`, one
    /// contiguous run found by binary search, in source order.
    #[must_use]
    pub fn children_touching(&self, range: ByteRange) -> &[Self] {
        let start = self
            .children
            .partition_point(|child| child.byte_range.end <= range.start);
        let touching = self.children.get(start..).unwrap_or(&[]);
        let count = touching.partition_point(|child| child.byte_range.start < range.end);
        touching.get(..count).unwrap_or(&[])
    }

    /// Total number of nodes in this subtree, including `self`. Computed
    /// bottom-up by the caller and cached alongside the fingerprint.
    #[must_use]
    pub fn subtree_node_count(&self) -> usize {
        self.children
            .iter()
            .map(Self::subtree_node_count)
            .fold(1_usize, usize::saturating_add)
    }
}

impl Drop for NormalizedNode {
    fn drop(&mut self) {
        let mut stack = std::mem::take(&mut self.children);
        while let Some(mut node) = stack.pop() {
            stack.append(&mut node.children);
        }
    }
}

/// Named children of `node` in source order.
///
/// The one place the crate spells the tree-sitter cursor dance. Every
/// raw-tree walk — language parsing, the merge engine, the cluster
/// noise filters — reads children through here, so a walk never
/// open-codes `node.walk()` and the borrow of the cursor never leaks
/// into a caller's control flow.
pub(crate) fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

/// Named children of `node` sharing at least one byte with `range`, in
/// source order. One cursor pass that stops at the first child starting
/// past `range`, materialising only the children it touches, so a
/// range-bounded walk through a node with thousands of children costs
/// the children before and inside the range, never a `Vec` of them all.
/// (tree-sitter's `first_named_child_for_byte` is not used: it answers
/// `None` when the byte falls inside an anonymous child such as a
/// leading keyword.)
pub(crate) fn named_children_touching(node: Node<'_>, range: ByteRange) -> Vec<Node<'_>> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    if !cursor.goto_first_child() {
        return out;
    }
    loop {
        let child = cursor.node();
        if child.start_byte() >= range.end {
            break;
        }
        if child.is_named() && child.end_byte() > range.start {
            out.push(child);
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::missing_docs_in_private_items)]
mod tests {
    use super::*;

    /// Children in source order, as every language parser emits them.
    const SIBLING_RANGES: [(usize, usize); 4] = [(0, 10), (10, 10), (12, 20), (30, 40)];
    /// Inside the third sibling.
    const INNER: ByteRange = ByteRange { start: 14, end: 18 };
    /// Spans the second and third siblings, so no child covers it.
    const SPANNING: ByteRange = ByteRange { start: 10, end: 20 };
    /// Lies in the gap between the third and fourth siblings.
    const GAP: ByteRange = ByteRange { start: 22, end: 26 };

    fn parent_with_siblings() -> NormalizedNode {
        let mut registry = crate::state::FileRegistry::new();
        let file_id = registry.register("siblings.rs".into());
        let child = |(start, end): (usize, usize)| NormalizedNode {
            kind: "child",
            children: Vec::new(),
            byte_range: ByteRange { start, end },
            file_id,
        };
        NormalizedNode {
            kind: "parent",
            children: SIBLING_RANGES.into_iter().map(child).collect(),
            byte_range: ByteRange { start: 0, end: 40 },
            file_id,
        }
    }

    #[test]
    fn child_covering_finds_the_one_child_by_range() {
        let parent = parent_with_siblings();
        assert_eq!(
            parent.child_covering(INNER).map(|child| child.byte_range),
            Some(ByteRange { start: 12, end: 20 })
        );
        assert!(parent.child_covering(SPANNING).is_none());
        assert!(parent.child_covering(GAP).is_none());
    }

    #[test]
    fn children_touching_is_the_run_sharing_bytes_with_the_range() {
        let parent = parent_with_siblings();
        let ranges = |range: ByteRange| -> Vec<(usize, usize)> {
            parent
                .children_touching(range)
                .iter()
                .map(|child| (child.byte_range.start, child.byte_range.end))
                .collect()
        };
        assert_eq!(ranges(INNER), vec![(12, 20)]);
        assert_eq!(ranges(SPANNING), vec![(12, 20)]);
        assert_eq!(ranges(GAP), Vec::<(usize, usize)>::new());
        // A zero-width sibling inside the range shares its position with
        // it, exactly as `node_intersects_range` reads a raw node.
        assert_eq!(
            ranges(ByteRange { start: 5, end: 35 }),
            vec![(0, 10), (10, 10), (12, 20), (30, 40)]
        );
    }

    /// Three one-line statements; the second spans bytes 6..11.
    const THREE_STATEMENTS: &str = "a = 1\nb = 2\nc = 3\n";
    const SECOND_STATEMENT: ByteRange = ByteRange { start: 6, end: 11 };
    const WHOLE_SOURCE: ByteRange = ByteRange { start: 0, end: 17 };
    const PAST_THE_END: ByteRange = ByteRange { start: 17, end: 20 };

    /// Parses `source` with `language`, or names what did not load.
    fn parsed(
        language: &tree_sitter::Language,
        source: &str,
    ) -> Result<tree_sitter::Tree, Box<dyn std::error::Error>> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(language)?;
        parser
            .parse(source, None)
            .ok_or_else(|| "parse produced no tree".into())
    }

    #[test]
    fn named_children_touching_matches_the_filtered_full_walk(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tree = parsed(&tree_sitter_python::LANGUAGE.into(), THREE_STATEMENTS)?;
        let root = tree.root_node();
        for range in [
            SECOND_STATEMENT,
            WHOLE_SOURCE,
            PAST_THE_END,
            ByteRange { start: 3, end: 8 },
        ] {
            let touching: Vec<(usize, usize)> = named_children_touching(root, range)
                .iter()
                .map(|child| (child.start_byte(), child.end_byte()))
                .collect();
            let filtered: Vec<(usize, usize)> = named_children(root)
                .into_iter()
                .filter(|child| child.start_byte() < range.end && child.end_byte() > range.start)
                .map(|child| (child.start_byte(), child.end_byte()))
                .collect();
            assert_eq!(touching, filtered, "range {range:?}");
        }
        assert_eq!(named_children_touching(root, SECOND_STATEMENT).len(), 1);
        assert_eq!(named_children_touching(root, WHOLE_SOURCE).len(), 3);
        Ok(())
    }

    /// A Flutter widget class: the Dart grammar's class body carries
    /// annotations, declarations and punctuation in one child list.
    const DART_WIDGET: &str = "class ExampleApp extends StatelessWidget {\n  const ExampleApp({super.key});\n  @override\n  Widget build(BuildContext context) {\n    return MaterialApp(home: Text(\"alpha\"));\n  }\n}\n";

    fn assert_touching_agrees_everywhere(node: Node<'_>, range: ByteRange) {
        let touching: Vec<(usize, usize)> = named_children_touching(node, range)
            .iter()
            .map(|child| (child.start_byte(), child.end_byte()))
            .collect();
        let filtered: Vec<(usize, usize)> = named_children(node)
            .into_iter()
            .filter(|child| child.start_byte() < range.end && child.end_byte() > range.start)
            .map(|child| (child.start_byte(), child.end_byte()))
            .collect();
        assert_eq!(touching, filtered, "{} at {range:?}", node.kind());
        for child in named_children(node) {
            assert_touching_agrees_everywhere(child, range);
        }
    }

    #[test]
    fn named_children_touching_agrees_on_every_node_of_a_dart_class(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tree = parsed(&tree_sitter_dart::LANGUAGE.into(), DART_WIDGET)?;
        let whole = ByteRange {
            start: 0,
            end: DART_WIDGET.len(),
        };
        let build_method = ByteRange {
            start: 90,
            end: 160,
        };
        for range in [whole, build_method] {
            assert_touching_agrees_everywhere(tree.root_node(), range);
        }
        Ok(())
    }

    #[test]
    fn byte_range_len_and_is_empty_agree_across_the_boundary() {
        let spanning = ByteRange { start: 2, end: 7 };
        assert_eq!(spanning.len(), 5);
        assert!(!spanning.is_empty());
        let zero_width = ByteRange { start: 4, end: 4 };
        assert_eq!(zero_width.len(), 0);
        assert!(zero_width.is_empty());
        // Saturating semantics: inverted ranges clamp to empty.
        let inverted = ByteRange { start: 9, end: 3 };
        assert_eq!(inverted.len(), 0);
        assert!(inverted.is_empty());
    }
}
