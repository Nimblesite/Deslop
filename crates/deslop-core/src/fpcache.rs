//! On-disk fingerprint + normalised-tree cache ([PIPELINE-INCREMENTAL]).
//!
//! Keyed by `(language_id, tool_version, min_nodes, content_hash)`. A
//! cache hit rehydrates both the structural fingerprints and the
//! normalised AST (kept because downstream token extraction walks it),
//! skipping tree-sitter entirely for unchanged files. Any mismatch on
//! the cache key — or a blob whose tree nests past [`MAX_AST_DEPTH`] —
//! degrades gracefully to a miss, so a stale or corrupt blob cannot
//! corrupt or crash a run, at worst it wastes disk.
//!
//! The on-disk format is a single little-endian binary blob. Nothing
//! from `serde` — the shape is tight, versioned by a magic header, and
//! easily auditable byte-by-byte.

use std::{
    fs,
    io::{self, Cursor, Read},
    path::{Path, PathBuf},
};

use crate::{
    ast::{ByteRange, NormalizedNode},
    fingerprint::Fingerprint,
    lang::shared::{intern_kind, MAX_AST_DEPTH},
    state::FileId,
};

/// Magic number at the top of every cache blob. Any mismatch is
/// treated as a miss; lets us detect corruption and truncated
/// writes without a separate integrity check.
const MAGIC: u32 = 0xC0DE_D17E;

/// Tool version pinned into the cache key so a `deslop-core` bump
/// (grammar pins, normalisation rules, hashing changes) invalidates
/// every previously-cached blob. The blob itself carries only MAGIC
/// — the tool version is expressed via the cache directory path so
/// blobs from different versions can coexist without collision.
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
}

/// On-disk fingerprint cache scoped to a `(language, min_nodes)`
/// partition. One instance per language per run, opened lazily through
/// [`FingerprintCache::open`].
/// The `root: PathBuf` field — `<base>/fingerprints/<lang>/<ver>/<min>` —
/// was removed with the quarantine below: [`FingerprintCache::path_for`]
/// was its only reader, so retaining it would be dead code. `open` still
/// resolves and creates the directory so the layout contract is unchanged
/// when the key derivation is restored.
#[derive(Debug)]
pub struct FingerprintCache;

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
        Ok(Self)
    }

    /// Returns the cached pre-analysis for `source` under `file_id`,
    /// if present and loadable. Any decode failure is logged at
    /// `tracing::warn!` and treated as a miss.
    #[must_use]
    pub fn get(&self, source: &[u8], file_id: FileId) -> Option<CachedFile> {
        let path = self.path_for(source);
        let bytes = fs::read(&path).ok()?;
        match decode(&bytes, file_id) {
            Ok(cached) => Some(cached),
            Err(error) => {
                tracing::warn!(
                    path = %path.display(),
                    %error,
                    "fingerprint cache entry unreadable — treating as miss",
                );
                None
            }
        }
    }

    /// Writes `cached` to disk keyed by the hash of `source`. Errors
    /// are non-fatal — the caller retries the full analysis path.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error when the blob cannot be
    /// written.
    pub fn store(&self, source: &[u8], cached: &CachedFile) -> io::Result<()> {
        let path = self.path_for(source);
        let encoded = encode(cached);
        fs::write(path, encoded)
    }

    /// QUARANTINED — accuracy defect, do not restore ([PIPELINE-INCREMENTAL]).
    ///
    /// This method resolved the on-disk cache path for a source blob as
    /// `content_hash(&String::from_utf8_lossy(source))`, then
    /// `self.root.join(format!("{hash}.bin"))`.
    ///
    /// Hashing the *lossy* decode instead of the source bytes made the
    /// cache key non-injective. Every maximal invalid UTF-8 subsequence
    /// collapses to one U+FFFD before hashing, so files that differ in
    /// bytes — and in byte *length* — share a single cache entry. The
    /// second such file read in a run is served the first file's
    /// normalised tree and fingerprints, and every byte range in the
    /// report is shifted by the length difference: the run reports a
    /// clone at offsets that belong to a different file. Because the
    /// served fingerprints are the *other* file's, a collision between
    /// files that are not structurally identical is a false positive for
    /// code the second file does not contain and a false negative for the
    /// code it does.
    ///
    /// Pinned by `lossy_utf8_cache_key_must_not_collide_across_distinct_files`
    /// in `crates/deslop/tests/cache_key_lossy_utf8_collision.rs`, which
    /// seeds two files whose bytes differ by one and asserts the cached
    /// run's spans equal the `--no-incremental` spans.
    ///
    /// # Panics
    ///
    /// Always. The cache cannot be keyed correctly until the key is
    /// derived from `source` itself.
    #[allow(clippy::panic, clippy::unused_self)]
    fn path_for(&self, _source: &[u8]) -> PathBuf {
        panic!(
            "fingerprint cache key quarantined: lossy-UTF-8 hashing collided distinct files \
             and reported clone spans from the wrong file — see \
             crates/deslop/tests/cache_key_lossy_utf8_collision.rs"
        );
    }
}

