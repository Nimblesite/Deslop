//! Reproducible workload for rendered cluster-signal measurement.

use std::{collections::HashMap, hint::black_box, path::PathBuf};

use crate::{
    ast::{ByteRange, NormalizedNode},
    fingerprint::Fingerprint,
    lsh::{Signature, SignatureIndex, SIGNATURE_LEN},
    overlap::{
        benchmark::{benchmark_report, measure_repeated, BenchmarkReport},
        OverlapMeasurer,
    },
    state::{FileId, FileRegistry},
};

use super::signals::measured_signals;

/// Rendered occurrences in the widest cluster called out by the profile.
const OCCURRENCE_COUNT: usize = 877;

/// Byte width assigned to each synthetic occurrence.
const OCCURRENCE_BYTES: usize = 8;

/// Nominal node count; equal hashes answer before tree resolution.
const OCCURRENCE_NODES: usize = 3;

/// Stable workload name in benchmark artifacts.
const WORKLOAD_NAME: &str = "repeated-exact-877";

/// Bit rotation separating the token score in the checksum.
const TOKEN_CHECKSUM_ROTATION: u32 = 1;

/// Bit rotation separating the embedding score in the checksum.
const EMBEDDING_CHECKSUM_ROTATION: u32 = 2;

/// Keeps repeated score checksums below `usize` saturation.
const SCORE_CHECKSUM_MODULUS: u64 = 1_000_003;

/// Merkle hash byte shared by the exact group.
const HASH_SEED: u8 = 1;

/// `MinHash` slot value shared by the exact group.
const SIGNATURE_FILL: u64 = 1;

/// In-memory corpus reused across timing samples.
struct Corpus {
    /// Empty by design: equal hashes answer before tree resolution.
    trees: Vec<NormalizedNode>,
    /// Occurrence fingerprints in corpus order.
    fingerprints: Vec<Fingerprint>,
    /// Repeated `MinHash` groups in corpus order.
    signatures: Vec<Signature>,
    /// Every occurrence index, in rendered order.
    occurrences: Vec<usize>,
}

/// Measures repeated cluster-signal valuation without scanning a repo.
///
/// # Errors
///
/// Returns an error when `repetitions` is zero.
pub fn measure(label: &str, repetitions: usize) -> Result<BenchmarkReport, &'static str> {
    if repetitions == 0 {
        return Err("repetitions must be positive");
    }
    let corpus = Corpus::new();
    let workload = measure_repeated(
        WORKLOAD_NAME,
        OCCURRENCE_COUNT,
        OCCURRENCE_COUNT,
        repetitions,
        |count| run(&corpus, count),
    );
    Ok(benchmark_report(label, repetitions, vec![workload]))
}

impl Corpus {
    /// Builds the deterministic repeated-group population.
    fn new() -> Self {
        let mut registry = FileRegistry::new();
        let file_id = registry.register(PathBuf::from("cluster-signals.rs"));
        Self {
            trees: Vec::new(),
            fingerprints: (0..OCCURRENCE_COUNT)
                .map(|index| fingerprint(file_id, index))
                .collect(),
            signatures: vec![signature(); OCCURRENCE_COUNT],
            occurrences: (0..OCCURRENCE_COUNT).collect(),
        }
    }
}

/// Fingerprint with the one exact hash shared by every occurrence.
fn fingerprint(file_id: FileId, index: usize) -> Fingerprint {
    let start = index.saturating_mul(OCCURRENCE_BYTES);
    let end = start.saturating_add(OCCURRENCE_BYTES);
    Fingerprint {
        hash: [HASH_SEED; 32],
        file_id,
        byte_range: ByteRange { start, end },
        node_count: OCCURRENCE_NODES,
    }
}

/// `MinHash` signature shared by every occurrence.
const fn signature() -> Signature {
    [SIGNATURE_FILL; SIGNATURE_LEN]
}

/// Executes the signal build repeatedly and returns a value checksum.
fn run(corpus: &Corpus, repetitions: usize) -> usize {
    let signatures = SignatureIndex::from_slice(&corpus.signatures);
    let vectors: HashMap<usize, Vec<f32>> = HashMap::new();
    // The benchmark prices the valuation cost of the widest profile
    // cluster, so every rendered pair is admitted — the pair set the
    // measurement folds is the same population the old all-pairs loop
    // valued ([FUSED-CLUSTER-SIGNALS]).
    let mut admitted_pairs: Vec<(usize, usize)> = Vec::new();
    for (position, &left) in corpus.occurrences.iter().enumerate() {
        for &right in corpus.occurrences.iter().skip(position.saturating_add(1)) {
            admitted_pairs.push((left, right));
        }
    }
    (0..repetitions).fold(0_usize, |checksum, _iteration| {
        let mut overlap = OverlapMeasurer::new(&corpus.trees);
        let measured = measured_signals(
            &corpus.occurrences,
            &admitted_pairs,
            &corpus.fingerprints,
            &signatures,
            &vectors,
            &mut overlap,
        );
        checksum.saturating_add(black_box(score_checksum(measured.score)))
    })
}

/// Stable checksum over every rendered signal value.
fn score_checksum(score: crate::pair::PairScore) -> usize {
    let bits = score.structural.to_bits()
        ^ score
            .token_jaccard
            .to_bits()
            .rotate_left(TOKEN_CHECKSUM_ROTATION)
        ^ score
            .embedding_cos
            .to_bits()
            .rotate_left(EMBEDDING_CHECKSUM_ROTATION);
    usize::try_from(bits % SCORE_CHECKSUM_MODULUS).unwrap_or(usize::MAX)
}
