//! [FUSED-SHARED-SUBTREE-MEMO] A rank fold reuses fully measured rescue overlap.

use std::fs;

use tempfile::TempDir;

use super::ClusterKindMeasurer;
use crate::{
    ast::{ByteRange, NormalizedNode},
    embedding::EmbeddingMode,
    fingerprint::Fingerprint,
    lang::{python::PythonParser, LanguageParser},
    overlap::OverlapMeasurer,
    pair::{CandidatePair, PairScore, FUSED_THRESHOLD, SHARED_SUBTREE_MIN_OVERLAP},
    pipeline::{EmbeddingSettings, PipelineSession},
    state::FileId,
};

const MIN_NODES: u32 = 1;
const LEFT_NAME: &str = "ledger.py";
const RIGHT_NAME: &str = "book.py";
const LEFT_SOURCE: &str = "def settle(item):\n    value = item.base\n    value = value + item.tax\n    value = value + item.fee\n    value = value - item.credit\n    value = value + item.postage\n    item.record(value)\n    return value\n";
const RIGHT_SOURCE: &str = "def settle(item):\n    value = item.base\n    value = value + item.tax\n    value = value + item.fee\n    value = value + item.postage\n    item.record(value)\n    return value\n";
const NO_ALIGNMENTS: u64 = 0;
const ONE_HIT: u64 = 1;
const ONE_ALIGNMENT: u64 = 1;
const NO_HITS: u64 = 0;
const NO_SIGNAL: f64 = 0.0;
const BELOW_FLOOR: f64 = 0.5;
const NO_INDEX: usize = 0;
const NO_NODES: usize = 0;
const BASE_PAIR: CandidatePair = CandidatePair {
    left: NO_INDEX,
    right: NO_INDEX,
    endpoint_node_counts: (NO_NODES, NO_NODES),
    lsh_only_node_floor: NO_NODES,
    lsh_only_min_jaccard: FUSED_THRESHOLD,
    fused_min_score: FUSED_THRESHOLD,
    shared_subtree_overlap: NO_SIGNAL,
    verified_async_core: false,
    score: PairScore {
        structural: NO_SIGNAL,
        token_jaccard: NO_SIGNAL,
        embedding_cos: NO_SIGNAL,
    },
};

fn workspace() -> Result<TempDir, String> {
    let temp = TempDir::new().map_err(|error| error.to_string())?;
    fs::write(temp.path().join(LEFT_NAME), LEFT_SOURCE).map_err(|error| error.to_string())?;
    fs::write(temp.path().join(RIGHT_NAME), RIGHT_SOURCE).map_err(|error| error.to_string())?;
    Ok(temp)
}

fn embeddings_off() -> EmbeddingSettings<'static> {
    EmbeddingSettings {
        mode: EmbeddingMode::Off,
        provider: None,
        batch_yield: None,
        progress: None,
    }
}

fn endpoint(session: &PipelineSession, file_id: FileId) -> Result<usize, String> {
    session
        .store
        .fingerprints()
        .iter()
        .enumerate()
        .filter(|(_, fingerprint)| fingerprint.file_id == file_id)
        .max_by_key(|(_, fingerprint)| fingerprint.node_count)
        .map(|(index, _)| index)
        .ok_or_else(|| "a fingerprint for each source file".to_owned())
}

fn parsed_trees(
    session: &PipelineSession,
    files: &[FileId],
) -> Result<Vec<NormalizedNode>, String> {
    files
        .iter()
        .map(|file_id| {
            let source = session
                .sources
                .get(file_id)
                .ok_or("held fixture source missing")?;
            PythonParser
                .parse_and_normalize(source, *file_id)
                .map_err(|error| error.to_string())
        })
        .collect()
}

fn candidate_pair(
    indices: [usize; 2],
    fingerprints: &[Fingerprint],
    score: f64,
) -> Result<CandidatePair, String> {
    let [left_index, right_index] = indices;
    let (left, right) = indexed_endpoints(fingerprints, indices)?;
    let smaller = left.node_count.min(right.node_count);
    Ok(CandidatePair {
        left: left_index,
        right: right_index,
        endpoint_node_counts: (smaller, left.node_count.max(right.node_count)),
        lsh_only_node_floor: smaller,
        shared_subtree_overlap: score,
        ..BASE_PAIR
    })
}

fn initialized_fixture() -> Result<(PipelineSession, Vec<FileId>), String> {
    let temp = workspace()?;
    let (session, _report) = PipelineSession::initialise(
        temp.path().to_path_buf(),
        MIN_NODES,
        false,
        None,
        embeddings_off(),
    )
    .map_err(|error| error.to_string())?;
    let mut files: Vec<_> = session.sources.keys().copied().collect();
    files.sort_unstable();
    Ok((session, files))
}

