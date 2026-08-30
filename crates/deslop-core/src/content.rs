//! Content-evidence measurement for shape-identical clone clusters.
//!
//! Implements [FUSED-CONTENT-GATE]: normalisation collapses
//! identifiers and literals, so `structural` and `token_jaccard` agree by
//! construction on any shape match and cannot tell a renamed copy of real
//! logic from mandatory scaffolding or a data table. This pass measures
//! what normalisation erased, in two independent populations:
//!
//! - **agreement** — the fraction of collapsed-leaf positions whose raw
//!   source bytes match on the elected admitted pair, identifiers and
//!   literals pooled. High for verbatim and lightly-edited copies.
//! - **rename consistency** — the Type-2 discriminator
//!   ([TECH-PMATCH-BAKER]): a genuine maximal rename preserves every
//!   literal and maps identifiers through one bijective substitution
//!   whose renamed names *repeat* — Baker's prev-encoding, where a
//!   symbol's first occurrence constrains nothing and repetition carries
//!   the binding proof. Sibling scaffolding changes its literals and
//!   substitutes only its own subject name. Pooling the populations
//!   averaged this proof away and demoted textbook Type-2 clones to
//!   `structural_only`; measured separately, a renamed clone keeps its
//!   supported duplicate bucket.
//!
//! The result is stored on each [`Cluster`] so bucket routing and every
//! report surface can separate real clones from shape coincidence.

use std::collections::HashMap;
use std::hash::BuildHasher;

/// The collapsed-leaf frontier every content measurement reads.
mod frontier;
/// Type-2 rename evidence ([TECH-PMATCH-BAKER], #409).
mod rename;

use frontier::{
    key_set_jaccard, keys_of, member_content, member_count, operators_disagree, population,
    positional_agreement, LeafKey, MemberContent, Population,
};
use rename::ModalBijection;

use crate::{ast::NormalizedNode, cluster::Cluster, fingerprint::Fingerprint, state::FileId};

/// Indexes normalised trees by file for frontier resolution. Shared by
/// every content measurement so one walk site owns the shape.
pub(crate) fn tree_index_of(trees: &[NormalizedNode]) -> HashMap<FileId, &NormalizedNode> {
    trees.iter().map(|tree| (tree.file_id, tree)).collect()
}

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
/// report surfaces ([FUSED-CONTENT-GATE]).
#[derive(Debug, Clone, Copy)]
pub struct ContentEvidence {
    /// Fraction of collapsed-leaf positions whose raw bytes match on the
    /// elected signal pair, identifiers and literals pooled, in `[0, 1]`.
    pub agreement: f64,
    /// Elected-pair Type-2 rename evidence in `[0, 1]`
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
    /// escape hatch), independently of the elected pair's agreement.
    pub verbatim_dominated: bool,
    /// True when the pass actually compared two members' raw content.
    /// `false` for [`Self::unmeasured`] and for a cluster whose members
    /// could not be resolved to source bytes. The other fields carry
    /// deliberately generous defaults so a missing measurement never
    /// demotes a cluster some *other* signal proves; this flag is how a
    /// route with no other signal tells "measured full agreement" apart
    /// from "nothing was measured" ([FUSED-CONTENT-GATE]).
    pub measured: bool,
}

impl ContentEvidence {
    /// Content support for bucket routing: either population may vouch
    /// for a shape-identical cluster — elected-pair byte agreement or a proven
    /// consistent rename. [FUSED-CONTENT-GATE] routes on both, never on
    /// their mean; the mean is what demoted maximal Type-2 renames. The
    /// rule itself lives in [`crate::buckets::content_support`], which
    /// the decision surfaces reading the *rendered* signals share, so
    /// the measured and rendered views cannot drift apart.
    #[must_use]
    pub fn support(self) -> f64 {
        crate::buckets::content_support(self.agreement, self.rename_consistency)
    }

    /// Evidence for a cluster no measurement pass has touched: full
    /// agreement (so nothing is demoted on a missing
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

/// Token identity of one cluster member: its normalised-subtree digest
/// paired with its collapsed-leaf keys. Shape alone pairs different
/// statements that share a frontier (an assignment and an assert over
/// the same name and literal); keys alone pair different shapes over
/// the same vocabulary. A copy has to agree on both.
type TokenIdentity<'keys> = ([u8; 32], &'keys [LeafKey]);

/// One token-identical family's running tally: the index of its
/// earliest member, and how many members have joined it.
type FamilyTally = (usize, usize);

/// The largest token-identical family inside a cluster: one member of
/// it, and how many members it holds.
#[derive(Debug, Clone, Copy)]
struct DominantFamily {
    /// Number of members in the family.
    size: usize,
}

