//! Store retention ([PIPELINE-INCREMENTAL-RETENTION]).
//!
//! A full pass knows exactly which blobs its corpus can address, so it
//! is the one moment the store can be pruned without guessing:
//!
//! - **Stale tool-version partitions are always removed.** A blob under
//!   another `TOOL_VERSION` directory is unaddressable by construction
//!   and can never hit again.
//! - **Orphans are kept while the store is under budget.** A blob in
//!   the current partition whose source bytes are no longer in the
//!   corpus is exactly the content-addressed reuse set for a revert or
//!   a branch switch — [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]
//!   asserts a revert full-hits the store, so eager orphan removal
//!   would be a recall regression against that contract.
//! - **Over budget, eviction is orphans-first, then oldest-first.**
//!   Evicting a live blob is always safe — the next pass misses,
//!   rebuilds from source, and self-heals — so the budget is a hard
//!   bound, not a correctness surface.
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
    /// Whether the current pass can no longer address this blob.
    orphan: bool,
}

/// Prunes the fingerprint store after a full pass: removes stale
/// tool-version partitions, then enforces [`STORE_BUDGET_BYTES`] with
/// orphans-first, oldest-first eviction. Logs counts only.
pub fn sweep_store(cache_base: &Path, live: &LiveBlobs, min_nodes: u32) {
    let root = cache_base.join(FINGERPRINT_DIR);
    if !root.is_dir() {
        return;
    }
    let stale_partitions = remove_stale_versions(&root);
    let inventory = blob_inventory(&root, live, min_nodes);
    let store_bytes = inventory
        .iter()
        .fold(0_u64, |total, record| total.saturating_add(record.len));
    let orphan_blobs = inventory.iter().filter(|record| record.orphan).count();
    let evicted_blobs = enforce_budget(inventory, store_bytes, STORE_BUDGET_BYTES);
    tracing::info!(
        stale_partitions,
        orphan_blobs,
        evicted_blobs,
        store_bytes,
        "fingerprint store swept",
    );
}

/// Removes every `<language>/<version>` partition whose version is not
/// the running [`TOOL_VERSION`]. Returns the number removed.
fn remove_stale_versions(root: &Path) -> usize {
    directory_entries(root)
        .iter()
        .flat_map(|language_dir| directory_entries(language_dir))
        .filter(|version_dir| {
            directory_name(version_dir).is_some_and(|version| version != TOOL_VERSION)
        })
        .filter(|version_dir| fs::remove_dir_all(version_dir).is_ok())
        .count()
}

/// Collects every blob in the store with its size, age, and whether the
/// current pass can still address it. Only a blob in the current
/// `(language, version, min_nodes)` partition can be proven orphaned;
/// blobs under other `min_nodes` partitions stay age-ranked because a
/// different invocation may still address them.
fn blob_inventory(root: &Path, live: &LiveBlobs, min_nodes: u32) -> Vec<BlobRecord> {
    let current_min = min_nodes.to_string();
    let mut inventory: Vec<BlobRecord> = Vec::new();
    for language_dir in directory_entries(root) {
        let live_names = directory_name(&language_dir).and_then(|name| live.for_language(name));
        for min_dir in directory_entries(&language_dir.join(TOOL_VERSION)) {
            let provable = directory_name(&min_dir).is_some_and(|name| name == current_min);
            collect_partition_blobs(&min_dir, provable, live_names, &mut inventory);
        }
    }
    inventory
}

/// Appends every `.bin` blob in one partition directory. `provable` is
/// true only for the partition the current pass addressed; there, a
/// blob outside `live_names` (or any blob, when the pass holds no live
/// files for the language) is a provable orphan.
fn collect_partition_blobs(
    partition: &Path,
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
        let orphan = provable && !live_names.is_some_and(|set| set.contains(file_name.as_str()));
        inventory.push(BlobRecord {
            path,
            len: metadata.len(),
            modified,
            orphan,
        });
    }
}

/// Evicts blobs until the store fits `budget`: provable orphans first,
/// oldest first within each class, path as the deterministic
/// tie-breaker. Returns the number evicted.
fn enforce_budget(mut inventory: Vec<BlobRecord>, store_bytes: u64, budget: u64) -> usize {
    let mut total = store_bytes;
    if total <= budget {
        return 0;
    }
    inventory.sort_by(|left, right| {
        (!left.orphan, left.modified, &left.path).cmp(&(!right.orphan, right.modified, &right.path))
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
