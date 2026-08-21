//! Content-evidence measurement for shape-identical clone clusters.
//!
//! Implements [FUSION-CONTENT-GATE]: normalisation collapses
//! identifiers and literals, so `structural` and `token_jaccard` agree by
//! construction on any shape match and cannot tell a renamed copy of real
//! logic from mandatory scaffolding or a data table. This pass measures
//! what normalisation erased, in two independent populations:
//!
//! - **agreement** — the fraction of collapsed-leaf positions whose raw
//!   source bytes still match across members, identifiers and literals
//!   pooled. High for verbatim and lightly-edited copies.
//! - **rename consistency** — the Type-2 discriminator
//!   ([TECH-PMATCH-BAKER]): a genuine maximal rename preserves every
//!   literal and maps identifiers through one bijective substitution
//!   whose renamed names *repeat* — Baker's prev-encoding, where a
//!   symbol's first occurrence constrains nothing and repetition carries
//!   the binding proof. Sibling scaffolding changes its literals and
//!   substitutes only its own subject name. Pooling the populations
//!   averaged this proof away and demoted textbook Type-2 clones to
//!   `structural_only`; measured separately, a renamed clone keeps its
//!   act-now verdict.
//!
//! The result is stored on each [`Cluster`] so bucket routing, the
//! rendered fused confidence, and the ranking weight can separate real
//! clones from shape coincidence.

use std::collections::{BTreeSet, HashMap};
use std::hash::BuildHasher;

/// Type-2 rename evidence ([TECH-PMATCH-BAKER], #409).
mod rename;

use rename::ModalBijection;

use crate::{
    ast::{ByteRange, NormalizedNode},
    cluster::Cluster,
    fingerprint::Fingerprint,
    lang::shared::{LITERAL_KIND, OPERATOR_KIND},
    state::FileId,
    tokens::collapsed_leaves,
};

/// Minimum literal-leaf count before a subtree's literal dominance is
/// reported at all ([CLONE-NOISE-LITERAL-TABLE]). A data table is a run
/// of values; a tiny subtree that happens to be mostly literals (a
/// two-element tuple return, a short argument list) is not a table and
/// must not reach the data-category classifier.
const LITERAL_TABLE_MIN_LITERALS: usize = 8;

/// Exclusive share a single token-identical family must exceed — a
/// strict majority of the members — before the guard vouches for the
/// whole cluster. The #104 mixed cluster is a verbatim pair among a
/// couple of lookalikes (share ≥ 2/3) and must stay visible; two
/// disjoint verbatim pairs splitting a four-member cluster (share
/// exactly 1/2 each, `python-dict-assert-call-in-payload`) are two
/// separate duplications and dominate nothing; and the real-corpus
/// failure mode is two byte-identical example widgets hiding inside 453
/// framework-mandated declarations (share ≈ 0.004), where full
/// agreement would resurrect the exact #331 mega-cluster the content
/// gate exists to demote.
const VERBATIM_MEMBER_SHARE_FLOOR: f64 = 0.5;

/// Smallest token-identical family that is a copy of anything.
const MIN_VERBATIM_FAMILY: usize = 2;

