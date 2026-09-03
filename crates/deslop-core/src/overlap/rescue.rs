//! Applies the shared-subtree rescue over the candidate set
//! ([FUSED-SHARED-SUBTREE], gh #408).
//!
//! Only pairs the fused threshold would otherwise drop — despite
//! corroborating token evidence — are measured: aligning two subtrees for
//! all candidates would repeat the admission-cost mistake
//! [FUSED-CONTENT-GATE] deliberately avoids, and a pair that already
//! survives needs no rescue.
//!
//! The pass is measured work over a corpus-scale population, so it runs
//! sharded across the available cores ([PERF-FLUTTER-TODO-RESCUE]): each
//! shard owns a disjoint slice of the candidate list and its own
//! [`OverlapMeasurer`]. Every measurement is a pure function of the
//! corpus, so sharding changes no value — only which thread computes it.
//! Shard results merge in shard order, keeping the reported counters
//! deterministic.
//!
//! Observability is the aggregate gate counters in [`super::tally`] plus
//! fixed-interval progress records, never per-pair events
//! ([PERF-FLUTTER-TODO-OBSERVABILITY]).

use std::{collections::HashMap, hash::BuildHasher, num::NonZeroUsize};

use crate::{
    ast::NormalizedNode,
    cluster::scope::DeclarationScopes,
    content::pair_content_agreement,
    fingerprint::{ranges_overlap, Fingerprint},
    pair::{
        crosses_files, rescue_eligible, CandidatePair, ExactClones, RESCUE_MIN_CONTENT_AGREEMENT,
        SHARED_SUBTREE_MIN_NODE_COUNT, SHARED_SUBTREE_MIN_OVERLAP,
    },
    state::FileId,
};

use super::{tally::RescueTally, OverlapMeasurer};

/// Everything a rescue measurement reads besides the pair itself,
/// resolved once per pass and shared read-only by every shard.
pub(super) struct RescueContext<'a, S, L: BuildHasher> {
    /// Every member's normalised tree by file, for content agreement.
    tree_index: HashMap<FileId, &'a NormalizedNode>,
    /// Raw source per file.
    sources: &'a HashMap<FileId, Vec<u8>, S>,
    /// Language per file.
    languages: &'a HashMap<FileId, &'static str, L>,
    /// The exact whole-function clones a container may not merely wrap
    /// ([FUSED-SHARED-SUBTREE-ECHO]).
    anchors: ExactClones,
    /// The exact clones inside each file, for the same-file scope rule
    /// ([FUSED-SHARED-SUBTREE-SAME-FILE]).
    interiors: ExactClones,
    /// Authored declarations per file, for the same-file scope rule
    /// ([FUSED-SHARED-SUBTREE-SAME-FILE]).
    scopes: DeclarationScopes<'a, L>,
}

impl<'a, S: BuildHasher, L: BuildHasher> RescueContext<'a, S, L> {
    /// Resolves the pass-wide inputs for `pairs`.
    pub(super) fn new(
        pairs: &[CandidatePair],
        fingerprints: &[Fingerprint],
        trees: &'a [NormalizedNode],
        sources: &'a HashMap<FileId, Vec<u8>, S>,
        languages: &'a HashMap<FileId, &'static str, L>,
    ) -> Self {
        let scopes = DeclarationScopes::new(trees, languages);
        let anchors = ExactClones::whole_functions_across_files(pairs, fingerprints, &scopes);

        Self {
            tree_index: trees.iter().map(|tree| (tree.file_id, tree)).collect(),
            sources,
            languages,
            anchors,
            interiors: ExactClones::within_one_file(pairs, fingerprints),
            scopes,
        }
    }

    /// [FUSED-SHARED-SUBTREE-SAME-FILE] Whether the rescue may measure
    /// this pair at all.
    ///
    /// Across files every eligible pair is measured. Inside one file the
    /// route is open only to two endpoints that are each a whole
    /// authored declaration — modifier through closing brace — and are
    /// disjoint. Two methods that drifted apart inside one file are the
    /// same duplication as two that drifted apart across files; a window
    /// cut over part of one, a nested view of another, and a table row
    /// are none of them a declaration. Admitting those unioned a file's
    /// subtrees into a single component that the same-file collapse then
    /// reduced to one logical location, and the file's real duplication
    /// disappeared rather than being reported
    /// (`issue_119_role_gate_exercised`).
    fn measures(&self, left: &Fingerprint, right: &Fingerprint) -> bool {
        crosses_files(left, right)
            || (!ranges_overlap(left, right)
                && self.scopes.aligned_function(left).is_some()
                && self.scopes.aligned_function(right).is_some()
                && self.shares_a_copied_interior(left, right))
    }

