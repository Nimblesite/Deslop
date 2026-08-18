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

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::BuildHasher;

use crate::{
    ast::NormalizedNode, cluster::Cluster, fingerprint::Fingerprint, lang::shared::LITERAL_KIND,
    state::FileId, tokens::collapsed_leaves,
};

/// Minimum literal-leaf count before a subtree's literal dominance is
/// reported at all ([CLONE-NOISE-LITERAL-TABLE]). A data table is a run
/// of values; a tiny subtree that happens to be mostly literals (a
/// two-element tuple return, a short argument list) is not a table and
/// must not reach the data-category classifier.
const LITERAL_TABLE_MIN_LITERALS: usize = 8;

/// Minimum occurrences of a substituted identifier pair before it counts
/// as rename evidence ([TECH-PMATCH-BAKER]). In Baker's prev-encoding a
/// parameter symbol's first occurrence matches anything and constrains
/// nothing; only repetition carries binding proof — sibling scaffolding
/// gets one consistent substitution for free from its own subject name.
const RENAME_CORROBORATION_MIN_OCCURRENCES: usize = 2;

/// Half-saturation anchor mass for rename evidence: [`anchor_weight`]
/// scales the rename proof by `anchors / (anchors + this)`, where
/// anchors are every position whose byte-level evidence affirms the
/// copy — preserved literals, identity identifiers, and substitutions
/// corroborated by repetition. Replaces the deleted
/// `RENAME_EVIDENCE_MIN_LITERALS = 4` cliff, which zeroed every
/// sub-floor pair and rendered a maximal one-literal Type-2 rename at
/// `fused = 0.0588` (`deslop/tests/type2_rename_anchor_floor.rs`):
/// scarce anchors now weaken the proof smoothly instead of erasing it,
/// while a forwarding scaffold's echoed subject substitution (its name
/// twice plus its collaborator, mass 3, weight 3/7) stays below every
/// routing floor.
const RENAME_EVIDENCE_HALF_MASS: f64 = 4.0;

/// Minimum share of members that must participate in verbatim
/// duplicates before the guard vouches for the whole cluster. The #104
/// mixed cluster is a verbatim pair among a couple of lookalikes
/// (share ≥ 2/3) and must stay visible; the real-corpus failure mode is
/// the opposite — two byte-identical example widgets hiding inside 453
/// framework-mandated declarations (share ≈ 0.004), where full
/// agreement would resurrect the exact #331 mega-cluster the content
/// gate exists to demote.
const VERBATIM_MEMBER_SHARE_FLOOR: f64 = 0.5;

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
    /// ([TECH-PMATCH-BAKER]): the lesser of literal preservation and
    /// corroborated rename-mapping coverage, scaled by the anchor-mass
    /// weight, so the value carries both how consistent the substitution
    /// is and how much independent evidence backs it. `0.0` when a
    /// member pair lacks positional alignment.
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
    /// True when at least [`VERBATIM_MEMBER_SHARE_FLOOR`] of the members
    /// participate in byte-equivalent leaf-content duplicates. Byte
    /// equality between whole members is proof of copying in its own
    /// right (the #190 verbatim escape hatch), so [`Self::agreement`]
    /// reports full agreement for such a cluster rather than the
    /// positional score the odd-one-out members would dilute.
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