/// Measured raw-content evidence for one cluster, produced by
/// [`attach_content_evidence`] and consumed by bucket routing, the
/// rendered fused confidence, and the ranking weight
/// ([FUSION-CONTENT-GATE]).
#[derive(Debug, Clone, Copy)]
pub struct ContentEvidence {
    /// Mean fraction of collapsed-leaf positions whose raw bytes match
    /// the canonical member, identifiers and literals pooled, in `[0, 1]`.
    pub agreement: f64,
    /// Mean Type-2 rename evidence in `[0, 1]`
    /// ([TECH-PMATCH-BAKER]): the lesser of literal consistency (a
    /// literal preserved, or echoing an elected identifier substitution
    /// — renamed alongside its symbol, #409) and corroborated
    /// rename-mapping coverage, scaled by the anchor-mass weight, so
    /// the value carries both how consistent the substitution is and
    /// how much independent evidence backs it. `0.0` when a member pair
    /// lacks positional alignment.
    pub rename_consistency: f64,
    /// Fraction of the canonical member's collapsed leaves that are
    /// literal positions ([CLONE-NOISE-LITERAL-TABLE]).
    pub literal_fraction: f64,
    /// Positive proof that the members differ in *substance* rather than
    /// in bound names ([RANK-STRUCTURAL-ONLY]): some aligned literal
    /// position carries different bytes, or the identifier substitution
    /// needs more than one consistent 1:1 mapping. `false` when the
    /// members duplicate substance **and** when nothing could be
    /// measured — a finding of scaffolding is positive evidence, never an
    /// absent measurement.
    pub substance_varies: bool,
    /// True when one token-identical family — members equal in both
    /// normalised subtree shape and every collapsed leaf's raw bytes —
    /// holds a strict majority of the members (more than
    /// [`VERBATIM_MEMBER_SHARE_FLOOR`]). Token equality between whole
    /// members is proof of copying in its own right (the #190 verbatim
    /// escape hatch), so [`Self::agreement`] reports full agreement for
    /// such a cluster rather than the positional score the odd-one-out
    /// members would dilute.
    pub verbatim_dominated: bool,
    /// True when the pass actually compared two members' raw content.
    /// `false` for [`Self::unmeasured`] and for a cluster whose members
    /// could not be resolved to source bytes. The other fields carry
    /// deliberately generous defaults so a missing measurement never
    /// demotes a cluster some *other* signal proves; this flag is how a
    /// route with no other signal tells "measured full agreement" apart
    /// from "nothing was measured" ([FUSION-CONTENT-GATE]).
    pub measured: bool,
}

impl ContentEvidence {
    /// Content support for bucket routing: either population may vouch
    /// for a shape-identical cluster — pooled byte agreement or a proven
    /// consistent rename. [FUSION-CONTENT-GATE] routes on both, never on
    /// their mean; the mean is what demoted maximal Type-2 renames. The
    /// rule itself lives in [`crate::buckets::content_support`], which
    /// the decision surfaces reading the *rendered* signals share, so
    /// the measured and rendered views cannot drift apart.
    #[must_use]
    pub fn support(self) -> f64 {
        crate::buckets::content_support(self.agreement, self.rename_consistency)
    }

    /// Evidence for a cluster no measurement pass has touched: full
    /// pooled agreement (so nothing is demoted on a missing
    /// measurement), no rename proof, no literal dominance.
    #[must_use]
    pub const fn unmeasured() -> Self {
        Self {
            agreement: 1.0,
            rename_consistency: 0.0,
            literal_fraction: 0.0,
            substance_varies: false,
            verbatim_dominated: false,
            measured: false,
        }
    }
}

/// The largest token-identical family inside a cluster: one member of
/// it, and how many members it holds.
#[derive(Debug, Clone, Copy)]
struct DominantFamily {
    /// Index of the family's earliest member — the anchor every other
    /// member's agreement is measured against.
    anchor: usize,
    /// Number of members in the family.
    size: usize,
}

/// Which evidence population a collapsed frontier leaf belongs to.
///
/// Identifiers and literals are the two populations the rename and
/// literal-preservation measurements are defined over. Operators are a
/// third ([PIPELINE-NORMALIZE-AST-OPERATOR]) and deliberately belong to
/// neither: there is no substitution that turns `+` into `-`, so
/// counting an operator as an identifier would report a broken rename,
/// and counting it as a literal would report a data table. It is
/// evidence of its own kind — positional agreement, and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Population {
    /// A collapsed identifier position.
    Identifier,
    /// A collapsed literal position.
    Literal,
    /// A behaviour-bearing operator token.
    Operator,
}

impl Population {
    /// The population a normalised leaf kind belongs to. Anything that
    /// is neither a literal nor an operator reached the frontier as a
    /// collapsed identifier.
    fn of(kind: &str) -> Self {
        match kind {
            LITERAL_KIND => Self::Literal,
            OPERATOR_KIND => Self::Operator,
            _ => Self::Identifier,
        }
    }
}

