//! Store retention ([PIPELINE-INCREMENTAL-RETENTION]).
//!
//! A full pass knows exactly which blobs its corpus can address, so it
//! is the one moment the store can be pruned without guessing:
//!
//! - **Nothing is deleted while the store is under budget.** Two
//!   classes of blob look useless and are not: an *orphan* in the
//!   current partition (source bytes gone from the corpus) is exactly
//!   the content-addressed reuse set a revert or branch switch
//!   full-hits — [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] asserts
//!   that — and a blob under **another tool version** may belong to a
//!   different binary still running against this workspace (an old LSP
//!   beside an upgraded CLI). Deleting either costs hits and can fail a
//!   concurrent writer's store, so retention leaves both alone until
//!   the budget actually demands space.
//! - **Over budget, eviction is other-version blobs first, then
//!   orphans, then oldest-first.** Other-version blobs go first because
//!   this binary can never address them; orphans next because the
//!   current corpus does not reference them. Evicting any blob is safe —
//!   the next pass that addresses it misses, rebuilds from source, and
//!   self-heals — so the budget is a hard bound, not a correctness
//!   surface.
//!
//! Every step is best-effort: an unremovable entry is skipped, never an
//! error — retention must not fail a pass that already has its corpus.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::embedding::bytes_hash;

use super::{blob_file_name, FINGERPRINT_DIR, TOOL_VERSION};

/// Total on-disk budget for the fingerprint store, enforced after every
/// full pass. ~11× the pinned benchmark corpus's measured 185.8 MiB
/// store, so ordinary repositories never see eviction; a store that
/// outgrows it sheds orphans first, then its oldest blobs, each of
/// which self-heals as a miss if ever re-addressed.
const STORE_BUDGET_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// The blob file names a full pass can address, per language partition.
/// Recorded once per processed file while the corpus is built.
#[derive(Debug, Default)]
pub struct LiveBlobs {
    /// Live blob file names keyed by language id.
    per_language: HashMap<&'static str, HashSet<String>>,
}

impl LiveBlobs {
    /// Records `source` as live under `language`, using the same
    /// bytes-in hashing as the cache key itself
    /// ([PIPELINE-INCREMENTAL]).
    pub fn record(&mut self, language: &'static str, source: &[u8]) {
        let _known = self
            .per_language
            .entry(language)
            .or_default()
            .insert(blob_file_name(&bytes_hash(source)));
    }

    /// The live file-name set for a language partition directory name.
    fn for_language(&self, language: &str) -> Option<&HashSet<String>> {
        self.per_language.get(language)
    }
}

/// One blob considered for eviction.
#[derive(Debug)]
struct BlobRecord {
    /// Absolute path of the blob file.
    path: PathBuf,
    /// File size in bytes.
    len: u64,
    /// Last-modified time; the eviction age signal.
    modified: SystemTime,
    /// How readily this blob may be evicted under budget pressure —
    /// the primary eviction sort key.
    class: EvictionClass,
}

/// Eviction precedence under budget pressure. Declaration order *is*
/// the precedence: `Ord` derives from it, and the eviction sort relies
/// on that, so reordering these variants changes the policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EvictionClass {
    /// Under a tool version this binary cannot address at all. Evicted
    /// first — but only under pressure, since another running binary
    /// may still be using it.
    OtherVersion,
    /// In the current partition, but its source bytes are no longer in
    /// the corpus. Useful only as revert reuse.
    Orphan,
    /// Addressable by the current corpus, or under another `min_nodes`
    /// partition a different invocation may still address. Evicted last,
    /// oldest first.
    Live,
}