/// Serialises `cached` into the little-endian blob format.
fn encode(cached: &CachedFile) -> Vec<u8> {
    let mut out = Vec::with_capacity(1024);
    out.extend_from_slice(&MAGIC.to_le_bytes());
    encode_tree(&cached.tree, &mut out);
    let fp_len = u64::try_from(cached.fingerprints.len()).unwrap_or(u64::MAX);
    out.extend_from_slice(&fp_len.to_le_bytes());
    for fp in &cached.fingerprints {
        encode_fingerprint(fp, &mut out);
    }
    out
}

/// Appends one normalised node (and its subtree) to `out`.
fn encode_tree(node: &NormalizedNode, out: &mut Vec<u8>) {
    let kind_bytes = node.kind.as_bytes();
    let kind_len = u32::try_from(kind_bytes.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&kind_len.to_le_bytes());
    out.extend_from_slice(kind_bytes);
    let start = u64::try_from(node.byte_range.start).unwrap_or(u64::MAX);
    let end = u64::try_from(node.byte_range.end).unwrap_or(u64::MAX);
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&end.to_le_bytes());
    let child_count = u32::try_from(node.children.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&child_count.to_le_bytes());
    for child in &node.children {
        encode_tree(child, out);
    }
}

/// Appends one [`Fingerprint`] record to `out`.
fn encode_fingerprint(fp: &Fingerprint, out: &mut Vec<u8>) {
    out.extend_from_slice(&fp.hash);
    let start = u64::try_from(fp.byte_range.start).unwrap_or(u64::MAX);
    let end = u64::try_from(fp.byte_range.end).unwrap_or(u64::MAX);
    let nodes = u64::try_from(fp.node_count).unwrap_or(u64::MAX);
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&end.to_le_bytes());
    out.extend_from_slice(&nodes.to_le_bytes());
}

/// Parses the blob at `bytes` into a [`CachedFile`], reassigning every
/// node and fingerprint to `file_id` (the registry handle issued for
/// *this* run).
fn decode(bytes: &[u8], file_id: FileId) -> io::Result<CachedFile> {
    let mut cursor = Cursor::new(bytes);
    let magic = read_u32(&mut cursor)?;
    if magic != MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "fingerprint cache magic mismatch",
        ));
    }
    let tree = decode_tree(&mut cursor, file_id, 1)?;
    let fp_count = u64_to_usize(read_u64(&mut cursor)?)?;
    let mut fingerprints = Vec::with_capacity(fp_count);
    for _ in 0..fp_count {
        fingerprints.push(decode_fingerprint(&mut cursor, file_id)?);
    }
    Ok(CachedFile { tree, fingerprints })
}

