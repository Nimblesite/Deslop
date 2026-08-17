//! On-disk fingerprint + normalised-tree cache ([PIPELINE-INCREMENTAL],
//! [PIPELINE-INCREMENTAL-INTEGRITY]).
//!
//! Keyed by `(language_id, tool_version, min_nodes, source_byte_hash)`.
//! A cache hit rehydrates the structural fingerprints, the normalised
//! AST (kept because downstream token extraction walks it), and the
//! per-fingerprint `MinHash` signatures
//! ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]), skipping tree-sitter and
//! signature construction entirely for unchanged files.
//!
//! Every blob carries a BLAKE3 **binding digest** over its payload and
//! the full address that wrote it; a lookup recomputes the digest from
//! its *own* address before decoding, so corruption, misplacement,
//! trailing bytes, and stale semantic revisions all degrade identically
//! to a plain miss that re-parses from source and overwrites the blob.
//! The wire format, the digest, and the bounded decode live in
//! [`blob`]; this module owns the on-disk layout and the cache API.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::{
    ast::NormalizedNode, embedding::bytes_hash, fingerprint::Fingerprint, lsh::Signature,
    state::FileId,
};

use blob::{blob_len_admissible, decode, encode, BlobBinding, MAX_BLOB_BYTES};
pub use retention::{sweep_store, LiveBlobs};

mod blob;
mod retention;
#[cfg(test)]
mod tests;

/// Tool version pinned into the cache key so a `deslop-core` bump
/// (grammar pins, normalisation rules, hashing changes) invalidates
/// every previously-cached blob. Expressed via the cache directory path
/// so blobs from different versions can coexist without collision;
/// [`blob::SEMANTIC_EPOCH`] covers the revisions this string cannot.
const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Fingerprint cache directory relative to the analysis root. Shares
/// [`crate::paths::cache_dir`] with the embedding cache; a subdirectory
/// per-subsystem keeps the layout auditable.
const FINGERPRINT_DIR: &str = "fingerprints";

/// Cached pre-analysis for one file: normalised tree plus every
/// fingerprint (structural + sibling) that was extracted at the chosen
/// `min_nodes` threshold.
#[derive(Debug)]
pub struct CachedFile {
    /// Root of the normalised AST. Already rewritten to use the
    /// caller-issued [`FileId`]; safe to push straight into the
    /// pipeline.
    pub tree: NormalizedNode,
    /// Structural + sibling fingerprints extracted from `tree`.
    pub fingerprints: Vec<Fingerprint>,
    /// One `MinHash` signature per entry of `fingerprints`,
    /// positionally 1:1 ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]).
    /// Persisted so a warm pass attaches them instead of rebuilding
    /// every signature from token streams — the dominant cost of the
    /// LSH stage. The decode enforces the count invariant.
    pub signatures: Vec<Signature>,
}

/// On-disk fingerprint cache scoped to a `(language, min_nodes)`
/// partition. One instance per language per run, opened lazily through
/// [`FingerprintCache::open`]. Carries its partition identity so every
/// read and write is verified against the address that reached it
/// ([PIPELINE-INCREMENTAL-INTEGRITY]).
#[derive(Debug)]
pub struct FingerprintCache {
    /// Fully-qualified cache directory for the current partition —
    /// `<base>/fingerprints/<lang>/<ver>/<min>`.
    root: PathBuf,
    /// Parser language id of this partition.
    language_id: String,
    /// Subtree-size floor of this partition.
    min_nodes: u32,
}

impl FingerprintCache {
    /// Opens (or creates) the cache directory for `(language_id,
    /// min_nodes)` under `base`.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error when the directory tree cannot
    /// be created.
    pub fn open(base: &Path, language_id: &str, min_nodes: u32) -> io::Result<Self> {
        let root = base
            .join(FINGERPRINT_DIR)
            .join(language_id)
            .join(TOOL_VERSION)
            .join(min_nodes.to_string());
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            language_id: language_id.to_owned(),
            min_nodes,
        })
    }

    /// Returns the cached pre-analysis for `source` under `file_id`,
    /// if present, size-admissible, and bound to exactly this lookup's
    /// address. Any rejection is logged at `tracing::warn!` and treated
    /// as a miss.
    #[must_use]
    pub fn get(&self, source: &[u8], file_id: FileId) -> Option<CachedFile> {
        let source_hash = bytes_hash(source);
        let path = self.blob_path(&source_hash);
        if !blob_len_admissible(&path) {
            return None;
        }
        let bytes = fs::read(&path).ok()?;
        match decode(&bytes, &self.binding(&source_hash), file_id) {
            Ok(cached) => Some(cached),
            Err(error) => {
                tracing::warn!(
                    path = %path.display(),
                    %error,
                    "fingerprint cache entry rejected — treating as miss",
                );
                None
            }
        }
    }

    /// Writes `cached` to disk keyed by the hash of `source`, bound to
    /// this partition's address. Errors are non-fatal — the caller
    /// retries the full analysis path.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error when the blob cannot be
    /// written.
    pub fn store(&self, source: &[u8], cached: &CachedFile) -> io::Result<()> {
        let source_hash = bytes_hash(source);
        let encoded = encode(cached, &self.binding(&source_hash));
        if u64::try_from(encoded.len()).unwrap_or(u64::MAX) > MAX_BLOB_BYTES {
            tracing::warn!(
                len = encoded.len(),
                "fingerprint cache blob exceeds the size bound — not persisted",
            );
            return Ok(());
        }
        fs::write(self.blob_path(&source_hash), encoded)
    }

    /// Resolves the on-disk path for a source hash.
    ///
    /// Hashing bytes — never a decoded string — is load-bearing: a
    /// lossy decode collapses every maximal invalid UTF-8 subsequence
    /// to one U+FFFD, so byte-distinct files would share one entry and
    /// the second file read in a run would be served the first file's
    /// tree and fingerprints. Pinned by
    /// `lossy_utf8_cache_key_must_not_collide_across_distinct_files`
    /// in `crates/deslop/tests/cache_key_lossy_utf8_collision.rs`.
    fn blob_path(&self, source_hash: &str) -> PathBuf {
        self.root.join(blob_file_name(source_hash))
    }

    /// The full address a blob under `source_hash` must be bound to —
    /// every component of the documented store key
    /// `(language_id, tool_version, min_nodes, source_byte_hash)`.
    fn binding<'a>(&'a self, source_hash: &'a str) -> BlobBinding<'a> {
        BlobBinding {
            language_id: &self.language_id,
            tool_version: TOOL_VERSION,
            min_nodes: self.min_nodes,
            source_hash,
        }
    }
}

/// On-disk file name of the blob for `source_hash` — the single
/// definition of the `.bin` convention, shared by the lookup path and
/// retention ([PIPELINE-INCREMENTAL-RETENTION]).
fn blob_file_name(source_hash: &str) -> String {
    format!("{source_hash}.bin")
}