/// Prunes the fingerprint store after a full pass: classifies every
/// blob, then enforces [`STORE_BUDGET_BYTES`] by eviction class and age.
/// Nothing is deleted while the store fits. Logs counts only.
pub fn sweep_store(cache_base: &Path, live: &LiveBlobs, min_nodes: u32) {
    let root = cache_base.join(FINGERPRINT_DIR);
    if !root.is_dir() {
        return;
    }
    let inventory = blob_inventory(&root, live, min_nodes);
    let store_bytes = inventory
        .iter()
        .fold(0_u64, |total, record| total.saturating_add(record.len));
    let class_count = |wanted: EvictionClass| {
        inventory
            .iter()
            .filter(|record| record.class == wanted)
            .count()
    };
    let other_version_blobs = class_count(EvictionClass::OtherVersion);
    let orphan_blobs = class_count(EvictionClass::Orphan);
    let evicted_blobs = enforce_budget(inventory, store_bytes, STORE_BUDGET_BYTES);
    tracing::info!(
        other_version_blobs,
        orphan_blobs,
        evicted_blobs,
        store_bytes,
        "fingerprint store swept",
    );
}

/// Collects every blob in the store with its size, age, and eviction
/// class. Only a blob in the current `(language, version, min_nodes)`
/// partition can be proven orphaned; blobs under another `min_nodes`
/// stay [`EvictionClass::Live`] because a different invocation may still
/// address them, and blobs under another tool version become
/// [`EvictionClass::OtherVersion`] rather than being deleted outright —
/// another running binary may still own them.
fn blob_inventory(root: &Path, live: &LiveBlobs, min_nodes: u32) -> Vec<BlobRecord> {
    let current_min = min_nodes.to_string();
    let mut inventory: Vec<BlobRecord> = Vec::new();
    for language_dir in directory_entries(root) {
        let live_names = directory_name(&language_dir).and_then(|name| live.for_language(name));
        for version_dir in directory_entries(&language_dir) {
            let current_version =
                directory_name(&version_dir).is_some_and(|version| version == TOOL_VERSION);
            for min_dir in directory_entries(&version_dir) {
                let provable = current_version
                    && directory_name(&min_dir).is_some_and(|name| name == current_min);
                let class = if current_version {
                    EvictionClass::Live
                } else {
                    EvictionClass::OtherVersion
                };
                collect_partition_blobs(&min_dir, class, provable, live_names, &mut inventory);
            }
        }
    }
    inventory
}

/// Appends every `.bin` blob in one partition directory under `class`.
/// `provable` is true only for the partition the current pass addressed;
/// there, a blob outside `live_names` (or any blob, when the pass holds
/// no live files for the language) is downgraded to
/// [`EvictionClass::Orphan`].
fn collect_partition_blobs(
    partition: &Path,
    class: EvictionClass,
    provable: bool,
    live_names: Option<&HashSet<String>>,
    inventory: &mut Vec<BlobRecord>,
) {
    for path in directory_entries(partition) {
        let Some(file_name) = blob_name(&path) else {
            continue;
        };
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        let orphaned = provable && !live_names.is_some_and(|set| set.contains(file_name.as_str()));
        inventory.push(BlobRecord {
            path,
            len: metadata.len(),
            modified,
            class: if orphaned {
                EvictionClass::Orphan
            } else {
                class
            },
        });
    }
}

/// Evicts blobs until the store fits `budget`: by [`EvictionClass`],
/// then oldest first within a class, path as the deterministic
/// tie-breaker. Returns the number evicted.
fn enforce_budget(mut inventory: Vec<BlobRecord>, store_bytes: u64, budget: u64) -> usize {
    let mut total = store_bytes;
    if total <= budget {
        return 0;
    }
    inventory.sort_by(|left, right| {
        (left.class, left.modified, &left.path).cmp(&(right.class, right.modified, &right.path))
    });
    let mut evicted = 0_usize;
    for record in &inventory {
        if total <= budget {
            break;
        }
        if fs::remove_file(&record.path).is_ok() {
            total = total.saturating_sub(record.len);
            evicted = evicted.saturating_add(1);
        }
    }
    evicted
}

/// Child directories of `dir`; empty on any read failure.
fn directory_entries(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| entry.ok().map(|found| found.path()))
        .collect()
}

/// The final path component as UTF-8, if it has one.
fn directory_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(std::ffi::OsStr::to_str)
}

/// The file name of a `.bin` blob, or `None` for anything else so
/// foreign files are never touched.
fn blob_name(path: &Path) -> Option<String> {
    if !path.extension().is_some_and(|extension| extension == "bin") {
        return None;
    }
    directory_name(path).map(str::to_owned)
}

#[cfg(test)]
mod tests;
