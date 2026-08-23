//! Unit pins for the blob format and its binding digest
//! ([PIPELINE-INCREMENTAL-INTEGRITY],
//! [PIPELINE-INCREMENTAL-ANALYSIS-REUSE]). Every rejection here is the
//! defect class the incremental persistence audit reproduced end to
//! end: corrupted payloads, misaddressed blobs, trailing bytes, and
//! length fields that drove allocations. The E2E half lives in
//! `crates/deslop/tests/cache_blob_integrity.rs`.

use std::path::PathBuf;

use super::{
    blob::{
        binding_digest, decode, encode, encode_tree, BlobBinding, MAGIC, MAX_BLOB_BYTES,
        SEMANTIC_EPOCH,
    },
    *,
};
use crate::{ast::ByteRange, lsh::SIGNATURE_LEN, state::FileRegistry};

/// Source bytes every binding in these tests addresses.
const SOURCE: &[u8] = b"pub fn twice(value: i32) -> i32 { value + value }\n";

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
    let signatures = vec![
        std::array::from_fn(|index| u64::try_from(index).unwrap_or(u64::MAX)),
        std::array::from_fn(|index| {
            u64::try_from(index)
                .unwrap_or(u64::MAX)
                .saturating_add(10_000)
        }),
    ];
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

/// The binding the [`SOURCE`] fixture is stored under.
fn source_binding(source_hash: &str) -> BlobBinding<'_> {
    BlobBinding {
        language_id: "rust",
        tool_version: TOOL_VERSION,
        min_nodes: 8,
        source_hash,
    }
}

/// Encodes the sample bundle under the canonical [`SOURCE`] binding.
fn encoded_sample(file_id: FileId) -> Vec<u8> {
    let hash = bytes_hash(SOURCE);
    encode(&sample(file_id), &source_binding(&hash))
}

/// Asserts `decode` rejects `blob` under `binding` as `InvalidData`.
fn assert_rejected(blob: &[u8], binding: &BlobBinding<'_>, file_id: FileId, label: &str) {
    assert_eq!(
        decode(blob, binding, file_id)
            .err()
            .map(|error| error.kind()),
        Some(io::ErrorKind::InvalidData),
        "{label}: the blob must be rejected as invalid data — a miss, never a hit"
    );
}

// [PIPELINE-INCREMENTAL-ANALYSIS-REUSE] The blob must give back
// exactly the tree, fingerprints, and signatures it was handed —
// signatures positionally 1:1 with fingerprints.
#[test]
fn round_trip_preserves_tree_fingerprints_and_signatures() -> io::Result<()> {
    let mut registry = FileRegistry::new();
    let stored_file_id = registry.register(PathBuf::from("stored.rs"));
    let requested_file_id = registry.register(PathBuf::from("requested.rs"));
    let hash = bytes_hash(SOURCE);
    let binding = source_binding(&hash);
    let original = sample(stored_file_id);
    let expected = sample(requested_file_id);
    let decoded = decode(&encode(&original, &binding), &binding, requested_file_id)?;
    assert_eq!(
        decoded.fingerprints, expected.fingerprints,
        "decoded fingerprints must preserve their records and bind to the requested file"
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
        decoded.tree.file_id, requested_file_id,
        "decoded tree nodes must bind to the requested file"
    );
    assert_eq!(
        decoded.tree.children.len(),
        1,
        "decoded tree must keep its child structure"
    );
    assert_eq!(
        decoded
            .tree
            .children
            .first()
            .map(|child| (child.kind, child.file_id)),
        Some(("identifier", requested_file_id)),
        "decoded children must preserve their kind and bind to the requested file"
    );
    Ok(())
}

// [PIPELINE-INCREMENTAL-ANALYSIS-REUSE] A signature list that does
// not pair 1:1 with the fingerprint list can never be served — the
// positional binding is the whole reuse contract.
#[test]
fn signature_count_mismatch_is_rejected_as_invalid_data() {
    let file_id = registered_file_id();
    let hash = bytes_hash(SOURCE);
    let binding = source_binding(&hash);
    let mut cached = sample(file_id);
    let _dropped = cached.signatures.pop();
    assert_rejected(
        &encode(&cached, &binding),
        &binding,
        file_id,
        "signature/fingerprint count mismatch",
    );
}

