//! Applies the shared-subtree rescue over the candidate set
//! ([FUSION-SHARED-SUBTREE], gh #408).
//!
//! The loop is corpus-scale — tens of millions of scanned candidates on
//! a large repository — so its observability contract is aggregate
//! counters plus fixed-interval progress records, never per-pair events
//! ([PIPELINE-OBSERVABILITY-STAGES]). The per-pair debug record this
//! module once emitted produced 793,076 lines on the Flutter corpus and
//! materially slowed the exact stage it was reporting on.

use std::time::Instant;

use crate::{
    ast::NormalizedNode,
    fingerprint::Fingerprint,
    observe::{bump, elapsed_ms},
    pair::{
        CandidatePair, SHARED_SUBTREE_MIN_JACCARD, SHARED_SUBTREE_MIN_NODE_COUNT,
        SHARED_SUBTREE_MIN_OVERLAP,
    },
};

use super::{MeasureStats, OverlapMeasurer};

/// Measured pairs between progress records. Count-based so the cadence
/// is deterministic for a given candidate set; each record carries
/// elapsed time so throughput is readable from any two records
/// ([PIPELINE-OBSERVABILITY-STAGES]).
const RESCUE_PROGRESS_INTERVAL: u64 = 25_000;

/// Aggregate counters for one rescue pass
/// ([PIPELINE-OBSERVABILITY-STAGES]).
#[derive(Debug, Default, Clone, Copy)]
struct RescueStats {
    /// Candidate pairs scanned.
    scanned: u64,
    /// Pairs that passed eligibility plus the cross-file gate and were
    /// measured.
    measured: u64,
    /// Measured pairs whose overlap clears the admission floor.
    rescued: u64,
}

/// Measures shared-subtree overlap onto every candidate pair the fused
/// threshold would otherwise drop despite corroborating token evidence
/// ([FUSION-SHARED-SUBTREE]). Only those pairs are measured: aligning
/// two subtrees for all candidates would repeat the admission-cost
/// mistake [FUSION-CONTENT-GATE] deliberately avoids, and a pair that
/// already survives needs no rescue.
pub fn apply_shared_subtree_rescue(
    pairs: &mut [CandidatePair],
    fingerprints: &[Fingerprint],
    trees: &[NormalizedNode],
) {
    let mut measurer = OverlapMeasurer::new(trees);
    let mut stats = RescueStats::default();
    let started = Instant::now();
    for pair in pairs.iter_mut() {
        bump(&mut stats.scanned);
        if !measure_one(pair, fingerprints, &mut measurer, &mut stats) {
            continue;
        }
        if stats.measured % RESCUE_PROGRESS_INTERVAL == 0 {
            log_progress(stats, measurer.stats(), started);
        }
    }
    log_completion(stats, measurer.stats(), started);
}

/// Measures one pair when it is eligible, resolvable, and cross-file;
/// returns whether a measurement happened.
fn measure_one(
    pair: &mut CandidatePair,
    fingerprints: &[Fingerprint],
    measurer: &mut OverlapMeasurer<'_>,
    stats: &mut RescueStats,
) -> bool {
    if !rescue_eligible(pair) {
        return false;
    }
    let (Some(left), Some(right)) = (fingerprints.get(pair.left), fingerprints.get(pair.right))
    else {
        return false;
    };
    if !crosses_files(left, right) {
        return false;
    }
    bump(&mut stats.measured);
    pair.shared_subtree_overlap = measurer.rescue_overlap(left, right);
    if pair.shared_subtree_overlap >= SHARED_SUBTREE_MIN_OVERLAP {
        bump(&mut stats.rescued);
    }
    true
}

/// True when the pair's endpoints live in different files.
///
/// The rescue is deliberately cross-file only. Every clone this route
/// exists to recover is a copy *between* files ([FUSION-SHARED-SUBTREE],
/// gh #408), and admitting same-file pairs on shape overlap is the
/// #197 in-file sibling-family shape, which the report already spends a
/// dedicated proof suppressing. It is also what keeps a single-file
/// corpus intact: same-file rescues union that file's subtrees into one
/// transitive component, and the same-file overlap collapse then
/// reduces it to a single logical location, which is dropped below
/// `MIN_REPORTABLE_MEMBERS` — so the file's real duplication
/// disappeared entirely rather than being reported
/// (`issue_119_role_gate_exercised`).
fn crosses_files(left: &Fingerprint, right: &Fingerprint) -> bool {
    left.file_id != right.file_id
}

/// True for a pair worth measuring: dropped below its fused floor on a
/// zero structural anchor, yet carrying the token corroboration and
/// endpoint substance the rescue route requires.
fn rescue_eligible(pair: &CandidatePair) -> bool {
    let score = pair.score.finite();
    score.structural <= 0.0
        && score.bounded_fused() < pair.fused_min_score
        && score.token_jaccard >= SHARED_SUBTREE_MIN_JACCARD
        && pair.endpoint_node_counts.0 >= SHARED_SUBTREE_MIN_NODE_COUNT
}

/// One fixed-interval progress record ([PIPELINE-OBSERVABILITY-STAGES]).
fn log_progress(stats: RescueStats, measure: MeasureStats, started: Instant) {
    tracing::info!(
        scanned = stats.scanned,
        measured = stats.measured,
        rescued = stats.rescued,
        alignments = measure.alignments,
        exact_hits = measure.exact_hits,
        bound_skips = measure.bound_skips,
        elapsed_ms = elapsed_ms(started),
        "shared-subtree rescue progress"
    );
}

/// The stage-completion record. Emitted even when nothing was eligible,
/// so a stage that measured nothing is distinguishable from one that
/// never finished ([PIPELINE-OBSERVABILITY-STAGES]).
fn log_completion(stats: RescueStats, measure: MeasureStats, started: Instant) {
    tracing::info!(
        scanned = stats.scanned,
        measured = stats.measured,
        rescued_pairs = stats.rescued,
        alignments = measure.alignments,
        credit_fallbacks = measure.credit_fallbacks,
        hash_equal = measure.hash_equal,
        exact_hits = measure.exact_hits,
        bound_hits = measure.bound_hits,
        bound_skips = measure.bound_skips,
        unresolved = measure.unresolved,
        elapsed_ms = elapsed_ms(started),
        "shared-subtree rescue overlaps measured"
    );
}
