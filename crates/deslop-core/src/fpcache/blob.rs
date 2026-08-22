//! The parse-store wire format ([PIPELINE-INCREMENTAL-INTEGRITY]): a
//! single little-endian binary blob — magic, binding digest, payload —
//! with a bounded, self-healing decode.
//!
//! Nothing from `serde`: the shape is tight, versioned by [`MAGIC`],
//! and easily auditable byte-by-byte. Every rejection in this module is
//! `InvalidData`, which the cache above treats as a plain miss — a
//! corrupt or misaddressed blob can cost a re-parse, never an incorrect
//! hit and never a crash.

use std::io::{self, Cursor, Read};

use crate::{
    ast::{ByteRange, NormalizedNode},
    fingerprint::Fingerprint,
    lang::shared::{intern_kind, MAX_AST_DEPTH},
    lsh::{Signature, SIGNATURE_LEN},
    state::FileId,
};

use super::CachedFile;

mod bounds;

use bounds::NodeBudget;
pub(super) use bounds::{read_bounded, MAX_BLOB_BYTES};

/// Magic number at the top of every cache blob, bumped whenever the
/// blob layout changes — `0xC0DE_D17E` was the pre-signature layout and
/// `0xC0DE_D17F` the pre-binding-digest one, so a blob written by
/// either decodes as a magic mismatch (a plain miss) and is rewritten
/// in the current format by the store that follows.
pub(super) const MAGIC: u32 = 0xC0DE_D180;

/// Semantic revision of the values inside a blob, folded into the
/// binding digest. Bumped whenever parsing, normalisation,
/// fingerprinting, or signature construction changes *meaning* without
/// changing the blob layout. Deliberately independent of
/// [`super::TOOL_VERSION`]: the workspace version is a permanently-
/// reused development string (`0.0.0-dev`), so the directory partition
/// alone cannot invalidate blobs across a semantic change
/// ([PIPELINE-INCREMENTAL-INTEGRITY]). A release build is stamped with
/// its own version and therefore partitioned on its own, so a forgotten
/// bump can only ever mislead a development store.
///
/// **This is the one invalidation lever no equivalence test can pull on
/// its own**, because a stale blob makes the warm *and* cold sides of a
/// comparison stale together. What catches a forgotten bump is the set
/// of goldens that pin the pre-change analysis and go red on any change
/// to its meaning — the per-language `Sample.expected.ast` dumps
/// ([PIPELINE-NORMALIZE-AST], `deslop/tests/cli/cache_and_debug.rs`) for
/// parsing and normalisation, and the two committed report goldens
/// (`report_golden.rs`, `incremental_multilang_golden.rs`) for
/// fingerprinting and signature construction. Each names this constant
/// in its failure message, so the change that must bump it is also the
/// change that is told to.
/// Epoch 2: [PIPELINE-NORMALIZE-AST-OPERATOR] keeps behaviour-bearing
/// anonymous tokens as operator leaves, so the normalised tree of an
/// unchanged file changed meaning.
///
/// Epoch 3: the same section stopped collapsing those leaves to one
/// `__op__` kind and gave each one its own token (`__op__+`). Epoch 2
/// alone is not enough to cover it — a store warmed under epoch 2 holds
/// trees in which `base + fee` and `base - fee` still hash identically,
/// so a warm run would keep certifying an operator swap as duplication
/// long after the normalisation that produced it was replaced.
pub(super) const SEMANTIC_EPOCH: u32 = 3;

/// Bytes of blob header preceding the payload: the magic plus the
/// 32-byte binding digest.
const HEADER_LEN: usize = 4 + 32;

/// Exact encoded bytes of one fingerprint record: 32-byte hash plus
/// start, end, and node count as `u64`s.
const FINGERPRINT_RECORD_LEN: usize = 32 + 8 + 8 + 8;

/// Exact encoded bytes of one `MinHash` signature.
const SIGNATURE_RECORD_LEN: usize = SIGNATURE_LEN * 8;

/// Minimum encoded bytes of one tree node: kind length, byte range,
/// and child count, with an empty kind string.
const MIN_NODE_LEN: usize = 4 + 8 + 8 + 4;