    /// [FUSED-SHARED-SUBTREE-SAME-FILE] Whether the two declarations
    /// still hold a copied interior: a Merkle-equal clone inside both of
    /// them, substantive enough to clear the floor every rescued
    /// endpoint clears ([`SHARED_SUBTREE_MIN_NODE_COUNT`]).
    ///
    /// This is the discriminator between a copy that drifted and a
    /// family that never was one. `csharp-merge-drift`'s two methods
    /// still share four whole statements outright, 32 nodes of authored
    /// code the edit never touched, while the `dart-issue-197` settings
    /// accessors share a skeleton and no statement at all: their overlap
    /// is 0.81 to 0.88, indistinguishable from the drifted pair's 0.84,
    /// and their raw-content agreement reaches 0.56 against its 0.55.
    /// Shape and agreement cannot separate them. Copied code can.
    fn shares_a_copied_interior(&self, left: &Fingerprint, right: &Fingerprint) -> bool {
        self.interiors.enclosed_nodes(left, right) >= SHARED_SUBTREE_MIN_NODE_COUNT
    }

    /// The exact clones this pair could merely be echoing: across files
    /// the whole-function runs, inside one file the file's own exact
    /// clones ([FUSED-SHARED-SUBTREE-ECHO]).
    fn echo_anchors(&self, left: &Fingerprint, right: &Fingerprint) -> &ExactClones {
        if crosses_files(left, right) {
            &self.anchors
        } else {
            &self.interiors
        }
    }
}

/// Fewest candidate pairs worth sharding at all — below this the thread
/// spawn costs more than the measurements.
///
/// The `None` arm is compile-time dead: the literal is non-zero, and
/// [`std::num::NonZeroUsize::new`] is the only stable way to say so in a
/// `const` without `unsafe`.
const MIN_SHARD_WORK: NonZeroUsize = match NonZeroUsize::new(4_096) {
    Some(floor) => floor,
    None => NonZeroUsize::MIN,
};

/// Candidate pairs per claimed chunk. Small enough that a worker which
/// draws a run of expensive endpoints cannot hold the stage open, large
/// enough that claiming a chunk costs nothing beside measuring it.
const RESCUE_CHUNK_PAIRS: usize = 512;

/// Measures shared-subtree overlap onto every rescue-eligible candidate
/// pair, in parallel when the population justifies it.
///
/// `sources` and `languages` let the pass apply the per-edge content
/// gate ([FUSED-CONTENT-GATE], gh #458) to pairs whose overlap cleared
/// the floor: a Merkle-identical signature alone must not admit a pair
/// whose bodies share nothing.
pub fn apply_shared_subtree_rescue<S: BuildHasher + Sync, L: BuildHasher + Sync>(
    pairs: &mut [CandidatePair],
    fingerprints: &[Fingerprint],
    trees: &[NormalizedNode],
    sources: &HashMap<FileId, Vec<u8>, S>,
    languages: &HashMap<FileId, &'static str, L>,
) {
    // Content agreement needs every member's tree and the echo rule
    // needs every exact function pair, both resolved once for the whole
    // pass; each measurement then reads through this context.
    let context = RescueContext::new(pairs, fingerprints, trees, sources, languages);
    let workers = crate::shard::worker_count(pairs.len(), MIN_SHARD_WORK);
    if workers <= 1 {
        let mut measurer = OverlapMeasurer::new(trees);
        let mut tally = RescueTally::new();
        for pair in pairs.iter_mut() {
            tally.scan();
            measure_one(pair, fingerprints, &context, &mut measurer, &mut tally);
        }
        tally.report_total(measurer.stats());
        return;
    }
    // [PERF-FLUTTER-TODO-RESCUE] Many small chunks handed out on
    // demand, not one contiguous block per worker: a measurement's
    // cost grows with its endpoint size, so contiguous blocks of a
    // sorted candidate list leave one worker running long after the
    // rest have finished (20.8 s against a 5.9 s balanced ideal on the
    // Flutter framework slice). Each worker keeps one measurer across
    // every chunk it claims, so the alignment memos still accumulate.
    let (_measured, shards) = crate::shard::map_chunks(
        pairs.chunks_mut(RESCUE_CHUNK_PAIRS),
        workers,
        || (RescueTally::new(), OverlapMeasurer::new(trees)),
        |(tally, measurer), chunk| {
            measure_chunk(chunk, fingerprints, &context, measurer, tally);
        },
    );
    report_shards(&shards);
}

/// Measures one claimed chunk onto the worker's own tally and measurer.
fn measure_chunk<S: BuildHasher, L: BuildHasher>(
    chunk: &mut [CandidatePair],
    fingerprints: &[Fingerprint],
    context: &RescueContext<'_, S, L>,
    measurer: &mut OverlapMeasurer<'_>,
    tally: &mut RescueTally,
) {
    for pair in chunk.iter_mut() {
        tally.scan();
        measure_one(pair, fingerprints, context, measurer, tally);
    }
}