/// One collapsed-leaf content key: the population flag plus a truncated
/// hash of the leaf's raw source bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct LeafKey {
    /// Which evidence population the leaf belongs to.
    population: Population,
    /// Truncated blake3 hash of the leaf's raw source bytes.
    key: u64,
}

/// One member's resolved content frontier: the per-leaf keys plus the
/// byte range each key hashed, so rename measurement can read a leaf's
/// raw bytes back without re-walking the tree
/// ([`rename::literal_echoes`], #409).
struct MemberContent {
    /// File every range below indexes into.
    file: FileId,
    /// Normalised-subtree digest of the member ([`Fingerprint::hash`])
    /// — the shape half of token identity in
    /// [`dominant_verbatim_share`].
    shape: [u8; 32],
    /// One key per collapsed leaf, in frontier order.
    keys: Vec<LeafKey>,
    /// The source byte range each key was hashed from, 1:1 with `keys`.
    ranges: Vec<ByteRange>,
}

/// The key slice of a resolved member, `None` when unresolvable.
fn keys_of(content: Option<&MemberContent>) -> Option<&[LeafKey]> {
    content.map(|content| content.keys.as_slice())
}

/// Raw source bytes of one collapsed leaf, by frontier index.
fn leaf_bytes<'src, S: BuildHasher>(
    content: &MemberContent,
    index: usize,
    sources: &'src HashMap<FileId, Vec<u8>, S>,
) -> Option<&'src [u8]> {
    let range = content.ranges.get(index)?;
    sources.get(&content.file)?.get(range.start..range.end)
}

/// Measures and attaches [`ContentEvidence`] for every cluster. Runs
/// once per render, immediately after ranking; cost is one walk per
/// cluster member over already-normalised trees — no re-parsing.
pub fn attach_content_evidence<S: BuildHasher>(
    clusters: &mut [Cluster],
    trees: &[NormalizedNode],
    sources: &HashMap<FileId, Vec<u8>, S>,
) {
    let tree_index: HashMap<FileId, &NormalizedNode> =
        trees.iter().map(|tree| (tree.file_id, tree)).collect();
    for cluster in clusters.iter_mut() {
        cluster.content = measure_cluster(&cluster.members, &tree_index, sources);
        tracing::debug!(
            cluster_id = %cluster.id,
            member_count = cluster.members.len(),
            agreement = cluster.content.agreement,
            rename_consistency = cluster.content.rename_consistency,
            literal_fraction = cluster.content.literal_fraction,
            substance_varies = cluster.content.substance_varies,
            verbatim_dominated = cluster.content.verbatim_dominated,
            "cluster content evidence"
        );
    }
    tracing::debug!(cluster_count = clusters.len(), "content evidence attached");
}