/// Reconstructs one [`NormalizedNode`] subtree and all of its
/// descendants from the cursor at nesting `depth`. Bounds recursion at
/// [`MAX_AST_DEPTH`] so a corrupt or pre-cap blob cannot overflow the
/// stack here — `decode_tree` is the only `NormalizedNode` producer
/// besides `normalise_node`, so the depth invariant must hold at both
///. Over-deep blobs fail decode and are treated as a cache miss,
/// which re-parses and re-rejects through the normaliser.
fn decode_tree(
    cursor: &mut Cursor<&[u8]>,
    file_id: FileId,
    depth: usize,
) -> io::Result<NormalizedNode> {
    if depth > MAX_AST_DEPTH {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "cached AST nests deeper than the depth limit",
        ));
    }
    let header = decode_node_header(cursor)?;
    let mut children = Vec::with_capacity(header.child_count);
    for _ in 0..header.child_count {
        children.push(decode_tree(&mut *cursor, file_id, depth.saturating_add(1))?);
    }
    Ok(NormalizedNode {
        kind: header.kind,
        children,
        byte_range: ByteRange {
            start: header.start,
            end: header.end,
        },
        file_id,
    })
}

/// One node's decoded header: interned kind, byte range, and child count,
/// read in the encoder's order. Split out of [`decode_tree`] so the
/// recursive walk stays small after the depth guard was added.
struct NodeHeader {
    /// Interned normalised node kind.
    kind: &'static str,
    /// Inclusive start byte offset into the source file.
    start: usize,
    /// Exclusive end byte offset into the source file.
    end: usize,
    /// Number of direct children that follow in the blob.
    child_count: usize,
}

/// Reads one node's kind / byte-range / child-count prefix from `cursor`.
fn decode_node_header(cursor: &mut Cursor<&[u8]>) -> io::Result<NodeHeader> {
    let kind_len = u32_to_usize(read_u32(&mut *cursor)?);
    let mut kind_bytes = vec![0_u8; kind_len];
    cursor.read_exact(&mut kind_bytes)?;
    let kind_str = std::str::from_utf8(&kind_bytes)
        .map_err(|source| io::Error::new(io::ErrorKind::InvalidData, source))?;
    let kind = intern_kind(kind_str);
    let start = u64_to_usize(read_u64(&mut *cursor)?)?;
    let end = u64_to_usize(read_u64(&mut *cursor)?)?;
    let child_count = u32_to_usize(read_u32(&mut *cursor)?);
    Ok(NodeHeader {
        kind,
        start,
        end,
        child_count,
    })
}

/// Reads one [`Fingerprint`] record from `cursor`, rebinding it to
/// `file_id`.
fn decode_fingerprint(cursor: &mut Cursor<&[u8]>, file_id: FileId) -> io::Result<Fingerprint> {
    let mut hash = [0_u8; 32];
    cursor.read_exact(&mut hash)?;
    let start = u64_to_usize(read_u64(&mut *cursor)?)?;
    let end = u64_to_usize(read_u64(&mut *cursor)?)?;
    let node_count = u64_to_usize(read_u64(&mut *cursor)?)?;
    Ok(Fingerprint {
        hash,
        file_id,
        byte_range: ByteRange { start, end },
        node_count,
    })
}

/// Converts a `u64` read from the cache blob into a `usize`, wrapping
/// a single out-of-range error variant so the decoder is legible.
/// Only fires on 32-bit targets with absurdly large blobs — but the
/// `Result` stays so those targets still fail cleanly rather than
/// silently truncating.
fn u64_to_usize(value: u64) -> io::Result<usize> {
    usize::try_from(value).map_err(|source| io::Error::new(io::ErrorKind::InvalidData, source))
}

/// `u32 → usize` always fits — `usize` is at least 32 bits on every
/// platform Rust supports — so this is a pure-widening helper.
fn u32_to_usize(value: u32) -> usize {
    value as usize
}

/// Reads a little-endian `u32` out of the cursor.
fn read_u32(cursor: &mut Cursor<&[u8]>) -> io::Result<u32> {
    let mut buf = [0_u8; 4];
    cursor.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

/// Reads a little-endian `u64` out of the cursor.
fn read_u64(cursor: &mut Cursor<&[u8]>) -> io::Result<u64> {
    let mut buf = [0_u8; 8];
    cursor.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}
