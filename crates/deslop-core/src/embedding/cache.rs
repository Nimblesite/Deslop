//! On-disk embedding cache keyed by
//! `(content_hash, provider_id, model_id, model_version)`.
//!
//! Implements the caching rule from [FUSION-EMBED-PROVIDER]: re-runs
//! with unchanged content / provider / model / version skip inference
//! entirely; swapping models invalidates only the embedding layer.
//! The cache is a simple sharded directory of little-endian `f32`
//! binary blobs — zero external dependencies and trivially auditable.

use std::{
    fs,
    path::{Path, PathBuf},
};

use blake3::Hasher;

use crate::embedding::provider::EmbeddingSpec;

/// Cache layout:
///
/// ```text
/// <root>/
///   embeddings/
///     <provider_id>/
///       <model_id>/
///         <model_version>/
///           <content_hash>.bin
/// ```
#[derive(Debug)]
pub struct EmbeddingCache {
    /// Fully-qualified cache directory for the current
    /// `(provider, model, version)` triple.
    root: PathBuf,
    /// Cached dimensionality reported by the provider; used to
    /// validate that a byte-blob on disk has the expected length.
    dimensions: usize,
}

impl EmbeddingCache {
    /// Opens (or creates) the cache directory for `spec` under
    /// `base`. `base` is [`crate::paths::cache_dir`] of the scan root.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error if the cache directory cannot
    /// be created.
    pub fn open(base: &Path, spec: &EmbeddingSpec) -> std::io::Result<Self> {
        let root = base
            .join("embeddings")
            .join(sanitise(&spec.provider_id))
            .join(sanitise(&spec.model_id))
            .join(sanitise(&spec.model_version));
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            dimensions: spec.dimensions,
        })
    }

    /// Returns the cached embedding for `content`, if present and of
    /// the expected dimensionality. Corrupt or wrong-size files are
    /// treated as misses and fall through to [`EmbeddingCache::store`].
    #[must_use]
    pub fn get(&self, content: &str) -> Option<Vec<f32>> {
        let path = self.path_for(content);
        let bytes = fs::read(path).ok()?;
        decode(&bytes, self.dimensions)
    }

    /// Writes `embedding` to disk under the hash of `content`. The
    /// caller guarantees `embedding.len() == self.dimensions`; the
    /// pipeline wires that invariant through
    /// [`crate::pipeline::compute_embeddings`] and the provider's
    /// [`EmbeddingProvider::embed`] contract.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error if the file cannot be written.
    pub fn store(&self, content: &str, embedding: &[f32]) -> std::io::Result<()> {
        let path = self.path_for(content);
        let encoded = encode(embedding);
        fs::write(path, encoded)
    }

    /// Returns the on-disk path for a given content blob.
    fn path_for(&self, content: &str) -> PathBuf {
        let hash = content_hash(content);
        self.root.join(format!("{hash}.bin"))
    }
}

/// Stable BLAKE3 digest of `content` rendered as lowercase hex.
#[must_use]
pub fn content_hash(content: &str) -> String {
    let mut hasher = Hasher::new();
    let _ = hasher.update(content.as_bytes());
    let digest = hasher.finalize();
    hex(digest.as_bytes())
}

/// Encodes `embedding` as little-endian `f32` bytes.
fn encode(embedding: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(embedding.len().saturating_mul(4));
    for value in embedding {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

/// Decodes a little-endian `f32` buffer back into a vector. Returns
/// `None` when the length does not match `dimensions`.
fn decode(bytes: &[u8], dimensions: usize) -> Option<Vec<f32>> {
    let expected = dimensions.checked_mul(4)?;
    if bytes.len() != expected {
        return None;
    }
    let mut out = Vec::with_capacity(dimensions);
    for index in 0..dimensions {
        let start = index.checked_mul(4)?;
        let end = start.checked_add(4)?;
        let slice = bytes.get(start..end)?;
        let array: [u8; 4] = slice.try_into().ok()?;
        out.push(f32::from_le_bytes(array));
    }
    Some(out)
}

/// Lowercase hex encoding without external deps.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        out.push(nibble((*byte >> 4) & 0x0F));
        out.push(nibble(*byte & 0x0F));
    }
    out
}

/// Maps a 0..=15 nibble to its lowercase hex character.
const fn nibble(value: u8) -> char {
    match value {
        0 => '0',
        1 => '1',
        2 => '2',
        3 => '3',
        4 => '4',
        5 => '5',
        6 => '6',
        7 => '7',
        8 => '8',
        9 => '9',
        10 => 'a',
        11 => 'b',
        12 => 'c',
        13 => 'd',
        14 => 'e',
        _ => 'f',
    }
}

/// Sanitises a path segment so a maliciously-named model cannot
/// escape the cache directory. Allowed characters (alphanumerics,
/// `.`, `-`, `_`) pass through; everything else, including an empty
/// input, collapses to `_` so the resulting directory name is always
/// non-empty.
fn sanitise(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len().max(1));
    for ch in segment.chars() {
        let safe = ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_';
        out.push(if safe { ch } else { '_' });
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}
