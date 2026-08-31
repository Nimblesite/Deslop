//! Embedding-pass batch data and mapping helpers.

use std::collections::HashMap;

use crate::{
    ast::ByteRange,
    embedding::{cosine_similarity, embedding_pairs, EmbeddingPair, EmbeddingSpec},
    fingerprint::Fingerprint,
    report::EmbeddingProvenance,
    state::FileId,
};

/// Accumulates successful vectors and rejected occurrence counts.
#[derive(Debug)]
pub(super) struct EmbeddingBatch {
    /// One entry per distinct successful snippet — the ANN index input.
    pub(super) vectors: Vec<IndexedEmbedding>,
    /// Logical occurrences represented by successful vectors.
    successes: usize,
    /// Logical occurrences skipped because the provider rejected them.
    pub(super) failures: usize,
}

impl EmbeddingBatch {
    /// Creates an empty batch with space for expected successes.
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            vectors: Vec::with_capacity(capacity),
            successes: 0,
            failures: 0,
        }
    }

    /// Adds one successful embedding vector, owned jointly by every
    /// fingerprint that produced this snippet.
    ///
    /// Identical source text is embedded once — the provider call is the
    /// expensive part — and the vector enters the ANN index once, because
    /// N identical points cost N insertions and N queries to return each
    /// other (GH #357). The vector still belongs to *each* fingerprint
    /// that shares it, which is why the owners travel with it: dropping
    /// them here would delete the embedding evidence for exactly the pairs
    /// the tool exists to find. The more perfect the duplicate, the more
    /// certainly its cosine would go missing, and a missing cosine renders
    /// as `embedding_cos = 0.0` — indistinguishable from "measured, and
    /// found unrelated".
    pub(super) fn push(&mut self, fingerprint_indices: &[usize], vector: &[f32]) {
        if fingerprint_indices.is_empty() {
            return;
        }
        self.vectors.push(IndexedEmbedding {
            fingerprint_indices: fingerprint_indices.to_vec(),
            vector: vector.to_vec(),
        });
        self.successes = self.successes.saturating_add(fingerprint_indices.len());
    }

    /// Occurrences that hold a vector — the logical success count, which
    /// duplicate collapse deliberately no longer equals.
    pub(super) fn successes(&self) -> usize {
        self.successes
    }

    /// Returns successful vectors plus rejected occurrences.
    pub(super) fn processed(&self) -> usize {
        self.successes.saturating_add(self.failures)
    }
}

/// Provider request waiting to be embedded.
#[derive(Debug)]
pub(super) struct PendingEmbedding {
    /// Every fingerprint index whose source text is this snippet.
    ///
    /// One provider request, many owners: the request is deduplicated by
    /// content hash, the *result* never is. Collapsing these to a single
    /// index is what erased cosines between byte-identical clones.
    pub(super) fingerprint_indices: Vec<usize>,
    /// Source text sent to the provider.
    pub(super) snippet: String,
    /// Stable content hash used for cache writes and diagnostics.
    pub(super) snippet_hash: String,
}

/// One point in the ANN index, plus every fingerprint it stands for.
#[derive(Debug)]
pub(super) struct IndexedEmbedding {
    /// Every fingerprint index whose source text produced this vector,
    /// in corpus order. Never empty.
    fingerprint_indices: Vec<usize>,
    /// Provider-returned vector.
    vector: Vec<f32>,
}