/// Measures one cluster's [`ContentEvidence`] from its members'
/// collapsed leaves, resolving each member's content keys exactly once.
fn measure_cluster<S: BuildHasher>(
    members: &[Fingerprint],
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> ContentEvidence {
    let member_contents: Vec<Option<MemberContent>> = members
        .iter()
        .map(|member| member_content(member, tree_index, sources))
        .collect();
    let canonical = member_contents.first().and_then(Option::as_ref);
    let canonical_keys = keys_of(canonical);
    let dominant = dominant_verbatim_family(&member_contents);
    let verbatim_dominated = member_contents.len() >= 2
        && dominant_verbatim_share(dominant, member_contents.len()) > VERBATIM_MEMBER_SHARE_FLOOR;
    ContentEvidence {
        agreement: cluster_agreement(&member_contents, dominant),
        rename_consistency: rename::cluster_rename_consistency(
            canonical,
            &member_contents,
            sources,
        ),
        literal_fraction: canonical_literal_fraction(canonical_keys),
        substance_varies: cluster_substance_varies(canonical_keys, &member_contents),
        verbatim_dominated,
        // A comparison needs a canonical member *and* something to
        // compare it against: one resolvable member alone measures
        // nothing, and every field above then carries its degenerate
        // default rather than evidence.
        measured: canonical.is_some() && member_contents.iter().skip(1).any(Option::is_some),
    }
}

/// Proof that a cluster's members differ in substance rather than in
/// bound names ([RANK-STRUCTURAL-ONLY]). One member that provably varies
/// convicts the cluster: a sibling-declaration family only has to change
/// one endpoint literal to stop being a copy of its neighbour.
/// Degenerate and unresolvable clusters return `false` — nothing was
/// measured, so nothing is proven.
fn cluster_substance_varies(
    canonical: Option<&[LeafKey]>,
    member_contents: &[Option<MemberContent>],
) -> bool {
    member_contents
        .iter()
        .skip(1)
        .any(|content| pair_substance_varies(canonical, keys_of(content.as_ref())))
}

/// Proof that two members differ in substance: their aligned leaves
/// disagree on a literal, or their identifiers need more than one
/// consistent substitution.
///
/// Deliberately carries no literal-anchor floor.
/// [`RENAME_EVIDENCE_MIN_LITERALS`] guards a *score* against agreement by
/// coincidence — few anchors make matching literals weak evidence *for* a
/// rename. A *disagreement* needs no such floor: differing bytes at an
/// anchored position are evidence on their own, and a two-literal body
/// under a maximal rename is still a clone.
fn pair_substance_varies(canonical: Option<&[LeafKey]>, member: Option<&[LeafKey]>) -> bool {
    let (Some(canonical), Some(member)) = (canonical, member) else {
        return false;
    };
    if canonical.len() != member.len() {
        return true;
    }
    let literals = population(canonical, member, Population::Literal);
    let literals_vary = !literals.is_empty() && literal_preservation(&literals) < 1.0;
    // [PIPELINE-NORMALIZE-AST-OPERATOR] An operator that changed is
    // substance that changed. There is no substitution that explains
    // `+` becoming `-`: it is a different computation, not a different
    // name for the same one.
    let operators = population(canonical, member, Population::Operator);
    let operators_vary = operators.iter().any(|(left, right)| left != right);
    literals_vary
        || operators_vary
        || mapping_consistency(&population(canonical, member, Population::Identifier)) < 1.0
}

/// Fraction of the canonical member's collapsed leaves that are literal
/// positions, in `[0, 1]` — the language-agnostic "is this a data
/// literal?" measurement ([CLONE-NOISE-LITERAL-TABLE]). `0.0` when the
/// member cannot be resolved or carries fewer than
/// [`LITERAL_TABLE_MIN_LITERALS`] literals, so tiny literal-heavy
/// subtrees never register as tables.
fn canonical_literal_fraction(canonical: Option<&[LeafKey]>) -> f64 {
    let Some(leaves) = canonical else {
        return 0.0;
    };
    let literals = leaves
        .iter()
        .filter(|leaf| leaf.population == Population::Literal)
        .count();
    // Operators are neither data nor names, so they belong to neither
    // side of "is this a data literal?" and are left out of the
    // denominator — a table stays as literal-dominated as it was before
    // operators joined the frontier ([CLONE-NOISE-LITERAL-TABLE]).
    let vocabulary = leaves
        .iter()
        .filter(|leaf| leaf.population != Population::Operator)
        .count();
    if literals < LITERAL_TABLE_MIN_LITERALS || vocabulary == 0 {
        return 0.0;
    }
    member_count(literals) / member_count(vocabulary)
}

/// Mean pooled agreement of every other member against one anchor.
/// `1.0` for degenerate single-member clusters; a member whose leaves
/// cannot be resolved contributes `0.0` — unresolvable content is no
/// evidence of agreement.
///
/// The anchor is a member of the largest token-identical family when
/// there is one, and the first member otherwise. That choice is the
/// whole of what the old `verbatim_dominated` short-circuit was for:
/// measuring a cluster of proven copies against a canonical that
/// happens *not* to be one of them averaged the copies down below the
/// support floor, so the measurement was replaced wholesale by `1.0`.
///
/// Replacing it was too much. A strict majority is not everyone: a
/// five-member cluster of three exact copies plus two shape-compatible
/// strangers cleared the floor at 3/5 and every member — strangers
/// included — was then handed the proof the three copies had earned,
/// saturating `fused` for code that is not duplicated. Anchoring
/// instead of short-circuiting keeps the copies at `1.0` where they
/// belong and leaves each stranger scoring its own real agreement, so
/// the cluster's number describes the cluster.
fn cluster_agreement(
    member_contents: &[Option<MemberContent>],
    dominant: Option<DominantFamily>,
) -> f64 {
    if member_contents.len() < 2 {
        return 1.0;
    }
    let anchor = dominant.map_or(0, |family| family.anchor);
    let canonical = keys_of(member_contents.get(anchor).and_then(Option::as_ref));
    let total: f64 = member_contents
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != anchor)
        .map(|(_, content)| pair_agreement(canonical, keys_of(content.as_ref())))
        .sum();
    total / member_count(member_contents.len().saturating_sub(1))
}