// A blob written by a superseded layout carries an old magic and must
// decode as a miss-grade error, never as a hit under today's rules.
#[test]
fn superseded_magics_are_rejected() {
    let file_id = registered_file_id();
    let hash = bytes_hash(SOURCE);
    let binding = source_binding(&hash);
    let encoded = encoded_sample(file_id);
    for old_magic in [0xC0DE_D17E_u32, 0xC0DE_D17F_u32] {
        let mut blob = old_magic.to_le_bytes().to_vec();
        blob.extend_from_slice(encoded.get(4..).unwrap_or_default());
        assert_eq!(
            blob.len(),
            encoded.len(),
            "the re-stamped blob must differ from the original only in its magic"
        );
        assert_rejected(&blob, &binding, file_id, "superseded magic");
    }
}

// [PIPELINE-INCREMENTAL-INTEGRITY] One flipped signature byte — tree,
// fingerprints, counts, and length untouched — must fail the binding
// digest. This is the audit's blocker 1: the old format served the
// corrupted signatures as a valid hit and `token_jaccard` moved.
#[test]
fn a_flipped_signature_byte_fails_the_binding_digest() {
    let file_id = registered_file_id();
    let hash = bytes_hash(SOURCE);
    let binding = source_binding(&hash);
    let mut blob = encoded_sample(file_id);
    if let Some(last) = blob.last_mut() {
        *last ^= 0xFF;
    }
    assert_rejected(&blob, &binding, file_id, "flipped signature byte");
}

// [PIPELINE-INCREMENTAL-INTEGRITY] A valid blob presented under any
// other address — different source bytes, different language
// partition, different `min_nodes` partition — must fail verification
// identically. This is the audit's blocker 2: the old format carried
// no binding, so a moved blob was served under whatever filename it
// sat at.
#[test]
fn a_blob_is_never_served_under_a_different_address() {
    let file_id = registered_file_id();
    let hash = bytes_hash(SOURCE);
    let blob = encoded_sample(file_id);
    let other_hash = bytes_hash(b"pub fn thrice(value: i32) -> i32 { value * 3 }\n");
    let wrong_source = source_binding(&other_hash);
    let wrong_language = BlobBinding {
        language_id: "javascript",
        ..source_binding(&hash)
    };
    let wrong_min_nodes = BlobBinding {
        min_nodes: 9,
        ..source_binding(&hash)
    };
    // The tool version is the axis a *relocation* attacks: every other
    // field is reproduced by the directory the blob sits in, so lifting
    // a blob from `<lang>/<old-version>/<min>/` into the current
    // version's partition presents an otherwise-perfect address. Without
    // `tool_version` inside the digest, that blob verifies and this
    // binary serves fingerprints built by different normalisation rules.
    let wrong_tool_version = BlobBinding {
        tool_version: "0.0.0-superseded",
        ..source_binding(&hash)
    };
    assert_rejected(&blob, &wrong_source, file_id, "wrong source hash");
    assert_rejected(&blob, &wrong_language, file_id, "wrong language partition");
    assert_rejected(
        &blob,
        &wrong_min_nodes,
        file_id,
        "wrong min_nodes partition",
    );
    assert_rejected(
        &blob,
        &wrong_tool_version,
        file_id,
        "blob relocated across tool versions",
    );
}

// [PIPELINE-INCREMENTAL-INTEGRITY] Trailing bytes past the payload
// must be rejected — the digest covers every byte after the header, so
// appended data can never verify. The old format accepted it.
#[test]
fn trailing_bytes_fail_the_binding_digest() {
    let file_id = registered_file_id();
    let hash = bytes_hash(SOURCE);
    let binding = source_binding(&hash);
    let mut blob = encoded_sample(file_id);
    blob.push(0xAB);
    assert_rejected(&blob, &binding, file_id, "trailing byte");
}