/// Measures and attaches [`ContentEvidence`] for every cluster. Runs
/// once per render, immediately after ranking; cost is one walk per
/// cluster member over already-normalised trees — no re-parsing.
/// `file_languages` selects each member's import/prologue boilerplate
/// exclusion ([PIPELINE-BOILERPLATE-FILTER]), so the frontier measures
/// the same population as every other axis.
pub fn attach_content_evidence<S: BuildHasher, L: BuildHasher>(
    clusters: &mut [Cluster],
    trees: &[NormalizedNode],
    sources: &HashMap<FileId, Vec<u8>, S>,
    file_languages: &HashMap<FileId, &'static str, L>,
) {
    let tree_index = tree_index_of(trees);
    for cluster in clusters.iter_mut() {
        cluster.content = measure_cluster(
            &cluster.members,
            cluster.signal_source,
            &tree_index,
            sources,
            file_languages,
        );
        // [PERF-FLUTTER-TODO-OBSERVABILITY] Per cluster, so `trace` rather
        // than `debug`: a corpus-scale run has to stay readable and stay
        // fast at the level someone reaches for first. The shared-subtree
        // rescue made the case — 793,076 per-item debug records buried the
        // stage events and measurably slowed the stage being diagnosed.
        // Every field below survives at `trace`; the aggregate under the
        // loop is what `debug` sees.
        tracing::trace!(
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
fn measure_cluster<S: BuildHasher, L: BuildHasher>(
    members: &[Fingerprint],
    signal_source: Option<(usize, usize)>,
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
    languages: &HashMap<FileId, &'static str, L>,
) -> ContentEvidence {
    let member_contents: Vec<Option<MemberContent>> = members
        .iter()
        .map(|member| member_content(member, tree_index, sources, languages))
        .collect();
    let canonical = member_contents.first().and_then(Option::as_ref);
    let canonical_keys = keys_of(canonical);
    let dominant = dominant_verbatim_family(&member_contents);
    let verbatim_dominated = member_contents.len() >= 2
        && dominant_verbatim_share(dominant, member_contents.len()) > VERBATIM_MEMBER_SHARE_FLOOR;
    let pair = signal_source.and_then(|(left, right)| {
        Some((
            member_contents.get(left)?.as_ref()?,
            member_contents.get(right)?.as_ref()?,
        ))
    });
    ContentEvidence {
        agreement: pair.map_or(0.0, |(left, right)| {
            pair_agreement(Some(&left.keys), Some(&right.keys))
        }),
        rename_consistency: pair.map_or(0.0, |(left, right)| {
            if operators_disagree(&left.keys, &right.keys) {
                0.0
            } else {
                rename::pair_rename_consistency(Some(left), Some(right), sources)
            }
        }),
        literal_fraction: canonical_literal_fraction(canonical_keys),
        substance_varies: cluster_substance_varies(canonical_keys, &member_contents),
        verbatim_dominated,
        measured: pair.is_some(),
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
    let operators_vary = operators_disagree(canonical, member);
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
    let mut families: HashMap<TokenIdentity<'_>, FamilyTally> = HashMap::new();
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
        .map(|(_, size)| DominantFamily { size })
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
///
/// Both measurements read only *authored* content
/// ([`Population::is_authored_content`]). An operator the members share
/// is already carried by `structural` and `token_jaccard`, so counting
/// it here would let the shape signals vouch for themselves through the
/// gate built to check them; an operator that differs still counts
/// against them in either measurement.
/// [FUSED-CONTENT-GATE] (per-edge, gh #458): one pair's own content
/// agreement, measured from the endpoints' collapsed leaves exactly as
/// a cluster's members are measured — the same measurement, one pair at
/// a time.
///
/// The shared-subtree rescue admits pairs on structural overlap and
/// token corroboration alone; a Merkle-identical signature can carry a
/// pair whose bodies share nothing (the `verbatim-plus-stranger`
/// fixture's stranger measures 0.0436 against a copy while its
/// signature is hash-equal). The cluster-level gate measures only after
/// the component is built — too late to keep the stranger out of the
/// family's act-now evidence — so the rescue consults this per
/// admitted edge and refuses to admit pairs whose own content does not
/// clear the floor.
pub(crate) fn pair_content_agreement<S: BuildHasher, L: BuildHasher>(
    left: &Fingerprint,
    right: &Fingerprint,
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
    languages: &HashMap<FileId, &'static str, L>,
) -> f64 {
    pair_agreement(
        keys_of(member_content(left, tree_index, sources, languages).as_ref()),
        keys_of(member_content(right, tree_index, sources, languages).as_ref()),
    )
}

/// Fraction of aligned collapsed positions whose raw bytes match — the
/// positional branch of the elected pair's content agreement
/// ([FUSED-CONTENT-GATE]); set-Jaccard fallback when the members do not
/// align position for position.
fn pair_agreement(canonical: Option<&[LeafKey]>, member: Option<&[LeafKey]>) -> f64 {
    let (Some(canonical), Some(member)) = (canonical, member) else {
        return 0.0;
    };
    if operators_disagree(canonical, member) {
        return 0.0;
    }
    if canonical.is_empty() && member.is_empty() {
        return 1.0;
    }
    if canonical.len() != member.len() {
        return key_set_jaccard(canonical, member);
    }
    positional_agreement(canonical, member)
}
