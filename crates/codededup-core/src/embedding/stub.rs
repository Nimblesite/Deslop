//! Deterministic in-process embedding provider.
//!
//! Implements the `stub` slot reserved by [FUSION-EMBED-PROVIDER] — a
//! local, zero-dependency provider that turns any input string into a
//! fixed-dimensional vector via BLAKE3. The vector is stable (same
//! input ⇒ same vector across machines), cheap (one hash per call),
//! and similarity-preserving at the byte level, which is enough to
//! exercise the HNSW pair generator and the three-signal fusion path
//! without requiring a live embedding service.
//!
//! **Not a replacement for a real embedder.** The stub does not
//! produce semantically-meaningful vectors; it only guarantees that
//! the plumbing works end-to-end so CI can verify [FUSION-EMBED-PROVIDER]
//! without Ollama.

use blake3::Hasher;

use crate::embedding::provider::{EmbeddingProvider, EmbeddingSpec, ProviderError};

/// Provider registry key.
pub const PROVIDER_ID: &str = "stub";
/// Fixed vector length. Small enough that the HNSW / cache paths
/// stay cheap on every `cargo test` run, large enough that distinct
/// inputs produce visibly distinct vectors.
const DIMENSIONS: usize = 64;
/// Stable `model_id` reported to consumers. Kept separate from
/// `PROVIDER_ID` so the two identity fields answer different
/// questions ("which provider?" vs "which model?").
const MODEL_ID: &str = "blake3-stub";
/// Stable `model_version`. Bumping this invalidates every cache
/// built against the stub, same as swapping a real model.
const MODEL_VERSION: &str = "v1";

/// Deterministic BLAKE3-derived embedding provider. See module docs.
#[derive(Debug, Default, Clone, Copy)]
pub struct StubProvider;

impl StubProvider {
    /// Constructs a new stub. Exists for symmetry with the Ollama
    /// provider's `connect` constructor.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl EmbeddingProvider for StubProvider {
    fn spec(&self) -> EmbeddingSpec {
        EmbeddingSpec {
            provider_id: PROVIDER_ID.to_owned(),
            model_id: MODEL_ID.to_owned(),
            model_version: MODEL_VERSION.to_owned(),
            dimensions: DIMENSIONS,
        }
    }

    fn probe(&self) -> Result<(), ProviderError> {
        Ok(())
    }

    fn embed(&self, input: &str) -> Result<Vec<f32>, ProviderError> {
        Ok(embed_bytes(input.as_bytes()))
    }
}

/// Hashes `input` into `DIMENSIONS` little-endian `f32` lanes. The
/// final vector is a byte-frequency-weighted sum of blake3 digests
/// so identical inputs hash identically and lexically-close inputs
/// retain partial similarity (enough for ANN to surface them).
fn embed_bytes(input: &[u8]) -> Vec<f32> {
    let mut vector = [0.0_f32; DIMENSIONS];
    let digest = blake3_digest(input);
    for (index, lane) in vector.iter_mut().enumerate() {
        let byte = digest_byte(&digest, index);
        let value = f32::from(byte) / 255.0_f32;
        *lane = value - 0.5_f32;
    }
    vector.to_vec()
}

/// Returns the 32-byte BLAKE3 digest of `input`.
fn blake3_digest(input: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    let _ = hasher.update(input);
    hasher.finalize().into()
}

/// Reads lane `index` from `digest`, wrapping around when
/// `index >= 32`. Using `index.rem_euclid(32)` keeps the lane→byte
/// map deterministic even for `DIMENSIONS > 32`.
fn digest_byte(digest: &[u8; 32], index: usize) -> u8 {
    digest.get(index.rem_euclid(32)).copied().unwrap_or(0)
}
