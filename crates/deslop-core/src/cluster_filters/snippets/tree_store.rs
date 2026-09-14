//! The bounded parse-tree store behind [`ParseCache`]
//! ([PERF-FLUTTER-TODO-MEMORY]).
//!
//! Filters walk a real tree-sitter CST rather than the normalised tree,
//! and a corpus-scale report asks for the same files over and over.
//! Keeping every CST would cost far more than the sources themselves —
//! a parsed tree is many times its own text — so the store holds a
//! working set bounded by the source bytes it covers and evicts the
//! least recently used file past that. Eviction changes only what has
//! to be re-parsed, never an answer: a miss recomputes exactly the tree
//! a hit would have returned.

use std::{collections::HashMap, sync::Arc};

use super::PARSE_TREE_SOURCE_BUDGET_BYTES;
use crate::state::FileId;

/// Store behaviour under the budget.
#[cfg(test)]
mod tests;

/// The bounded CST cache: parsed trees with their last-use stamps and
/// the source bytes they cover — under one lock, so the three can never
/// disagree and no caller can take them in two different orders.
///
/// [PERF-FLUTTER-TODO-PAIRS] Recency is a stamp per entry rather than a
/// position in an ordered queue. The queue made a cache *hit* cost a
/// linear scan to move the entry to the back, and the noise split asks
/// once per cluster member — tens of millions of scan steps on a large
/// corpus, every one of them holding the lock every worker needs. A
/// stamp is written in place, so a hit takes the lock for a single
/// store. Eviction still picks the least recently used entry; only
/// insertions pay to find it, and an insertion has already paid for a
/// parse.
#[derive(Default)]
pub(super) struct TreeStore {
    /// Parsed CST per file, with its size and last-use stamp.
    parsed: HashMap<FileId, TreeEntry>,
    /// Sum of the entries' source bytes.
    bytes: usize,
    /// Monotonic use counter; the largest stamp is the most recent.
    clock: u64,
}

/// One cached CST with what eviction needs to know about it.
struct TreeEntry {
    /// The parsed CST (`None` when the language has no grammar here or
    /// parsing failed — as expensive to rederive as a real tree).
    tree: Option<Arc<tree_sitter::Tree>>,
    /// Source bytes this entry covers, for the budget.
    bytes: usize,
    /// Value of the store's clock when this entry was last used.
    last_used: u64,
}

/// What the store knows about one file. Three states, not two: a file
/// that was never asked for must be parsed, while a file remembered as
/// unparseable must not be — collapsing the two would reparse every
/// grammarless file on every query.
#[derive(Debug)]
pub(super) enum Lookup {
    /// Never stored, or since evicted — the caller must parse.
    Absent,
    /// Stored before. `None` is the remembered verdict that the file
    /// has no grammar here, or failed to parse.
    Remembered(Option<Arc<tree_sitter::Tree>>),
}

impl TreeStore {
    /// The cached entry for `file_id`, stamped as most recently used.
    pub(super) fn hit(&mut self, file_id: FileId) -> Lookup {
        let stamp = self.tick();
        match self.parsed.get_mut(&file_id) {
            Some(entry) => {
                entry.last_used = stamp;
                Lookup::Remembered(entry.tree.clone())
            }
            None => Lookup::Absent,
        }
    }

    /// Records a freshly parsed tree and evicts least-recently-used
    /// entries until the covered source fits the budget.
    pub(super) fn insert(
        &mut self,
        file_id: FileId,
        bytes: usize,
        tree: Option<Arc<tree_sitter::Tree>>,
    ) {
        let last_used = self.tick();
        let replaced = self.parsed.insert(
            file_id,
            TreeEntry {
                tree,
                bytes,
                last_used,
            },
        );
        self.bytes = self
            .bytes
            .saturating_sub(replaced.map_or(0, |entry| entry.bytes))
            .saturating_add(bytes);
        while self.bytes > PARSE_TREE_SOURCE_BUDGET_BYTES {
            // The just-inserted entry is never evicted: it is the most
            // recently used, so a single file larger than the whole
            // budget stays (a giant file is a legitimate working set)
            // and the loop stops once nothing older is left.
            let Some(stale) = self.least_recently_used() else {
                break;
            };
            let Some(evicted) = self.parsed.remove(&stale) else {
                break;
            };
            self.bytes = self.bytes.saturating_sub(evicted.bytes);
        }
    }

    /// The next use stamp.
    fn tick(&mut self) -> u64 {
        self.clock = self.clock.saturating_add(1);
        self.clock
    }

    /// The file holding the oldest stamp, or `None` when at most one
    /// entry is left — the entry just inserted, which never evicts.
    fn least_recently_used(&self) -> Option<FileId> {
        if self.parsed.len() <= 1 {
            return None;
        }
        self.parsed
            .iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(file_id, _)| *file_id)
    }
}