/// Builds ANN pairs from successfully embedded snippets.
///
/// The index is queried over one point per distinct snippet, and each hit
/// is mapped back onto the fingerprints that own the two points. Every
/// owner pair of a hit carries the *same* measured cosine — the owners of
/// a point share one vector — and identical text yields identical
/// signatures and node counts, so the whole owner cross-product also
/// carries one identical [`crate::pair::PairScore`] and one identical
/// survival decision — so the extra edges cannot change a component, only
/// its cost. Measured on a two-file 9 KB fixture whose 3,638 subtrees
/// collapse to 51 vectors: the cross-product emitted 3,046,963 candidate
/// pairs where the linking topology below emits 3,549.
///
/// So the topology is the one `pair::candidates::collect_structural_bucket`
/// already uses for a bucket of identical fingerprints: a star inside each
/// group, and one linking edge between groups. Transitive closure needs no
/// more, and the rendered `embedding_cos` is measured from the vector map
/// over the occurrences the report shows — never from this list.
pub(super) fn pairs_from_successful_embeddings(
    fingerprints: &[Fingerprint],
    indexed: &[IndexedEmbedding],
) -> Vec<EmbeddingPair> {
    let representatives: Vec<Fingerprint> = indexed
        .iter()
        .filter_map(|item| representative_fingerprint(item, fingerprints))
        .collect();
    let vectors: Vec<Vec<f32>> = indexed.iter().map(|item| item.vector.clone()).collect();
    let footprints: Vec<Footprint> = indexed
        .iter()
        .map(|item| Footprint::of(item, fingerprints))
        .collect();
    embedding_pairs(&representatives, &vectors)
        .into_iter()
        .filter_map(|pair| linking_pair(pair, indexed, &footprints))
        .chain(
            indexed
                .iter()
                .flat_map(|item| shared_snippet_pairs(item, fingerprints)),
        )
        .collect()
}

/// Returns the fingerprint standing in for one indexed point.
fn representative_fingerprint(
    item: &IndexedEmbedding,
    fingerprints: &[Fingerprint],
) -> Option<Fingerprint> {
    item.fingerprint_indices
        .first()
        .and_then(|&index| fingerprints.get(index))
        .cloned()
}

/// Maps one ANN hit back onto the fingerprints the two points stand for.
///
/// The link is refused when the two points' [`Footprint`]s touch, which
/// is the group-level reading of the rule [`ranges_do_not_overlap`]
/// applies to a single pair: two snippets that appear nested in some file
/// are an ancestor and its descendant, not two clones, and an edge
/// between them fuses a whole file's nested windows into one
/// transitive component.
///
/// Before the duplicate collapse this could not happen often enough to
/// notice — a snippet repeated N times filled its own top-k with its own
/// twins, so the ANN pass rarely reached a neighbouring snippet at all,
/// and the per-pair guard caught the few that got through. Collapsing the
/// index is exactly the removal of that accident: one point now has k
/// slots for *different* snippets, and in a single file the nearest
/// different snippets are the enclosing statement, the sibling window and
/// the sub-expression. Measured on six identical statements in one C#
/// file: the per-pair guard passed the representative pair, the component
/// swallowed every nested window, the same-file overlap collapse reduced
/// it to one occurrence, and a six-occurrence `identical` cluster that
/// both the embeddings-off run and the pre-collapse run report vanished
/// from the report.
fn linking_pair(
    pair: EmbeddingPair,
    indexed: &[IndexedEmbedding],
    footprints: &[Footprint],
) -> Option<EmbeddingPair> {
    if footprints
        .get(pair.left)?
        .touches(footprints.get(pair.right)?)
    {
        return None;
    }
    let left = *indexed.get(pair.left)?.fingerprint_indices.first()?;
    let right = *indexed.get(pair.right)?.fingerprint_indices.first()?;
    Some(EmbeddingPair::ordered(left, right, pair.cosine))
}

/// Every byte range one indexed point occupies, by file, sorted by start.
///
/// Built once per point so the overlap question is answered by a merge
/// over two sorted lists rather than by comparing every owner of one
/// group with every owner of the other — the difference between a linear
/// scan and 810,000 comparisons for a single link on the measured
/// worst-case corpus.
#[derive(Debug)]
struct Footprint {
    /// Occupied ranges per file, each list sorted by start offset.
    by_file: HashMap<FileId, Vec<ByteRange>>,
}

impl Footprint {
    /// Collects the ranges of every fingerprint that owns `item`.
    fn of(item: &IndexedEmbedding, fingerprints: &[Fingerprint]) -> Self {
        let mut by_file: HashMap<FileId, Vec<ByteRange>> = HashMap::new();
        for owner in &item.fingerprint_indices {
            if let Some(fingerprint) = fingerprints.get(*owner) {
                by_file
                    .entry(fingerprint.file_id)
                    .or_default()
                    .push(fingerprint.byte_range);
            }
        }
        for ranges in by_file.values_mut() {
            ranges.sort_unstable_by_key(|range| (range.start, range.end));
        }
        Self { by_file }
    }

