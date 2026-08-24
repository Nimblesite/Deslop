//! The deterministic embedding vector [`MockOllama`] serves
//! ([FUSION-EMBED-PROVIDER], GH #369).
//!
//! Split from `mock_ollama.rs` so the HTTP mock owns the transport and
//! this module owns the one question that decides every embedding
//! assertion in the suite: what vector does a given snippet get, and
//! therefore what cosine does a given pair measure.
//!
//! The vector is an honest content statistic — a signed feature hash of
//! the snippet's distinct 5-byte shingles — so cosine tracks lexical
//! overlap and nothing else. That is deliberate, and it is also why
//! declared semantic groups exist: a Type-4 clone is behaviour-equal and
//! text-different, so no statistic over the text can score it, and a
//! test that needs one states the ground truth instead of pretending a
//! shingle hash discovered it.

use std::collections::BTreeSet;

/// Width of the deterministic content-sensitive test embedding.
pub(crate) const MOCK_EMBEDDING_DIMENSIONS: usize = 128;

/// Byte width of one content shingle.
const MOCK_SHINGLE_WIDTH: usize = 5;
/// Stable FNV-1a offset basis for shingle hashing.
const FNV_OFFSET_BASIS: u64 = 14_695_981_039_346_656_037;
/// Stable FNV-1a prime for shingle hashing.
const FNV_PRIME: u64 = 1_099_511_628_211;

/// Weight of one declared semantic-group component, sized to dominate
/// the shingle mass of any fixture-scale snippet: a whole test function
/// carries a few hundred distinct shingles (norm ≈ 25), so two
/// same-group snippets measure ≥ 0.9 while unmarked snippets keep the
/// pure shingle cosine.
const SEMANTIC_GROUP_WEIGHT: f32 = 100.0;

/// Returns a deterministic signed feature hash of the snippet's distinct
/// five-byte shingles. Content overlap drives cosine: renamed clones
/// stay close while unrelated snippets of coincidentally similar length do
/// not inherit the near-unit floor of the deleted four-lane vector (#369).
///
/// A snippet containing any marker of a declared semantic group
/// ([`MockOllama::spawn_semantic`]) additionally receives that group's
/// dominant shared component — the mock reporting the behaviour-level
/// verdict a real model would, which no content statistic can reach for
/// a genuine Type-4 pair.
pub(crate) fn embed_vector(text: &str, semantic_groups: &[Vec<String>]) -> Vec<f32> {
    let mut vector = vec![0.0_f32; MOCK_EMBEDDING_DIMENSIONS];
    for shingle in distinct_shingles(text) {
        let hash = shingle_hash(shingle);
        let lane = usize::from(u8::try_from(hash & 0x7F).unwrap_or_default());
        let sign = if hash & 0x80 == 0 { 1.0_f32 } else { -1.0_f32 };
        if let Some(slot) = vector.get_mut(lane) {
            *slot += sign;
        }
    }
    apply_semantic_groups(&mut vector, text, semantic_groups);
    vector
}

/// Adds the dominant shared component for every declared semantic group
/// whose marker the snippet contains, one reserved lane per group from
/// the top of the vector down.
fn apply_semantic_groups(vector: &mut [f32], text: &str, semantic_groups: &[Vec<String>]) {
    for (group_index, group) in semantic_groups.iter().enumerate() {
        if group.iter().any(|marker| text.contains(marker)) {
            let lane = MOCK_EMBEDDING_DIMENSIONS
                .saturating_sub(1)
                .saturating_sub(group_index);
            if let Some(slot) = vector.get_mut(lane) {
                *slot += SEMANTIC_GROUP_WEIGHT;
            }
        }
    }
}

/// Distinct byte shingles, with one whole-text feature for short inputs.
fn distinct_shingles(text: &str) -> BTreeSet<&[u8]> {
    let bytes = text.as_bytes();
    if bytes.len() < MOCK_SHINGLE_WIDTH {
        return std::iter::once(bytes).collect();
    }
    bytes.windows(MOCK_SHINGLE_WIDTH).collect()
}

/// Stable 64-bit FNV-1a hash of one shingle.
fn shingle_hash(shingle: &[u8]) -> u64 {
    shingle.iter().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}
