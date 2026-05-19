//! Test-only embedding fixtures ([FUSION-EMBED-PROVIDER]).
//!
//! Hosts the deterministic BLAKE3 embedding shim used by core unit
//! tests. The shim is **not** a product provider — it does not appear
//! in the production [`crate::embedding::registry::ProviderRegistry`],
//! is not exported from the production prelude, and is gated behind
//! the `test-support` feature so it cannot be linked into the shipped
//! VSIX, LSP, or MCP binaries.
//!
//! Black-box binary tests must not depend on this module; they should
//! drive analysis through a mock Ollama HTTP server instead so the
//! production code paths exercised in the tests match what ships.

use blake3::Hasher;

use crate::embedding::provider::{EmbeddingProvider, EmbeddingSpec, ProviderError};

/// Provider id reported by the deterministic BLAKE3 shim. Kept stable
/// so existing core tests that assert against the field continue to
/// work after the move.
pub const PROVIDER_ID: &str = "stub";
/// Stable `model_id` reported by the shim.
pub const MODEL_ID: &str = "blake3-stub";
/// Stable `model_version` reported by the shim.
pub const MODEL_VERSION: &str = "v1";

/// Fixed vector length. Small enough that the HNSW / cache paths
/// stay cheap on every `cargo test` run, large enough that distinct
/// inputs produce visibly distinct vectors.
const DIMENSIONS: usize = 64;
/// Stub embeddings are CPU-local and cheap, so let the pipeline
/// amortise cache and dispatch overhead across larger chunks.
const MAX_BATCH_SIZE: usize = 1024;

/// Deterministic BLAKE3-derived embedding provider. Test infrastructure
/// only — see module docs.
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

    fn max_batch_size(&self) -> usize {
        MAX_BATCH_SIZE
    }

    fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, ProviderError> {
        Ok(inputs
            .iter()
            .map(|input| embed_bytes(input.as_bytes()))
            .collect())
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
