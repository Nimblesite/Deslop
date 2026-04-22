//! Public embedding-pair API coverage.

use std::path::PathBuf;

use deslop_core::{
    ast::ByteRange, embedding::embedding_pairs, fingerprint::Fingerprint, state::FileRegistry,
};

#[test]
fn embedding_pairs_dedupes_bidirectional_hits_and_filters_weak_neighbors() {
    let (fingerprints, embeddings) = fixture_vectors();
    let pairs = embedding_pairs(&fingerprints, &embeddings);
    assert_eq!(pairs.len(), 1, "expected one high-cosine pair: {pairs:?}");
    let pair = pairs[0];
    assert_eq!((pair.left, pair.right), (0, 1));
    assert!(pair.cosine > 0.99, "cosine too low: {}", pair.cosine);
}

#[test]
fn embedding_pairs_rejects_mismatched_input_lengths() {
    let (fingerprints, mut embeddings) = fixture_vectors();
    let _dropped = embeddings.pop();
    assert!(embedding_pairs(&fingerprints, &embeddings).is_empty());
}

fn fixture_vectors() -> (Vec<Fingerprint>, Vec<Vec<f32>>) {
    let mut registry = FileRegistry::new();
    let fingerprints = (0..3)
        .map(|index| {
            let file_id = registry.register(PathBuf::from(format!("file-{index}.cs")));
            fingerprint(file_id, index)
        })
        .collect();
    let embeddings = vec![
        vec![1.0, 0.0, 0.0],
        vec![0.999, 0.001, 0.0],
        vec![-1.0, 0.0, 0.0],
    ];
    (fingerprints, embeddings)
}

fn fingerprint(file_id: deslop_core::state::FileId, index: usize) -> Fingerprint {
    Fingerprint {
        hash: [u8::try_from(index).unwrap_or_default(); 32],
        file_id,
        byte_range: ByteRange {
            start: index.saturating_mul(10),
            end: index.saturating_mul(10).saturating_add(5),
        },
        node_count: 5,
    }
}