/// Paired keys at the positions where both members carry `wanted`.
/// Shape-aligned members disagree on a position's population only at
/// parse-artifact boundaries; such positions belong to no population.
fn population(canonical: &[LeafKey], member: &[LeafKey], wanted: Population) -> Vec<(u64, u64)> {
    canonical
        .iter()
        .zip(member.iter())
        .filter(|(left, right)| left.population == wanted && right.population == wanted)
        .map(|(left, right)| (left.key, right.key))
        .collect()
}

/// Aligned literal positions whose raw bytes match — each one an
/// independent anchor priced by [`anchor_weight`].
fn preserved_literal_count(literals: &[(u64, u64)]) -> usize {
    literals
        .iter()
        .filter(|(left, right)| left == right)
        .count()
}

/// Fraction of literal positions whose raw bytes match — a Type-2 clone
/// preserves its literals; a sibling scaffold or a data table does not.
/// Vacuously `1.0` with no literal positions: absent literals prove
/// nothing either way, and [`anchor_weight`] already prices the missing
/// mass, so the mapping term carries the whole proof.
fn literal_preservation(literals: &[(u64, u64)]) -> f64 {
    vacuous_share(preserved_literal_count(literals), literals.len())
}

/// Share of identifier positions explained by one consistent 1:1
/// substitution ([`ModalBijection`], identity included). Vacuously
/// `1.0` with no identifier positions — an all-literal subtree leaves
/// nothing to substitute. This is the *substance* notion of consistency
/// ([`pair_substance_varies`]): corroboration is deliberately not
/// required here, because an uncorroborated-but-consistent substitution
/// is no proof that the members differ.
fn mapping_consistency(identifiers: &[(u64, u64)]) -> f64 {
    let bijection = ModalBijection::over(identifiers);
    let explained = identifiers
        .iter()
        .filter(|pair| bijection.explains(pair))
        .count();
    vacuous_share(explained, identifiers.len())
}

/// `numerator / denominator`, vacuously `1.0` over an empty denominator
/// — an empty evidence population proves nothing either way, and
/// [`anchor_weight`] prices the absent mass.
fn vacuous_share(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        return 1.0;
    }
    member_count(numerator) / member_count(denominator)
}

/// Share of members inside the single largest token-identical family —
/// members equal in both normalised subtree shape and every collapsed
/// leaf's raw bytes, i.e. copies of one another up to whitespace.
/// Transitive closure can merge such a family into a cluster of
/// same-shape neighbours; the mean against one canonical member would
/// average the proven copies below the support floor, so a cluster
/// *dominated* by one verbatim family short-circuits to full agreement
/// instead.
///
/// One family, not a pool: two disjoint verbatim pairs are two separate
/// duplications, and summing them certified a four-member cluster whose
/// halves disagree as verbatim (`python-dict-assert-call-in-payload`).
/// Shape is half the identity: an assignment and an assert over the
/// same identifier and literal carry equal leaf keys while being
/// different statements, so leaf keys alone pair non-copies
/// (`python-issue-72-monkeypatch-setenv`).
fn dominant_verbatim_family(member_contents: &[Option<MemberContent>]) -> Option<DominantFamily> {
    let mut families: HashMap<([u8; 32], &[LeafKey]), (usize, usize)> = HashMap::new();
    for (index, content) in member_contents.iter().enumerate() {
        let Some(content) = content else { continue };
        if content.keys.is_empty() {
            continue;
        }
        let entry = families
            .entry((content.shape, content.keys.as_slice()))
            .or_insert((index, 0_usize));
        entry.1 = entry.1.saturating_add(1);
    }
    // Largest family wins; the earliest member breaks a tie so the
    // choice is independent of hash iteration order
    // ([PIPELINE-DETERMINISM]).
    families
        .into_values()
        .filter(|(_, size)| *size >= MIN_VERBATIM_FAMILY)
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
        .map(|(anchor, size)| DominantFamily { anchor, size })
}

