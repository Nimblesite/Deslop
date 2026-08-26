//! Deterministic tree shapes for the overlap tests
//! ([PIPELINE-DETERMINISM]).
//!
//! Both overlap bounds are asserted against independent reference
//! implementations, and both need the same thing to assert against: a
//! corpus of tree shapes that is identical on every run and every
//! machine, covers equal trees, relabels, insertions at every depth and
//! wholly unrelated shapes, and is described by something other than
//! the code under test. One generator serves both, so a shape that
//! catches an alignment bug is a shape the bound is also asserted on.

use super::alignment::PostNode;

/// Node kinds the generated corpus draws from. Three is enough for a
/// relabel to be distinguishable from a delete-plus-insert, and small
/// enough that generated trees actually collide on kind.
pub(super) const KINDS: [&str; 3] = ["alpha", "beta", "gamma"];

/// Fixed LCG seed — the corpus is the same on every run and on every
/// machine ([PIPELINE-DETERMINISM]).
pub(super) const SEED: u64 = 0x2545_F491_4F6C_DD1D;

/// Knuth's LCG multiplier.
const LCG_MULTIPLIER: u64 = 6_364_136_223_846_793_005;

/// Knuth's LCG increment.
const LCG_INCREMENT: u64 = 1_442_695_040_888_963_407;

/// Most children any generated node is given.
const MAX_CHILDREN: usize = 3;

/// A tree in the shape a reference implementation reads directly.
#[derive(Debug, Clone)]
pub(super) struct RefTree {
    /// Node kind.
    pub(super) kind: &'static str,
    /// Ordered children.
    pub(super) children: Vec<RefTree>,
}

/// Deterministic value source for the generated corpus.
pub(super) struct Lcg(pub(super) u64);

impl Lcg {
    /// Next value below `bound`, or zero when `bound` is zero.
    pub(super) fn below(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(LCG_MULTIPLIER)
            .wrapping_add(LCG_INCREMENT);
        let drawn = usize::try_from(self.0 >> 33).unwrap_or(0);
        drawn.checked_rem(bound).unwrap_or(0)
    }
}

/// Total nodes in a forest.
pub(super) fn forest_nodes(forest: &[RefTree]) -> usize {
    forest
        .iter()
        .map(|tree| forest_nodes(&tree.children).saturating_add(1))
        .fold(0, usize::saturating_add)
}

/// Appends `tree`'s post-order sequence to `out`, recording each node's
/// 1-based leftmost-leaf index — the input the implementation reads.
fn push_postorder(tree: &RefTree, out: &mut Vec<PostNode>) -> usize {
    let mut leftmost = None;
    for child in &tree.children {
        let child_leftmost = push_postorder(child, out);
        if leftmost.is_none() {
            leftmost = Some(child_leftmost);
        }
    }
    let own = leftmost.unwrap_or_else(|| out.len().saturating_add(1));
    out.push(PostNode {
        kind: tree.kind,
        leftmost: own,
    });
    own
}

/// The post-order sequence of one tree.
pub(super) fn postorder(tree: &RefTree) -> Vec<PostNode> {
    let mut out = Vec::new();
    let _root = push_postorder(tree, &mut out);
    out
}

/// One generated tree of at most `budget` nodes.
pub(super) fn generate(source: &mut Lcg, budget: usize) -> RefTree {
    let kind = KINDS.get(source.below(KINDS.len())).copied().unwrap_or("");
    let remaining = budget.saturating_sub(1);
    let child_count = if remaining == 0 {
        0
    } else {
        source.below(remaining.min(MAX_CHILDREN).saturating_add(1))
    };
    let mut children = Vec::new();
    let mut left = remaining;
    for _ in 0..child_count {
        if left == 0 {
            break;
        }
        let share = source.below(left).saturating_add(1);
        let child = generate(source, share);
        left = left.saturating_sub(forest_nodes(std::slice::from_ref(&child)));
        children.push(child);
    }
    RefTree { kind, children }
}