/// Everything a blob must be provably bound to before it may be served
/// ([PIPELINE-INCREMENTAL-INTEGRITY]): the address that selected it.
/// [`binding_digest`] folds in the layout revision ([`MAGIC`]), the
/// semantic revision ([`SEMANTIC_EPOCH`]) and the signature width
/// ([`SIGNATURE_LEN`]) alongside these fields.
pub(super) struct BlobBinding<'a> {
    /// Parser language id of the partition the lookup runs in.
    pub(super) language_id: &'a str,
    /// Tool version of the partition the lookup runs in. Part of the
    /// documented store key, so it is part of the digest: a blob
    /// *relocated* between version directories (a copied, restored, or
    /// merged store) would otherwise verify cleanly under a version
    /// that never wrote it.
    pub(super) tool_version: &'a str,
    /// Subtree-size floor of the partition the lookup runs in.
    pub(super) min_nodes: u32,
    /// BLAKE3 hex digest of the raw source bytes being looked up.
    pub(super) source_hash: &'a str,
}

/// BLAKE3 digest binding `payload` to `binding` and to the current
/// layout, semantic, and signature-width revisions. Every variable-width
/// field is length-prefixed so the input stays injective — without it a
/// `("rust", "1.2")` address and a `("rust1", ".2")` one would hash
/// identically.
pub(super) fn binding_digest(binding: &BlobBinding<'_>, payload: &[u8]) -> [u8; 32] {
    let signature_width = u64::try_from(SIGNATURE_LEN).unwrap_or(u64::MAX);
    let mut hasher = blake3::Hasher::new();
    let _ = hasher.update(&MAGIC.to_le_bytes());
    let _ = hasher.update(&SEMANTIC_EPOCH.to_le_bytes());
    let _ = hasher.update(&signature_width.to_le_bytes());
    let _ = hasher.update(&binding.min_nodes.to_le_bytes());
    for field in [
        binding.language_id,
        binding.tool_version,
        binding.source_hash,
    ] {
        let _ = hasher.update(&u64::try_from(field.len()).unwrap_or(u64::MAX).to_le_bytes());
        let _ = hasher.update(field.as_bytes());
    }
    let _ = hasher.update(payload);
    *hasher.finalize().as_bytes()
}

/// Serialises `cached` into the little-endian blob format, bound to
/// `binding`: magic, binding digest, payload.
pub(super) fn encode(cached: &CachedFile, binding: &BlobBinding<'_>) -> Vec<u8> {
    let payload = encode_payload(cached);
    let digest = binding_digest(binding, &payload);
    let mut out = Vec::with_capacity(HEADER_LEN.saturating_add(payload.len()));
    out.extend_from_slice(&MAGIC.to_le_bytes());
    out.extend_from_slice(&digest);
    out.extend_from_slice(&payload);
    out
}

/// Serialises the payload — tree, fingerprints, signatures — into one
/// exactly-sized allocation.
fn encode_payload(cached: &CachedFile) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload_capacity(cached));
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

/// Exact payload byte length, so [`encode_payload`] allocates once.
fn payload_capacity(cached: &CachedFile) -> usize {
    let fingerprint_bytes = cached
        .fingerprints
        .len()
        .saturating_mul(FINGERPRINT_RECORD_LEN);
    let signature_bytes = cached.signatures.len().saturating_mul(SIGNATURE_RECORD_LEN);
    encoded_tree_len(&cached.tree)
        .saturating_add(8)
        .saturating_add(fingerprint_bytes)
        .saturating_add(8)
        .saturating_add(signature_bytes)
}

/// Exact encoded byte length of one node and its subtree.
fn encoded_tree_len(node: &NormalizedNode) -> usize {
    node.children.iter().map(encoded_tree_len).fold(
        MIN_NODE_LEN.saturating_add(node.kind.len()),
        usize::saturating_add,
    )
}

/// Appends one `MinHash` signature — [`SIGNATURE_LEN`] little-endian
/// `u64` slots — to `out`.
fn encode_signature(signature: &Signature, out: &mut Vec<u8>) {
    for slot in signature {
        out.extend_from_slice(&slot.to_le_bytes());
    }
}