/// Assembles a well-formed header around `payload` — valid magic and a
/// binding digest honestly computed over the payload — so a test can
/// prove the *payload parser* rejects malformed lengths on its own,
/// with the digest deliberately not in the way.
fn blob_with_valid_digest(payload: &[u8], binding: &BlobBinding<'_>) -> Vec<u8> {
    let mut blob = MAGIC.to_le_bytes().to_vec();
    blob.extend_from_slice(&binding_digest(binding, payload));
    blob.extend_from_slice(payload);
    blob
}

// [PIPELINE-INCREMENTAL-INTEGRITY] A corrupt record count must become
// `InvalidData` before it can size any allocation. This is the audit's
// blocker 4: a `u64::MAX` fingerprint count aborted the process with a
// `capacity overflow` panic instead of degrading to a miss.
#[test]
fn corrupt_record_counts_are_rejected_before_any_allocation() {
    let file_id = registered_file_id();
    let hash = bytes_hash(SOURCE);
    let binding = source_binding(&hash);
    for bomb in [u64::MAX, 1_000_000] {
        let mut payload = Vec::new();
        encode_tree(&sample(file_id).tree, &mut payload);
        payload.extend_from_slice(&bomb.to_le_bytes());
        let blob = blob_with_valid_digest(&payload, &binding);
        assert_rejected(&blob, &binding, file_id, "fingerprint count bomb");
    }
}

// [PIPELINE-INCREMENTAL-INTEGRITY] Corrupt node lengths — a kind
// length or child count far past the blob — must be rejected before
// allocation, same contract as the record counts.
#[test]
fn corrupt_node_lengths_are_rejected_before_any_allocation() {
    let file_id = registered_file_id();
    let hash = bytes_hash(SOURCE);
    let binding = source_binding(&hash);

    let kind_bomb = u32::MAX.to_le_bytes().to_vec();
    let blob = blob_with_valid_digest(&kind_bomb, &binding);
    assert_rejected(&blob, &binding, file_id, "kind length bomb");

    let mut child_bomb = Vec::new();
    child_bomb.extend_from_slice(&0_u32.to_le_bytes()); // empty kind
    child_bomb.extend_from_slice(&0_u64.to_le_bytes()); // start
    child_bomb.extend_from_slice(&12_u64.to_le_bytes()); // end
    child_bomb.extend_from_slice(&u32::MAX.to_le_bytes()); // child count
    let blob = blob_with_valid_digest(&child_bomb, &binding);
    assert_rejected(&blob, &binding, file_id, "child count bomb");

    let mut invalid_kind = 1_u32.to_le_bytes().to_vec();
    invalid_kind.push(0xFF);
    invalid_kind.extend_from_slice(&0_u64.to_le_bytes());
    invalid_kind.extend_from_slice(&12_u64.to_le_bytes());
    invalid_kind.extend_from_slice(&0_u32.to_le_bytes());
    let blob = blob_with_valid_digest(&invalid_kind, &binding);
    assert_rejected(&blob, &binding, file_id, "invalid UTF-8 node kind");
}

// [PIPELINE-INCREMENTAL-INTEGRITY] A blob file past the size bound is
// never read into memory — the bound protects the read allocation the
// decode-side checks cannot reach — and the oversized file is left on
// disk untouched for the next write to heal.
#[test]
fn an_oversized_blob_file_is_never_read() -> io::Result<()> {
    let tmp = tempfile::tempdir()?;
    let cache = FingerprintCache::open(tmp.path(), "rust", 8)?;
    let path = cache.blob_path(&bytes_hash(SOURCE));
    let file = fs::File::create(&path)?;
    let oversized = MAX_BLOB_BYTES.saturating_add(1);
    file.set_len(oversized)?;
    assert!(
        cache.get(SOURCE, registered_file_id()).is_none(),
        "a blob past the size bound must be a miss without being read"
    );
    assert_eq!(
        fs::metadata(&path)?.len(),
        oversized,
        "the refused blob must be left exactly as found — a lookup never \
         mutates the store"
    );
    Ok(())
}