/// One collapsed-leaf content key: the population flag plus a truncated
/// hash of the leaf's raw source bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct LeafKey {
    /// True when the leaf is a literal position, false for identifiers.
    literal: bool,
    /// Truncated blake3 hash of the leaf's raw source bytes.
    key: u64,
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
    let member_keys: Vec<Option<Vec<LeafKey>>> = members
        .iter()
        .map(|member| member_content_keys(member, tree_index, sources))
        .collect();
    let canonical = member_keys.first().and_then(Option::as_deref);
    let verbatim_dominated = member_keys.len() >= 2
        && duplicated_member_share(&member_keys) >= VERBATIM_MEMBER_SHARE_FLOOR;
    ContentEvidence {
        agreement: cluster_agreement(&member_keys, verbatim_dominated),
        rename_consistency: cluster_rename_consistency(canonical, &member_keys),
        literal_fraction: canonical_literal_fraction(canonical),
        substance_varies: cluster_substance_varies(canonical, &member_keys),
        verbatim_dominated,
        // A comparison needs a canonical member *and* something to
        // compare it against: one resolvable member alone measures
        // nothing, and every field above then carries its degenerate
        // default rather than evidence.
        measured: canonical.is_some() && member_keys.iter().skip(1).any(Option::is_some),
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
    member_keys: &[Option<Vec<LeafKey>>],
) -> bool {
    member_keys
        .iter()
        .skip(1)
        .any(|keys| pair_substance_varies(canonical, keys.as_deref()))
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
    let literals = population(canonical, member, true);
    let literals_vary = !literals.is_empty() && literal_preservation(&literals) < 1.0;
    literals_vary || mapping_consistency(&population(canonical, member, false)) < 1.0
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
    let literals = leaves.iter().filter(|leaf| leaf.literal).count();
    if literals < LITERAL_TABLE_MIN_LITERALS || leaves.is_empty() {
        return 0.0;
    }
    member_count(literals) / member_count(leaves.len())
}

/// Mean pooled agreement of every non-canonical member against the
/// canonical (first) member. `1.0` for degenerate single-member
/// clusters; a member whose leaves cannot be resolved contributes `0.0`
/// — unresolvable content is no evidence of agreement.
fn cluster_agreement(member_keys: &[Option<Vec<LeafKey>>], verbatim_dominated: bool) -> f64 {
    if member_keys.len() < 2 || verbatim_dominated {
        return 1.0;
    }
    let canonical = member_keys.first().and_then(Option::as_deref);
    let total: f64 = member_keys
        .iter()
        .skip(1)
        .map(|keys| pair_agreement(canonical, keys.as_deref()))
        .sum();
    total / member_count(member_keys.len().saturating_sub(1))
}

/// Mean Type-2 rename evidence of every non-canonical member against
/// the canonical member. Degenerate and unresolvable members score
/// `0.0`: rename evidence is a positive proof, never a default.
fn cluster_rename_consistency(
    canonical: Option<&[LeafKey]>,
    member_keys: &[Option<Vec<LeafKey>>],
) -> f64 {
    if member_keys.len() < 2 {
        return 0.0;
    }
    let total: f64 = member_keys
        .iter()
        .skip(1)
        .map(|keys| pair_rename_consistency(canonical, keys.as_deref()))
        .sum();
    total / member_count(member_keys.len().saturating_sub(1))
}

/// Type-2 rename evidence between two members ([TECH-PMATCH-BAKER]): the
/// lesser of literal preservation and corroborated rename-mapping
/// coverage, scaled by the smooth anchor-mass weight.
///
/// Baker's prev-encoding is the discriminator the deleted literal-anchor
/// cliff could not provide: a substituted identifier pair seen once is
/// an unconstrained wildcard — sibling scaffolding gets one free from
/// its own subject name — while repeated consistent substitutions and
/// preserved literals are independent anchors of deliberate copying.
/// Scarce anchors weaken the proof smoothly instead of erasing it; the
/// cliff rendered a maximal one-literal Type-2 rename at
/// `fused = 0.0588`, an agent-surface false negative pinned by
/// `deslop/tests/type2_rename_anchor_floor.rs`. `0.0` without
/// positional alignment.
fn pair_rename_consistency(canonical: Option<&[LeafKey]>, member: Option<&[LeafKey]>) -> f64 {
    let (Some(canonical), Some(member)) = (canonical, member) else {
        return 0.0;
    };
    if canonical.len() != member.len() {
        return 0.0;
    }
    let literals = population(canonical, member, true);
    let mapping = rename_mapping(&population(canonical, member, false));
    let anchors = preserved_literal_count(&literals).saturating_add(mapping.explained);
    literal_preservation(&literals).min(mapping.coverage) * anchor_weight(anchors)
}