/// Appends one normalised node (and its subtree) to `out`.
pub(super) fn encode_tree(node: &NormalizedNode, out: &mut Vec<u8>) {
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
/// *this* run). Serves nothing until the stored binding digest is
/// reproduced from `binding` and the payload bytes
/// ([PIPELINE-INCREMENTAL-INTEGRITY]).
pub(super) fn decode(
    bytes: &[u8],
    binding: &BlobBinding<'_>,
    file_id: FileId,
) -> io::Result<CachedFile> {
    let mut cursor = Cursor::new(bytes);
    if read_u32(&mut cursor)? != MAGIC {
        return Err(invalid_data("fingerprint cache magic mismatch"));
    }
    let mut stored_digest = [0_u8; 32];
    cursor.read_exact(&mut stored_digest)?;
    let payload = bytes.get(HEADER_LEN..).unwrap_or_default();
    if binding_digest(binding, payload) != stored_digest {
        return Err(invalid_data(
            "binding digest mismatch — blob does not belong to this address",
        ));
    }
    decode_payload(&mut cursor, file_id)
}

/// Decodes the digest-verified payload section, requiring it to consume
/// the blob exactly.
fn decode_payload(cursor: &mut Cursor<&[u8]>, file_id: FileId) -> io::Result<CachedFile> {
    let tree = decode_tree(&mut *cursor, file_id, 1, &mut NodeBudget::new())?;
    let (fingerprints, signatures) = decode_records(cursor, file_id)?;
    ensure_fully_consumed(cursor)?;
    Ok(CachedFile {
        tree,
        fingerprints,
        signatures,
    })
}

/// Decodes the fingerprint and signature sections, enforcing the
/// positional 1:1 count invariant the reuse contract rests on
/// ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]).
fn decode_records(
    cursor: &mut Cursor<&[u8]>,
    file_id: FileId,
) -> io::Result<(Vec<Fingerprint>, Vec<Signature>)> {
    let fp_count = read_record_count(&mut *cursor, FINGERPRINT_RECORD_LEN)?;
    let mut fingerprints = Vec::with_capacity(fp_count);
    for _ in 0..fp_count {
        fingerprints.push(decode_fingerprint(&mut *cursor, file_id)?);
    }
    let signature_count = read_record_count(&mut *cursor, SIGNATURE_RECORD_LEN)?;
    if signature_count != fp_count {
        return Err(invalid_data(
            "cached signature count disagrees with fingerprint count",
        ));
    }
    let mut signatures = Vec::with_capacity(signature_count);
    for _ in 0..signature_count {
        signatures.push(decode_signature(&mut *cursor)?);
    }
    Ok((fingerprints, signatures))
}

/// Rejects a blob whose payload decodes short of its final byte. The
/// digest already covers every byte, so trailing data normally fails
/// verification first; this guard catches the remaining case — an
/// encoder/decoder length disagreement — instead of silently accepting
/// it.
fn ensure_fully_consumed(cursor: &Cursor<&[u8]>) -> io::Result<()> {
    if remaining_bytes(cursor) != 0 {
        return Err(invalid_data(
            "cache blob carries trailing bytes past the decoded payload",
        ));
    }
    Ok(())
}

/// Reads a record count and proves `count` records of at least
/// `min_record_len` bytes fit in the bytes remaining, so a corrupt
/// count becomes `InvalidData` (a miss) before it can size any
/// allocation ([PIPELINE-INCREMENTAL-INTEGRITY]).
fn read_record_count(cursor: &mut Cursor<&[u8]>, min_record_len: usize) -> io::Result<usize> {
    let count = u64_to_usize(read_u64(&mut *cursor)?)?;
    ensure_remaining(cursor, count, min_record_len)?;
    Ok(count)
}

/// Errors unless `count * min_record_len` bytes remain past the cursor.
fn ensure_remaining(cursor: &Cursor<&[u8]>, count: usize, min_record_len: usize) -> io::Result<()> {
    let needed = count
        .checked_mul(min_record_len)
        .ok_or_else(|| invalid_data("cache record count overflows"))?;
    if needed > remaining_bytes(cursor) {
        return Err(invalid_data(
            "cache record count exceeds the bytes remaining in the blob",
        ));
    }
    Ok(())
}

/// Bytes between the cursor position and the end of the blob.
fn remaining_bytes(cursor: &Cursor<&[u8]>) -> usize {
    let position = usize::try_from(cursor.position()).unwrap_or(usize::MAX);
    cursor.get_ref().len().saturating_sub(position)
}

/// Shorthand for the `InvalidData` errors every rejection path uses —
/// each one is a miss, never a crash.
fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Reads one `MinHash` signature — [`SIGNATURE_LEN`] little-endian
/// `u64` slots — from `cursor`.
fn decode_signature(cursor: &mut Cursor<&[u8]>) -> io::Result<Signature> {
    let bytes = read_slice(cursor, SIGNATURE_RECORD_LEN)?;
    let mut signature: Signature = [0_u64; SIGNATURE_LEN];
    for (slot, encoded) in signature.iter_mut().zip(bytes.chunks_exact(8)) {
        let encoded = encoded
            .try_into()
            .map_err(|_| invalid_data("cached signature slot has the wrong width"))?;
        *slot = u64::from_le_bytes(encoded);
    }
    Ok(signature)
}

/// Reconstructs one [`NormalizedNode`] subtree and all of its
/// descendants from the cursor at nesting `depth`. Bounds recursion at
/// [`MAX_AST_DEPTH`] so a corrupt or pre-cap blob cannot overflow the
/// stack here — `decode_tree` is the only `NormalizedNode` producer
/// besides `normalise_node`, so the depth invariant must hold at both.
/// Over-deep blobs fail decode and are treated as a cache miss,
/// which re-parses and re-rejects through the normaliser.
fn decode_tree(
    cursor: &mut Cursor<&[u8]>,
    file_id: FileId,
    depth: usize,
    budget: &mut NodeBudget,
) -> io::Result<NormalizedNode> {
    if depth > MAX_AST_DEPTH {
        return Err(invalid_data("cached AST nests deeper than the depth limit"));
    }
    let header = decode_node_header(cursor)?;
    budget.claim(header.child_count)?;
    let mut children = Vec::new();
    children
        .try_reserve_exact(header.child_count)
        .map_err(|_| invalid_data("cached AST child list exceeds available memory"))?;
    for _ in 0..header.child_count {
        children.push(decode_tree(
            &mut *cursor,
            file_id,
            depth.saturating_add(1),
            budget,
        )?);
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

/// Reads one node's kind / byte-range / child-count prefix from
/// `cursor`, proving each length against the bytes remaining before
/// any allocation.
fn decode_node_header(cursor: &mut Cursor<&[u8]>) -> io::Result<NodeHeader> {
    let kind_len = u32_to_usize(read_u32(&mut *cursor)?);
    ensure_remaining(cursor, kind_len, 1)?;
    let kind_bytes = read_slice(cursor, kind_len)?;
    let kind_str = std::str::from_utf8(kind_bytes)
        .map_err(|source| io::Error::new(io::ErrorKind::InvalidData, source))?;
    let kind = intern_kind(kind_str);
    let start = u64_to_usize(read_u64(&mut *cursor)?)?;
    let end = u64_to_usize(read_u64(&mut *cursor)?)?;
    let child_count = u32_to_usize(read_u32(&mut *cursor)?);
    ensure_remaining(cursor, child_count, MIN_NODE_LEN)?;
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

/// Reads exactly `N` bytes out of the cursor.
fn read_array<const N: usize>(cursor: &mut Cursor<&[u8]>) -> io::Result<[u8; N]> {
    read_slice(cursor, N)?
        .try_into()
        .map_err(|_| invalid_data("cache record has the wrong fixed width"))
}

/// Borrows exactly `len` bytes from the cursor and advances it once.
fn read_slice<'a>(cursor: &mut Cursor<&'a [u8]>, len: usize) -> io::Result<&'a [u8]> {
    let start = usize::try_from(cursor.position())
        .map_err(|_| invalid_data("cache cursor position exceeds this platform"))?;
    let end = start
        .checked_add(len)
        .ok_or_else(|| invalid_data("cache record length overflows"))?;
    let bytes = (*cursor.get_ref())
        .get(start..end)
        .ok_or_else(|| invalid_data("cache record exceeds the bytes remaining"))?;
    let position = u64::try_from(end)
        .map_err(|_| invalid_data("cache cursor position exceeds the blob format"))?;
    cursor.set_position(position);
    Ok(bytes)
}

/// Reads a little-endian `u32` out of the cursor.
fn read_u32(cursor: &mut Cursor<&[u8]>) -> io::Result<u32> {
    Ok(u32::from_le_bytes(read_array(cursor)?))
}

/// Reads a little-endian `u64` out of the cursor.
fn read_u64(cursor: &mut Cursor<&[u8]>) -> io::Result<u64> {
    Ok(u64::from_le_bytes(read_array(cursor)?))
}
