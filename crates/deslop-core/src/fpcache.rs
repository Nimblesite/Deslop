//! On-disk fingerprint + normalised-tree cache ([PIPELINE-INCREMENTAL]).
//!
//! Keyed by `(language_id, tool_version, min_nodes, source_byte_hash)`. A
//! cache hit rehydrates the structural fingerprints, the normalised
//! AST (kept because downstream token extraction walks it), and the
//! per-fingerprint `MinHash` signatures
//! ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]), skipping tree-sitter and
//! signature construction entirely for unchanged files. Any mismatch on
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
    embedding::bytes_hash,
    fingerprint::Fingerprint,
    lang::shared::{intern_kind, MAX_AST_DEPTH},
    lsh::{Signature, SIGNATURE_LEN},
    state::FileId,
};

/// Magic number at the top of every cache blob, bumped whenever the
/// blob layout changes — `0xC0DE_D17E` was the pre-signature layout,
/// so a blob written before signatures were persisted decodes as a
/// magic mismatch (a plain miss) and is rewritten in the current
/// format by the store that follows. Any mismatch is treated as a
/// miss; lets us detect corruption and truncated writes without a
/// separate integrity check.
const MAGIC: u32 = 0xC0DE_D17F;

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
    /// One `MinHash` signature per entry of `fingerprints`,
    /// positionally 1:1 ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]).
    /// Persisted so a warm pass attaches them instead of rebuilding
    /// every signature from token streams — the dominant cost of the
    /// LSH stage. The decode enforces the count invariant.
    pub signatures: Vec<Signature>,
}

/// On-disk fingerprint cache scoped to a `(language, min_nodes)`
/// partition. One instance per language per run, opened lazily through
/// [`FingerprintCache::open`].
#[derive(Debug)]
pub struct FingerprintCache {
    /// Fully-qualified cache directory for the current partition —
    /// `<base>/fingerprints/<lang>/<ver>/<min>`.
    root: PathBuf,
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
        Ok(Self { root })
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

    /// Resolves the on-disk path for a given source blob, keyed by the
    /// BLAKE3 digest of the raw `source` bytes.
    ///
    /// Hashing bytes — never a decoded string — is load-bearing: a
    /// lossy decode collapses every maximal invalid UTF-8 subsequence
    /// to one U+FFFD, so byte-distinct files would share one entry and
    /// the second file read in a run would be served the first file's
    /// tree and fingerprints. Pinned by
    /// `lossy_utf8_cache_key_must_not_collide_across_distinct_files`
    /// in `crates/deslop/tests/cache_key_lossy_utf8_collision.rs`.
    fn path_for(&self, source: &[u8]) -> PathBuf {
        let hash = bytes_hash(source);
        self.root.join(format!("{hash}.bin"))
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
    let signature_len = u64::try_from(cached.signatures.len()).unwrap_or(u64::MAX);
    out.extend_from_slice(&signature_len.to_le_bytes());
    for signature in &cached.signatures {
        encode_signature(signature, &mut out);
    }
    out
}

/// Appends one `MinHash` signature — [`SIGNATURE_LEN`] little-endian
/// `u64` slots — to `out`.
fn encode_signature(signature: &Signature, out: &mut Vec<u8>) {
    for slot in signature {
        out.extend_from_slice(&slot.to_le_bytes());
    }
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
    let signature_count = u64_to_usize(read_u64(&mut cursor)?)?;
    if signature_count != fp_count {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "cached signature count disagrees with fingerprint count",
        ));
    }
    let mut signatures = Vec::with_capacity(signature_count);
    for _ in 0..signature_count {
        signatures.push(decode_signature(&mut cursor)?);
    }
    Ok(CachedFile {
        tree,
        fingerprints,
        signatures,
    })
}