fn candidate_indices(session: &PipelineSession, files: &[FileId]) -> Result<[usize; 2], String> {
    let mut endpoints: Vec<_> = files
        .iter()
        .map(|file_id| endpoint(session, *file_id))
        .collect::<Result<_, _>>()?;
    endpoints.sort_unstable();
    endpoints
        .try_into()
        .map_err(|_| "two endpoint files required".to_owned())
}

fn measured_score(
    indices: [usize; 2],
    fingerprints: &[Fingerprint],
    trees: &[NormalizedNode],
) -> Result<f64, String> {
    let (left, right) = indexed_endpoints(fingerprints, indices)?;
    assert_ne!(left.hash, right.hash, "an insertion changes the tree hash");
    let exact = OverlapMeasurer::new(trees).rescue_overlap(left, right);
    assert!(
        exact >= SHARED_SUBTREE_MIN_OVERLAP,
        "rescue exact overlap: {exact}"
    );
    Ok(exact)
}

fn ranked_fixture() -> Result<(PipelineSession, Vec<NormalizedNode>, CandidatePair, f64), String> {
    let (session, files) = initialized_fixture()?;
    let trees = parsed_trees(&session, &files)?;
    let fingerprints = session.store.fingerprints();
    let indices = candidate_indices(&session, &files)?;
    let exact = measured_score(indices, fingerprints, &trees)?;
    let pair = candidate_pair(indices, fingerprints, exact)?;
    Ok((session, trees, pair, exact))
}

fn pair_endpoints<'a>(
    fingerprints: &'a [Fingerprint],
    pair: &CandidatePair,
) -> Result<(&'a Fingerprint, &'a Fingerprint), String> {
    indexed_endpoints(fingerprints, [pair.left, pair.right])
}

fn indexed_endpoints(
    fingerprints: &[Fingerprint],
    indices: [usize; 2],
) -> Result<(&Fingerprint, &Fingerprint), String> {
    let [left_index, right_index] = indices;
    let left = fingerprints
        .get(left_index)
        .ok_or("left endpoint missing")?;
    let right = fingerprints
        .get(right_index)
        .ok_or("right endpoint missing")?;
    Ok((left, right))
}

/// [FUSED-SHARED-SUBTREE-MEMO] Only exact rescue scores can become
/// ranked overlap cache hits; the rank pass must not repeat alignment.
#[test]
fn ranked_fold_reuses_exact_rescue_alignment() -> Result<(), String> {
    let (session, trees, pair, exact) = ranked_fixture()?;
    let fingerprints = session.store.fingerprints();
    let (left, right) = pair_endpoints(fingerprints, &pair)?;
    let judge = ClusterKindMeasurer::new(&session, fingerprints, &trees, &[], &[pair]);
    let mut axes = judge.axes.lock().map_err(|_| "rank axes lock poisoned")?;
    assert_eq!(axes.overlap.overlap(left, right).to_bits(), exact.to_bits());
    assert_eq!(axes.overlap.stats().alignments, NO_ALIGNMENTS);
    assert_eq!(axes.overlap.stats().exact_hits, ONE_HIT);
    Ok(())
}

/// [FUSED-SHARED-SUBTREE-MEMO] A rescue bound below its floor cannot
/// answer ranking's exact overlap query.
#[test]
fn ranked_fold_does_not_promote_rescue_bound() -> Result<(), String> {
    let (session, trees, mut pair, exact) = ranked_fixture()?;
    pair.shared_subtree_overlap = BELOW_FLOOR;
    let fingerprints = session.store.fingerprints();
    let (left, right) = pair_endpoints(fingerprints, &pair)?;
    let judge = ClusterKindMeasurer::new(&session, fingerprints, &trees, &[], &[pair]);
    let mut axes = judge.axes.lock().map_err(|_| "rank axes lock poisoned")?;
    assert_eq!(axes.overlap.overlap(left, right).to_bits(), exact.to_bits());
    assert_eq!(axes.overlap.stats().alignments, ONE_ALIGNMENT);
    assert_eq!(axes.overlap.stats().exact_hits, NO_HITS);
    Ok(())
}

/// [FUSED-SHARED-SUBTREE-MEMO] A cached score never hides an endpoint
/// whose byte range does not resolve to a normalised view.
#[test]
fn seeded_score_does_not_answer_unresolved_endpoint() -> Result<(), String> {
    let (session, trees, pair, exact) = ranked_fixture()?;
    let fingerprints = session.store.fingerprints();
    let (left, right) = pair_endpoints(fingerprints, &pair)?;
    let mut left = left.clone();
    left.byte_range = ByteRange {
        start: usize::MAX,
        end: usize::MAX,
    };
    let mut overlap = OverlapMeasurer::new(&trees);
    overlap.remember_rescued_exact(&left, right, exact);
    assert_eq!(overlap.overlap(&left, right).to_bits(), NO_SIGNAL.to_bits());
    assert_eq!(overlap.stats().alignments, NO_ALIGNMENTS);
    assert_eq!(overlap.stats().exact_hits, NO_HITS);
    Ok(())
}