/// Rename-mapping evidence over one pair's aligned identifier positions
/// ([TECH-PMATCH-BAKER]), produced by [`rename_mapping`].
struct RenameMapping {
    /// Fraction of *constrained* identifier positions explained by
    /// admissible evidence: raw-byte identity (a fixed-symbol match,
    /// witnessed by the position itself), or a substitution that is
    /// bidirectionally modal *among the substituted pairs* and
    /// corroborated by repetition. A consistent substitution seen once
    /// is Baker's unconstrained first occurrence — `prev = 0` matches
    /// any other first occurrence — so it belongs to neither the
    /// numerator nor the denominator: a renamed one-shot declaration
    /// name is not evidence against the clone, and an inconsistent
    /// position still is. Vacuously 1.0 when nothing is constrained —
    /// an all-literal or all-wildcard subtree leaves the anchor weight
    /// to carry the verdict.
    coverage: f64,
    /// Explained positions — the identifier anchors [`anchor_weight`]
    /// prices. Identity positions are backed by byte equality at the
    /// position itself; substituted positions by repetition. Wildcards
    /// are backed by nothing and never count.
    explained: usize,
}

/// Measures [`RenameMapping`] for one pair's identifier positions,
/// classifying each position exactly as [TECH-PMATCH-BAKER]'s
/// prev-encoding constrains it: identity and corroborated substitutions
/// are explained, inconsistent positions are constrained-but-unexplained,
/// and consistent one-shot substitutions are wildcards outside the
/// population.
///
/// The parameter bijection is elected over the *substituted* pairs
/// alone — Baker's fixed symbols and parameters are disjoint alphabets,
/// and collapsed leaves carry no role, so a homonym byte-string (a
/// preserved property name that also names a renamed local) must not
/// let its identity occurrences and its substitution occurrences veto
/// each other in one modal election. Identity needs no election at all:
/// byte equality at the position is its own witness.
fn rename_mapping(identifiers: &[(u64, u64)]) -> RenameMapping {
    let substitutions = substituted_pairs(identifiers);
    let bijection = ModalBijection::over(&substitutions);
    let counts = pair_counts(substitutions.iter().copied());
    let (mut constrained, mut explained) = (0_usize, 0_usize);
    for pair in identifiers {
        let substituted = pair.0 != pair.1;
        let corroborated =
            counts.get(pair).copied().unwrap_or_default() >= RENAME_CORROBORATION_MIN_OCCURRENCES;
        if substituted && bijection.explains(pair) && !corroborated {
            continue;
        }
        constrained = constrained.saturating_add(1);
        if !substituted || bijection.explains(pair) {
            explained = explained.saturating_add(1);
        }
    }
    RenameMapping {
        coverage: vacuous_share(explained, constrained),
        explained,
    }
}

/// The aligned positions whose raw bytes differ — [TECH-PMATCH-BAKER]'s
/// parameter alphabet, the population [`rename_mapping`] elects its
/// bijection over.
fn substituted_pairs(identifiers: &[(u64, u64)]) -> Vec<(u64, u64)> {
    identifiers
        .iter()
        .filter(|(left, right)| left != right)
        .copied()
        .collect()
}

/// Smooth evidence-mass weight for rename proof:
/// `anchors / (anchors + RENAME_EVIDENCE_HALF_MASS)`. Zero anchors weigh
/// vacuous evidence to zero and accumulating independent anchors
/// approach full weight; a cliff here is what manufactured the
/// quarantined false negative.
fn anchor_weight(anchors: usize) -> f64 {
    let mass = member_count(anchors);
    mass / (mass + RENAME_EVIDENCE_HALF_MASS)
}