/// Emits the merged, deterministic totals for a sharded run.
///
/// Every counter merged here is additive, so the totals are the same
/// whichever worker claimed which chunk ([PIPELINE-DETERMINISM]).
fn report_shards(shards: &[(RescueTally, OverlapMeasurer<'_>)]) {
    let Some((first, measurer)) = shards.first() else {
        return;
    };
    let mut merged = first.clone();
    let mut totals = measurer.stats();
    for (tally, measurer) in shards.iter().skip(1) {
        merged.absorb(tally);
        totals = totals.add(measurer.stats());
    }
    merged.report_total(totals);
}

/// Measures one pair when it is eligible, resolvable, and cross-file,
/// recording every gate it passes.
///
/// A pair whose overlap cleared the floor still has to carry its own
/// content through the gate ([FUSED-CONTENT-GATE]): the overlap floor
/// is a *structural* claim and the token corroboration a *token*
/// claim, and neither knows whether the endpoints' collapsed leaves
/// agree. When the pair's own content agreement falls below
/// [`RESCUE_MIN_CONTENT_AGREEMENT`] the rescue refuses it — the overlap
/// is left unset so survival drops the pair exactly as if the rescue had
/// never measured it. The same refusal applies to a container that only
/// echoes an exact clone it wraps ([FUSED-SHARED-SUBTREE-ECHO]).
fn measure_one<S: BuildHasher, L: BuildHasher>(
    pair: &mut CandidatePair,
    fingerprints: &[Fingerprint],
    context: &RescueContext<'_, S, L>,
    measurer: &mut OverlapMeasurer<'_>,
    tally: &mut RescueTally,
) {
    if !rescue_eligible(pair) {
        return;
    }
    let (Some(left), Some(right)) = (fingerprints.get(pair.left), fingerprints.get(pair.right))
    else {
        return;
    };
    tally.eligible();
    if !context.measures(left, right) {
        return;
    }
    tally.in_scope(crosses_files(left, right));
    pair.shared_subtree_overlap = measurer.rescue_overlap(left, right);
    record_rescue_verdict(pair, left, right, context, measurer, tally);
}

/// Applies content and echo guards to one measured rescue candidate.
fn record_rescue_verdict<S: BuildHasher, L: BuildHasher>(
    pair: &mut CandidatePair,
    left: &Fingerprint,
    right: &Fingerprint,
    context: &RescueContext<'_, S, L>,
    measurer: &OverlapMeasurer<'_>,
    tally: &mut RescueTally,
) {
    let clears_overlap = pair.shared_subtree_overlap >= SHARED_SUBTREE_MIN_OVERLAP;
    let clears_content = !clears_overlap
        || pair_content_agreement(
            left,
            right,
            &context.tree_index,
            context.sources,
            context.languages,
        ) >= RESCUE_MIN_CONTENT_AGREEMENT;
    if clears_overlap && !clears_content {
        tally.content_gate_rejected();
        pair.shared_subtree_overlap = 0.0;
    }
    let echoes = clears_overlap && clears_content && is_container_echo(pair, left, right, context);
    if echoes {
        tally.container_echo_rejected();
        pair.shared_subtree_overlap = 0.0;
    }
    tally.measure(
        clears_overlap && clears_content && !echoes,
        measurer.stats(),
    );
}

/// [FUSED-SHARED-SUBTREE-ECHO] Whether the pair's shared mass, beyond
/// the largest exact whole-function clone both endpoints enclose, is too
/// small to rescue on its own.
///
/// Shared mass is the overlap share of the larger endpoint
/// ([FUSED-SHARED-SUBTREE]: `S = shared / max(n)`). A container whose
/// remainder falls below [`SHARED_SUBTREE_MIN_NODE_COUNT`] — the floor
/// every rescued endpoint already has to clear — is not a near-miss the
/// anchor axis missed; it is the class shell or preamble around a clone
/// the anchor axis already proved, and admitting it only hands
/// subsumption a wider, byte-divergent view of that clone.
fn is_container_echo<S: BuildHasher, L: BuildHasher>(
    pair: &CandidatePair,
    left: &Fingerprint,
    right: &Fingerprint,
    context: &RescueContext<'_, S, L>,
) -> bool {
    let Some(claimed) = context.echo_anchors(left, right).claimed_nodes(left, right) else {
        return false;
    };
    let larger = left.node_count.max(right.node_count);
    let shared = pair.shared_subtree_overlap * usize_to_f64(larger);
    shared - usize_to_f64(claimed) < usize_to_f64(SHARED_SUBTREE_MIN_NODE_COUNT)
}

/// Node counts as the `f64` the overlap share is measured in.
fn usize_to_f64(nodes: usize) -> f64 {
    u32::try_from(nodes).map_or(f64::MAX, f64::from)
}

/// [PERF-FLUTTER-TODO-RESCUE] Sharded and serial rescue must agree.
#[cfg(test)]
#[path = "rescue/shard_equivalence.rs"]
mod shard_equivalence_tests;