    /// True when the two points share any overlapping byte range.
    fn touches(&self, other: &Self) -> bool {
        self.by_file.iter().any(|(file, ranges)| {
            other
                .by_file
                .get(file)
                .is_some_and(|theirs| any_range_intersects(ranges, theirs))
        })
    }
}

/// Merges two start-sorted range lists, reporting the first intersection.
fn any_range_intersects(left: &[ByteRange], right: &[ByteRange]) -> bool {
    let (mut here, mut there) = (0_usize, 0_usize);
    while let (Some(mine), Some(yours)) = (left.get(here), right.get(there)) {
        if mine.end <= yours.start {
            here = here.saturating_add(1);
        } else if yours.end <= mine.start {
            there = there.saturating_add(1);
        } else {
            return true;
        }
    }
    false
}

/// Ties the owners of one collapsed point together, first owner to each
/// of the rest.
///
/// The ANN index can never surface these: a point is not its own
/// neighbour, so once identical snippets collapse to one point their
/// mutual evidence exists only here. It is the strongest evidence the
/// pass holds — one vector compared with itself — and losing it would
/// silence precisely the byte-identical clones, including the pair whose
/// two occurrences normalise to different Merkle hashes and therefore
/// have no structural edge to fall back on.
fn shared_snippet_pairs(
    item: &IndexedEmbedding,
    fingerprints: &[Fingerprint],
) -> Vec<EmbeddingPair> {
    let cosine = cosine_similarity(&item.vector, &item.vector);
    let Some((&canonical, rest)) = item.fingerprint_indices.split_first() else {
        return Vec::new();
    };
    rest.iter()
        .map(|&owner| EmbeddingPair::ordered(canonical, owner, cosine))
        .filter(|pair| ranges_do_not_overlap(pair, fingerprints))
        .collect()
}

/// Returns the source slice for `fingerprint` as a `String`.
pub(super) fn snippet_for(fingerprint: &Fingerprint, sources: &HashMap<FileId, Vec<u8>>) -> String {
    let Some(bytes) = sources.get(&fingerprint.file_id) else {
        return String::new();
    };
    let start = fingerprint.byte_range.start.min(bytes.len());
    let end = fingerprint.byte_range.end.min(bytes.len());
    bytes
        .get(start..end)
        .map(|slice| String::from_utf8_lossy(slice).into_owned())
        .unwrap_or_default()
}

/// Lifts an [`EmbeddingSpec`] and a finished batch into the report-facing
/// provenance struct.
///
/// All three counts come from the one batch so they cannot drift apart,
/// and they are deliberately in two different units. `attempted` and
/// `failed` count subtree *occurrences* — what a reader is owed about
/// coverage, since a rejected snippet costs every occurrence that shared
/// it its embedding signal. `indexed` counts the distinct vectors the ANN
/// index holds after byte-identical snippets collapse onto one point (GH
/// #357). On a corpus with duplicates `indexed` is therefore far below
/// `attempted`, and that gap is the work the pass no longer does — not
/// coverage it lost.
pub(super) fn provenance_from(spec: EmbeddingSpec, batch: &EmbeddingBatch) -> EmbeddingProvenance {
    EmbeddingProvenance {
        provider_id: spec.provider_id,
        model_id: spec.model_id,
        model_version: spec.model_version,
        dimensions: spec.dimensions,
        attempted_subtrees: batch.successes().saturating_add(batch.failures),
        succeeded_subtrees: batch.successes(),
        indexed_subtrees: batch.vectors.len(),
        failed_subtrees: batch.failures,
    }
}

/// Keeps semantic edges from joining nested same-file subtrees into one
/// transitive component. Cross-file pairs and disjoint same-file pairs are
/// valid clone evidence; ancestor/descendant ranges are not.
fn ranges_do_not_overlap(pair: &EmbeddingPair, fingerprints: &[Fingerprint]) -> bool {
    let Some(left) = fingerprints.get(pair.left) else {
        return false;
    };
    let Some(right) = fingerprints.get(pair.right) else {
        return false;
    };
    left.file_id != right.file_id
        || left.byte_range.end <= right.byte_range.start
        || right.byte_range.end <= left.byte_range.start
}