/// The share of a cluster held by its largest token-identical family.
fn dominant_verbatim_share(dominant: Option<DominantFamily>, members: usize) -> f64 {
    let Some(family) = dominant else {
        return 0.0;
    };
    if members == 0 {
        return 0.0;
    }
    member_count(family.size) / member_count(members)
}

/// Agreement between two members' collapsed-leaf content keys.
/// Shape-identical members carry equal-length key vectors and score the
/// positional match fraction. Shape-*mismatched* members (an LSH-paired
/// near-miss whose trees differ) have no positional alignment, so they
/// score the key-set Jaccard instead — a genuine Type-3 near-miss
/// shares nearly all its keys, while renamed scaffolding shares few.
/// Unresolvable members score `0.0`; two empty vectors agree fully — a
/// subtree with no identifiers or literals has nothing to disagree on.
fn pair_agreement(canonical: Option<&[LeafKey]>, member: Option<&[LeafKey]>) -> f64 {
    let (Some(canonical), Some(member)) = (canonical, member) else {
        return 0.0;
    };
    if canonical.is_empty() && member.is_empty() {
        return 1.0;
    }
    if canonical.len() != member.len() {
        return key_set_jaccard(canonical, member);
    }
    let equal = canonical
        .iter()
        .zip(member.iter())
        .filter(|(left, right)| left == right)
        .count();
    member_count(equal) / member_count(canonical.len())
}

/// Jaccard similarity of two members' content-key sets.
fn key_set_jaccard(left: &[LeafKey], right: &[LeafKey]) -> f64 {
    let left: BTreeSet<LeafKey> = left.iter().copied().collect();
    let right: BTreeSet<LeafKey> = right.iter().copied().collect();
    let intersection = left.intersection(&right).count();
    let union = left.union(&right).count();
    if union == 0 {
        return 1.0;
    }
    member_count(intersection) / member_count(union)
}

/// One member's resolved content frontier: a key and its source range
/// per collapsed leaf. `None` when the member's tree, source, or byte
/// range cannot be resolved.
fn member_content<S: BuildHasher>(
    member: &Fingerprint,
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> Option<MemberContent> {
    let root = tree_index.get(&member.file_id)?;
    let source = sources.get(&member.file_id)?;
    let leaves = collapsed_leaves(root, member)?;
    let keys = leaves
        .iter()
        .map(|(kind, range)| {
            source.get(range.start..range.end).map(|bytes| LeafKey {
                population: Population::of(kind),
                key: truncated_hash(bytes),
            })
        })
        .collect::<Option<Vec<LeafKey>>>()?;
    let ranges = leaves.iter().map(|(_, range)| *range).collect();
    Some(MemberContent {
        file: member.file_id,
        shape: member.hash,
        keys,
        ranges,
    })
}

/// First eight little-endian bytes of the blake3 hash of `bytes`.
fn truncated_hash(bytes: &[u8]) -> u64 {
    let digest = blake3::hash(bytes);
    let mut prefix = [0_u8; 8];
    for (slot, byte) in prefix.iter_mut().zip(digest.as_bytes().iter()) {
        *slot = *byte;
    }
    u64::from_le_bytes(prefix)
}

/// Lossless small-count conversion for agreement divisors.
fn member_count(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}