// [PIPELINE-INCREMENTAL-INTEGRITY] The digest must be a *function* of
// the whole address: stable for one address, and distinct for every
// single-field change. Without the second half the verification is
// theatre — a digest that ignored `min_nodes` would still pass every
// round-trip test while serving blobs across partitions.
#[test]
fn the_binding_digest_is_stable_per_address_and_distinct_across_addresses() {
    let hash = bytes_hash(SOURCE);
    let other_hash = bytes_hash(b"fn other() {}\n");
    let payload = b"payload bytes".as_slice();
    let base = source_binding(&hash);
    assert_eq!(
        binding_digest(&base, payload),
        binding_digest(&source_binding(&hash), payload),
        "one address over one payload must always digest identically, or a \
         freshly-written blob would fail its own verification"
    );
    let variants = [
        (
            "source hash",
            BlobBinding {
                source_hash: &other_hash,
                ..source_binding(&hash)
            },
        ),
        (
            "language",
            BlobBinding {
                language_id: "javascript",
                ..source_binding(&hash)
            },
        ),
        (
            "min_nodes",
            BlobBinding {
                min_nodes: 9,
                ..source_binding(&hash)
            },
        ),
        (
            "tool version",
            BlobBinding {
                tool_version: "0.0.0-superseded",
                ..source_binding(&hash)
            },
        ),
    ];
    for (field, variant) in variants {
        assert_ne!(
            binding_digest(&base, payload),
            binding_digest(&variant, payload),
            "changing the {field} must change the digest, or blobs are \
             interchangeable across that axis"
        );
    }
    assert_ne!(
        binding_digest(&base, payload),
        binding_digest(&base, b"payload byteS"),
        "changing one payload byte must change the digest"
    );
}

// [PIPELINE-INCREMENTAL-INTEGRITY] The format constants are the
// compatibility contract with every blob already on disk. Bumping one
// silently is how a stale-but-addressable blob gets served: the header
// says v3, the values inside are from an older semantics. Pin them so a
// change has to be deliberate, and paired with a bump of the other.
#[test]
fn the_blob_format_revisions_are_pinned() {
    assert_eq!(
        MAGIC, 0xC0DE_D180,
        "the layout magic changes only when the byte layout changes; \
         superseded values must stay rejected, never reused"
    );
    assert_eq!(
        SEMANTIC_EPOCH, 4,
        "the semantic epoch changes when parse/normalise/fingerprint/signature \
         *meaning* changes without moving a byte — the case the `0.0.0-dev` \
         directory partition cannot invalidate. 4 is the framing-token drop \
         (gh #147); bumping past it requires a new dated entry on the \
         constant's doc, then this pin"
    );
    assert_eq!(
        SIGNATURE_LEN * 8,
        1024,
        "a signature is 128 little-endian u64 slots; the width is bound into \
         the digest, so changing it must invalidate every stored blob"
    );
}

// [PIPELINE-INCREMENTAL-INTEGRITY] The cache API end to end: a stored
// bundle is served back under its own source, and the same blob copied
// under another source's address is refused — the unit-level twin of
// the E2E blob-swap scenario.
#[test]
fn the_cache_serves_its_own_address_and_refuses_a_copied_one() -> io::Result<()> {
    let other_source: &[u8] = b"pub fn thrice(value: i32) -> i32 { value * 3 }\n";
    let tmp = tempfile::tempdir()?;
    let cache = FingerprintCache::open(tmp.path(), "rust", 8)?;
    let file_id = registered_file_id();
    cache.store(SOURCE, &sample(file_id))?;

    let served = cache.get(SOURCE, file_id);
    assert!(
        served.is_some_and(|cached| cached.fingerprints == sample(file_id).fingerprints),
        "the stored bundle must be served back under its own source"
    );

    let _copied = fs::copy(
        cache.blob_path(&bytes_hash(SOURCE)),
        cache.blob_path(&bytes_hash(other_source)),
    )?;
    assert!(
        cache.get(other_source, file_id).is_none(),
        "a blob copied under another source's address must be refused"
    );
    Ok(())
}