/// Paired keys at the positions where both members carry the requested
/// population (literal or identifier). Shape-aligned members disagree on
/// a position's population only at parse-artifact boundaries; such
/// positions belong to neither population.
fn population(canonical: &[LeafKey], member: &[LeafKey], literal: bool) -> Vec<(u64, u64)> {
    canonical
        .iter()
        .zip(member.iter())
        .filter(|(left, right)| left.literal == literal && right.literal == literal)
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

/// The bidirectionally-modal substitution test shared by the substance
/// and rename measures: a position is explained when its pair is the
/// modal partner in both directions. A genuine rename maps every
/// occurrence of a name to one new name; scattergun similarity does
/// not. The caller chooses the electorate: [`mapping_consistency`]
/// elects over every identifier position (identity included), while
/// [`rename_mapping`] elects over the substituted pairs alone.
struct ModalBijection {
    /// Modal partner of each left key.
    forward: BTreeMap<u64, u64>,
    /// Modal partner of each right key.
    backward: BTreeMap<u64, u64>,
}

impl ModalBijection {
    /// Builds the two modal maps over one pair's identifier positions.
    fn over(identifiers: &[(u64, u64)]) -> Self {
        Self {
            forward: modal_partners(identifiers.iter().map(|(left, right)| (*left, *right))),
            backward: modal_partners(identifiers.iter().map(|(left, right)| (*right, *left))),
        }
    }

    /// True when the pair is the modal partner in both directions.
    fn explains(&self, (left, right): &(u64, u64)) -> bool {
        self.forward.get(left) == Some(right) && self.backward.get(right) == Some(left)
    }
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

/// Occurrence count per aligned key pair — the corroboration ledger for
/// [`rename_mapping`] and the tally [`modal_partners`] folds.
fn pair_counts(pairs: impl Iterator<Item = (u64, u64)>) -> BTreeMap<(u64, u64), usize> {
    let mut counts: BTreeMap<(u64, u64), usize> = BTreeMap::new();
    for pair in pairs {
        let slot = counts.entry(pair).or_insert(0_usize);
        *slot = slot.saturating_add(1);
    }
    counts
}

/// Modal partner per key: the partner seen most often. Counting and
/// folding run over [`BTreeMap`]s in ascending order and replacement
/// requires a strictly greater count, so ties resolve to the smallest
/// partner key and the map is deterministic across runs.
fn modal_partners(pairs: impl Iterator<Item = (u64, u64)>) -> BTreeMap<u64, u64> {
    let mut modes: BTreeMap<u64, (u64, usize)> = BTreeMap::new();
    for ((key, partner), count) in pair_counts(pairs) {
        let best = modes.entry(key).or_insert((partner, count));
        if count > best.1 {
            *best = (partner, count);
        }
    }
    modes
        .into_iter()
        .map(|(key, (partner, _))| (key, partner))
        .collect()
}

/// Share of members whose non-empty content-key vector also appears on
/// another member — verbatim copies hiding among same-shape lookalikes,
/// the population the renderer's body-equivalence guard relies on.
/// Transitive closure can merge a
/// genuine byte-identical pair into a cluster of same-shape neighbours;
/// the mean against one canonical member would average the proven copy
/// below the support floor, so a cluster *dominated* by verbatim copies
/// short-circuits to full agreement instead.
fn duplicated_member_share(member_keys: &[Option<Vec<LeafKey>>]) -> f64 {
    let mut counts: HashMap<&[LeafKey], usize> = HashMap::new();
    for keys in member_keys.iter().flatten() {
        if !keys.is_empty() {
            let entry = counts.entry(keys.as_slice()).or_insert(0_usize);
            *entry = entry.saturating_add(1);
        }
    }
    let duplicated: usize = counts.values().filter(|count| **count >= 2).copied().sum();
    if member_keys.is_empty() {
        return 0.0;
    }
    member_count(duplicated) / member_count(member_keys.len())
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

/// One content key per collapsed leaf: the population flag plus a
/// truncated blake3 hash of the leaf's raw source bytes. `None` when the
/// member's tree, source, or byte range cannot be resolved.
fn member_content_keys<S: BuildHasher>(
    member: &Fingerprint,
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> Option<Vec<LeafKey>> {
    let root = tree_index.get(&member.file_id)?;
    let source = sources.get(&member.file_id)?;
    let leaves = collapsed_leaves(root, member)?;
    leaves
        .iter()
        .map(|(kind, range)| {
            source.get(range.start..range.end).map(|bytes| LeafKey {
                literal: *kind == LITERAL_KIND,
                key: truncated_hash(bytes),
            })
        })
        .collect()
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