/// Reads one `MinHash` signature — [`SIGNATURE_LEN`] little-endian
/// `u64` slots — from `cursor`.
fn decode_signature(cursor: &mut Cursor<&[u8]>) -> io::Result<Signature> {
    let mut signature: Signature = [0_u64; SIGNATURE_LEN];
    for slot in &mut signature {
        *slot = read_u64(&mut *cursor)?;
    }
    Ok(signature)
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::state::FileRegistry;

    /// A two-node tree, two fingerprints, and two distinct signatures —
    /// the smallest bundle exercising every record type in the blob.
    fn sample(file_id: FileId) -> CachedFile {
        let leaf = NormalizedNode {
            kind: "identifier",
            children: Vec::new(),
            byte_range: ByteRange { start: 3, end: 9 },
            file_id,
        };
        let tree = NormalizedNode {
            kind: "function_item",
            children: vec![leaf],
            byte_range: ByteRange { start: 0, end: 12 },
            file_id,
        };
        let fingerprints = vec![
            Fingerprint {
                hash: [7_u8; 32],
                file_id,
                byte_range: ByteRange { start: 0, end: 12 },
                node_count: 2,
            },
            Fingerprint {
                hash: [9_u8; 32],
                file_id,
                byte_range: ByteRange { start: 3, end: 9 },
                node_count: 1,
            },
        ];
        let signatures = vec![[1_u64; SIGNATURE_LEN], [2_u64; SIGNATURE_LEN]];
        CachedFile {
            tree,
            fingerprints,
            signatures,
        }
    }

    /// A [`FileId`] issued by a throwaway registry.
    fn registered_file_id() -> FileId {
        FileRegistry::new().register(PathBuf::from("blob_fixture.rs"))
    }

    // [PIPELINE-INCREMENTAL-ANALYSIS-REUSE] The blob must give back
    // exactly the tree, fingerprints, and signatures it was handed —
    // signatures positionally 1:1 with fingerprints.
    #[test]
    fn round_trip_preserves_tree_fingerprints_and_signatures() -> io::Result<()> {
        let file_id = registered_file_id();
        let original = sample(file_id);
        let decoded = decode(&encode(&original), file_id)?;
        assert_eq!(
            decoded.fingerprints, original.fingerprints,
            "decoded fingerprints must match the encoded records exactly"
        );
        assert_eq!(
            decoded.signatures, original.signatures,
            "decoded signatures must match the encoded slots exactly, in order"
        );
        assert_eq!(
            decoded.tree.kind, "function_item",
            "decoded tree root must keep its normalised kind"
        );
        assert_eq!(
            decoded.tree.byte_range,
            ByteRange { start: 0, end: 12 },
            "decoded tree root must keep its byte range"
        );
        assert_eq!(
            decoded.tree.children.len(),
            1,
            "decoded tree must keep its child structure"
        );
        assert_eq!(
            decoded.tree.children.first().map(|child| child.kind),
            Some("identifier"),
            "decoded child must keep its normalised kind"
        );
        Ok(())
    }

    // [PIPELINE-INCREMENTAL-ANALYSIS-REUSE] A signature list that does
    // not pair 1:1 with the fingerprint list can never be served — the
    // positional binding is the whole reuse contract.
    #[test]
    fn signature_count_mismatch_is_rejected_as_invalid_data() {
        let file_id = registered_file_id();
        let mut cached = sample(file_id);
        let _dropped = cached.signatures.pop();
        assert_eq!(
            decode(&encode(&cached), file_id)
                .err()
                .map(|error| error.kind()),
            Some(io::ErrorKind::InvalidData),
            "a blob whose signature count disagrees with its fingerprint count must fail decode"
        );
    }

    // A blob written by the pre-signature layout carries the old magic
    // and must decode as a miss-grade error, never as a signature-less
    // hit.
    #[test]
    fn pre_signature_magic_is_rejected() {
        let file_id = registered_file_id();
        let encoded = encode(&sample(file_id));
        let mut blob = 0xC0DE_D17E_u32.to_le_bytes().to_vec();
        blob.extend_from_slice(encoded.get(4..).unwrap_or_default());
        assert!(
            blob.len() == encoded.len(),
            "the re-stamped blob must differ from the original only in its magic"
        );
        assert_eq!(
            decode(&blob, file_id).err().map(|error| error.kind()),
            Some(io::ErrorKind::InvalidData),
            "the pre-signature magic must be rejected as invalid data"
        );
    }
}
