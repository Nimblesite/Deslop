//! Type-2 rename evidence ([TECH-PMATCH-BAKER], [FUSED-CONTENT-GATE],
//! [REPAIR-RENAME-LITERAL-ECHO]).
//!
//! One product per member pair:
//! `pooled_coverage * anchor_weight`, where the literal positions in
//! the pool are gh #409's subject and the anchor mass is gh #410's. A literal renamed *alongside the symbol it names* is part
//! of the rename, not evidence against it: `"OrderService"` renamed to
//! `"UserService"` with the `OrderService` symbol is the rename done
//! *thoroughly*, and counting it as a differing literal inverted the
//! score — the half-finished rename outscored the complete one
//! (`crates/deslop/tests/rename_literal_monotonicity.rs`). Such an
//! **echo** is recognised by content, never by coincidence: the
//! literal's bytes must transform into the partner's bytes exactly by
//! the same substitution the identifier bijection explains, and the echo
//! then corroborates that substitution the way a repeated identifier
//! occurrence would.

use std::{collections::BTreeMap, collections::HashMap, hash::BuildHasher};

use crate::{buckets::CONTENT_SUPPORT_FLOOR, content::PairScope, state::FileId};

use super::{
    frontier::{frontiers_aligned, member_count, population, MemberContent, Population},
    vacuous_share,
};

use literal_echo::{affirming_literal_count, literal_echoes, LiteralEchoes};

/// Literal echoes of a rename, and the byte transform that proves one.
mod literal_echo;

/// Minimum occurrences of a substituted identifier pair before it counts
/// as rename evidence ([TECH-PMATCH-BAKER]). In Baker's prev-encoding a
/// parameter symbol's first occurrence matches anything and constrains
/// nothing; only repetition carries binding proof — sibling scaffolding
/// gets one consistent substitution for free from its own subject name.
/// A literal echo of the substitution counts toward this bar (#409): the
/// echo is a second independent position witnessing the same mapping.
const RENAME_CORROBORATION_MIN_OCCURRENCES: usize = 2;

/// Half-saturation anchor mass for rename evidence: [`anchor_weight`]
/// scales the rename proof by `anchors / (anchors + this)`, where
/// anchors are every position whose byte-level evidence affirms the
/// copy — preserved literals, echoed literals, identity identifiers,
/// and substitutions corroborated by repetition. Replaces the deleted
/// `RENAME_EVIDENCE_MIN_LITERALS = 4` cliff, which zeroed every
/// sub-floor pair and priced a maximal one-literal Type-2 rename at
/// `0.0588` (`deslop/tests/type2_rename_anchor_floor.rs`):
/// scarce anchors now weaken the proof smoothly instead of erasing it,
/// while a forwarding scaffold's echoed subject substitution (its name
/// twice plus its collaborator, mass 3, weight 3/7) stays below every
/// routing floor.
const RENAME_EVIDENCE_HALF_MASS: f64 = 4.0;

/// Half-saturation anchor mass for a pair of **whole authored
/// declarations** ([REPAIR-RENAME-ANCHOR-MASS]).
///
/// The mass term prices coincidence: scarce affirming positions might be
/// two windows that happen to line up. Two whole declarations are not a
/// window alignment — the author wrote both of them, opening brace to
/// closing brace — so the coincidence the discount is pricing is
/// weaker, exactly as [FUSED-CONTENT-GATE-INTERIOR] finds it *stronger*
/// for a window carved out of one function.
///
/// It is not an escape hatch: a one-line REST wrapper is a whole
/// declaration too, and `dart-forwarding-duplicate-route`'s five
/// distinct-route wrappers affirm five positions each, weigh `5/8` and
/// stay refused, while `Billing`'s two-statement methods affirm nine,
/// weigh `9/12` and certify. The separation is how much authored code
/// the two declarations prove identical, which is the quantity this
/// term has always measured.
const AUTHORED_RENAME_EVIDENCE_HALF_MASS: f64 = 3.0;

/// Type-2 rename evidence between two members ([TECH-PMATCH-BAKER]): one
/// pooled coverage over the pair's constrained identifier positions and
/// every aligned literal position, scaled by the smooth anchor-mass
/// weight. On a cross-file pair a drifted literal that echoes nothing
/// stays in the denominator, weakening the proof in proportion to the
/// evidence around it instead of vetoing an otherwise fully-anchored
/// rename. A same-file pair keeps the stricter min of the
/// literal-affirmation share and identifier coverage, matching the
/// promote floor's conservatism: a same-file rename family is the #197
/// sibling shape, and its literal axis must vouch on its own. The mass
/// term the coverage is scaled by is scope-aware too
/// ([REPAIR-RENAME-ANCHOR-MASS]).
/// The pool opens only where the literal population affirms at all:
/// constrained literals with zero preservation and zero echoes are the
/// #134 stride family — every substantive byte disagrees and nothing
/// outside the substitution vouches, so the axis is `0.0`. Which
/// literals are constrained is [`LiteralEvidence::measure`]'s call
/// ([FUSED-CONTENT-GATE-PARAMETER]).
///
/// Baker's prev-encoding is the discriminator the deleted literal-anchor
/// cliff could not provide: a substituted identifier pair seen once is
/// an unconstrained wildcard — sibling scaffolding gets one free from
/// its own subject name — while repeated consistent substitutions and
/// preserved literals are independent anchors of deliberate copying.
/// Scarce anchors weaken the proof smoothly instead of erasing it; the
/// cliff priced a maximal one-literal Type-2 rename at
/// `0.0588`, an agent-surface false negative pinned by
/// `deslop/tests/type2_rename_anchor_floor.rs`. `0.0` without
/// positional alignment.
///
/// Literal consistency counts a preserved literal *and* a literal echo
/// of a bijection-explained substitution as affirming positions (#409),
/// and each echo raises the anchor mass and corroborates its
/// substitution, so completing a rename can never score below leaving
/// it half-finished (`rename_literal_monotonicity.rs`).
pub(super) fn pair_rename_consistency<S: BuildHasher>(
    canonical: Option<&MemberContent>,
    member: Option<&MemberContent>,
    sources: &HashMap<FileId, Vec<u8>, S>,
    scope: PairScope,
) -> f64 {
    let (Some(canonical), Some(member)) = (canonical, member) else {
        return 0.0;
    };
    if !frontiers_aligned(canonical, member) {
        return 0.0;
    }
    let positions = literal_positions(canonical, member);
    let echoes = literal_echoes(canonical, member, sources, &positions);
    let mapping = rename_mapping(
        &population(&canonical.keys, &member.keys, Population::Identifier),
        &echoes.per_substitution,
    );
    let literals = LiteralEvidence::measure(&positions, &echoes, &mapping);
    if literals.affirming == 0 && literals.constrained > 0 {
        return 0.0;
    }
    let anchors = literals
        .affirming
        .saturating_add(mapping.anchors(scope, literals.aligned));
    let coverage = literals.coverage(&mapping, scope);
    coverage * evidence_weight(coverage, anchors, scope)
}

/// The pair's aligned literal positions, split into what the coverage
/// must explain and what it does explain ([FUSED-CONTENT-GATE],
/// [FUSED-CONTENT-GATE-PARAMETER]).
struct LiteralEvidence {
    /// Every aligned literal position, whatever it says.
    aligned: usize,
    /// Positions the coverage must explain — see
    /// [`LiteralEvidence::measure`].
    constrained: usize,
    /// Positions that affirm the copy: preserved bytes or an echo of a
    /// bijection-explained substitution.
    affirming: usize,
}

impl LiteralEvidence {
    /// Measures one pair's literal positions.
    ///
    /// A preserved literal and a literal echo affirm the copy at the
    /// position itself. A drifted literal that echoes nothing
    /// contradicts the *rename* the identifier bijection claims, so it
    /// is constrained and unexplained — the `#134` stride family renames
    /// consistently end to end and diverges at one aligned literal, and
    /// that one position is the whole difference between it and a
    /// reportable Type-2 clone.
    ///
    /// [FUSED-CONTENT-GATE-PARAMETER] Where the bijection claims no
    /// rename — no substituted identifier position is corroborated —
    /// there is no claim for a drifted literal to contradict, and
    /// [TECH-PMATCH-BAKER]'s prev-encoding applies to the literal
    /// alphabet exactly as it does to the identifier one: a substitution
    /// seen *once* is an unconstrained wildcard. Two declarations whose
    /// every identifier position is byte-identical and whose literals
    /// each substitute once are one parameterised declaration, and those
    /// literals are its parameters — `csharp-merge-manyholes` keeps
    /// every identifier and every call and substitutes at all twelve
    /// literal positions, which is what `[AUTOFIX-MERGE-GATE]`
    /// independently calls a clone too parameterised to merge
    /// mechanically.
    ///
    /// A *repeated* substitution is not a wildcard. It is the sibling
    /// family's own subject carried through its body — the star-shadow
    /// fixture's `ApplyAlpha` says `"alpha"` three times against
    /// `"dup"` — and it stays constrained, so a sibling that shares a
    /// shape and no byte cannot join the copy it sits beside. An
    /// inconsistent substitution stays constrained too: it contradicts
    /// the parameterisation as surely as it would a rename.
    fn measure(
        positions: &[LiteralPosition],
        echoes: &LiteralEchoes,
        mapping: &RenameMapping,
    ) -> Self {
        let affirming = affirming_literal_count(positions, echoes);
        let pairs = literal_pairs(positions);
        let bijection = ModalBijection::over(&substituted_pairs(&pairs));
        let occurrences = pair_counts(pairs.iter().copied());
        let constrained = if mapping.renames() {
            positions.len()
        } else {
            positions
                .iter()
                .filter(|(index, keys)| {
                    keys.0 == keys.1
                        || echoes.positions.contains(index)
                        || !bijection.explains(keys)
                        || occurrences.get(keys).copied().unwrap_or_default()
                            >= RENAME_CORROBORATION_MIN_OCCURRENCES
                })
                .count()
        };
        Self {
            aligned: positions.len(),
            constrained,
            affirming,
        }
    }

    /// The pooled coverage over this pair's constrained positions.
    ///
    /// A cross-file pair pools the literal and identifier populations
    /// into one share. A same-file pair keeps the stricter min of the
    /// two, matching the promote floor's conservatism: a same-file
    /// rename family is the `#197` sibling shape, and its literal axis
    /// must vouch on its own.
    fn coverage(&self, mapping: &RenameMapping, scope: PairScope) -> f64 {
        if scope.same_file {
            return vacuous_share(self.affirming, self.constrained)
                .min(vacuous_share(mapping.explained, mapping.constrained));
        }
        vacuous_share(
            mapping.explained.saturating_add(self.affirming),
            mapping.constrained.saturating_add(self.constrained),
        )
    }
}

/// One aligned literal position: its frontier index and the two content
/// keys at it. The frontier is walked once per pair and every literal
/// measure reads the result — the affirming count, the echo candidates,
/// and [`LiteralEvidence`].
pub(super) type LiteralPosition = (usize, (u64, u64));

/// Aligned positions where both members carry a literal.
fn literal_positions(canonical: &MemberContent, member: &MemberContent) -> Vec<LiteralPosition> {
    canonical
        .keys
        .iter()
        .zip(member.keys.iter())
        .enumerate()
        .filter(|(_, (left, right))| {
            left.population == Population::Literal && right.population == Population::Literal
        })
        .map(|(index, (left, right))| (index, (left.key, right.key)))
        .collect()
}

/// The key pairs of [`literal_positions`], for the literal bijection.
fn literal_pairs(positions: &[LiteralPosition]) -> Vec<(u64, u64)> {
    positions.iter().map(|(_, keys)| *keys).collect()
}

/// Rename-mapping evidence over one pair's aligned identifier positions
/// ([TECH-PMATCH-BAKER]), produced by [`rename_mapping`].
struct RenameMapping {
    /// Constrained identifier positions: raw-byte identity (a
    /// fixed-symbol match, witnessed by the position itself), a
    /// substitution that is bidirectionally modal *among the
    /// substituted pairs* and corroborated by repetition, and every
    /// inconsistent position. A consistent substitution seen once is
    /// Baker's unconstrained first occurrence — `prev = 0` matches any
    /// other first occurrence — so it belongs to neither the numerator
    /// nor the denominator: a renamed one-shot declaration name is not
    /// evidence against the clone, and an inconsistent position still
    /// is. The caller pools these with the pair's aligned literal
    /// positions into one coverage share ([FUSED-CONTENT-GATE]).
    constrained: usize,
    /// Explained positions — the identifier anchors [`anchor_weight`]
    /// prices. Identity positions are backed by byte equality at the
    /// position itself; substituted positions by repetition. Wildcards
    /// are backed by nothing and never count.
    explained: usize,
    /// The explained positions whose raw bytes are equal on both sides —
    /// evidence the substitution did not supply.
    identity: usize,
}

impl RenameMapping {
    /// Whether the pair claims a rename at all: some substituted
    /// identifier position is explained, so a bijection is asserting
    /// that this copy was renamed rather than merely reused.
    fn renames(&self) -> bool {
        self.explained > self.identity
    }

    /// The identifier positions that anchor the proof.
    ///
    /// A window carved from inside a function that carries no literal at
    /// all offers the substitution nothing to contradict — the literal
    /// that would is on the line the window left out — so a substitution
    /// corroborated only by its own repetition cannot anchor it. Its
    /// anchors are the positions the rename did not supply: identity
    /// identifiers ([FUSED-CONTENT-GATE-INTERIOR]). A whole authored
    /// function or module with no literal is judged as before.
    fn anchors(&self, scope: PairScope, aligned_literals: usize) -> usize {
        if scope.interior && aligned_literals == 0 {
            self.identity
        } else {
            self.explained
        }
    }
}

/// Measures [`RenameMapping`] for one pair's identifier positions,
/// classifying each position exactly as [TECH-PMATCH-BAKER]'s
/// prev-encoding constrains it: identity and corroborated substitutions
/// are explained, inconsistent positions are constrained-but-unexplained,
/// and consistent one-shot substitutions are wildcards outside the
/// population. A literal echo counts toward a substitution's
/// corroboration (#409): the echoed literal is a further position
/// witnessing the same mapping.
///
/// The parameter bijection is derived over the *substituted* pairs
/// alone — Baker's fixed symbols and parameters are disjoint alphabets,
/// and collapsed leaves carry no role, so a homonym byte-string (a
/// preserved property name that also names a renamed local) must not
/// let its identity occurrences and its substitution occurrences veto
/// each other in one modal bijection. Identity needs no mapping at all:
/// byte equality at the position is its own witness.
fn rename_mapping(
    identifiers: &[(u64, u64)],
    echoes: &BTreeMap<(u64, u64), usize>,
) -> RenameMapping {
    let substitutions = substituted_pairs(identifiers);
    let bijection = ModalBijection::over(&substitutions);
    let counts = pair_counts(substitutions.iter().copied());
    let (mut constrained, mut explained, mut identity) = (0_usize, 0_usize, 0_usize);
    for pair in identifiers {
        let substituted = pair.0 != pair.1;
        let occurrences = counts
            .get(pair)
            .copied()
            .unwrap_or_default()
            .saturating_add(echoes.get(pair).copied().unwrap_or_default());
        let corroborated = occurrences >= RENAME_CORROBORATION_MIN_OCCURRENCES;
        if substituted && bijection.explains(pair) && !corroborated {
            continue;
        }
        constrained = constrained.saturating_add(1);
        if !substituted || bijection.explains(pair) {
            explained = explained.saturating_add(1);
        }
        if !substituted {
            identity = identity.saturating_add(1);
        }
    }
    RenameMapping {
        constrained,
        explained,
        identity,
    }
}

/// The aligned positions whose raw bytes differ — [TECH-PMATCH-BAKER]'s
/// parameter alphabet, the population [`rename_mapping`] derives its
/// bijection over.
pub(super) fn substituted_pairs(identifiers: &[(u64, u64)]) -> Vec<(u64, u64)> {
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
fn anchor_weight(anchors: usize, scope: PairScope) -> f64 {
    let mass = member_count(anchors);
    let half_mass = if scope.authored {
        AUTHORED_RENAME_EVIDENCE_HALF_MASS
    } else {
        RENAME_EVIDENCE_HALF_MASS
    };
    mass / (mass + half_mass)
}

/// The mass discount actually applied to one pair's rename proof, and
/// gh #410's answer: **a rename the measurement has certified carries
/// no doubt for the mass term to price.**
///
/// [`anchor_weight`] is an asymptote — it reaches `1.0` only in the
/// limit — so multiplying it into the proof put a ceiling on the whole
/// rename axis. With [`crate::buckets::RENAME_CONSISTENCY_DISCOUNT`]
/// stacked on top, `fused >= 0.85` needed 68 affirming positions:
/// unreachable at any body length a human writes, so the top agent band
/// meant "byte-identical" instead of "do not write this copy", and a
/// maximal Type-2 rename of real logic rendered `0.729`
/// (`deslop/tests/fused_golden_bands.rs`). Two discounts were stacked
/// to produce that, and only one of them was designed to.
///
/// `consistency` is the pooled coverage. At exactly `1.0` every aligned
/// literal is preserved or echoes a bijection-explained substitution,
/// and every *constrained* identifier position is either byte-identical
/// or a bijection-explained substitution corroborated by repetition:
/// the bijection is total, contradiction-free and literal-preserving,
/// and nothing in the pair disputes it. The
/// remaining doubt the mass term prices is coincidence — and that doubt
/// is discharged by mass, which is the same quantity. So the
/// certification is granted only where the mass term **already vouches
/// for the pair on its own**, at [`CONTENT_SUPPORT_FLOOR`]: certifying
/// never promotes a cluster the mass discount would have demoted, it
/// only stops charging a proven rename for evidence it is not missing.
/// Below that bar — and for every pair carrying a single contradiction
/// — the smooth discount applies unchanged, so an anchor-poor
/// forwarding scaffold (its subject name twice plus one collaborator,
/// mass 3, weight 3/7) stays exactly where
/// `[REPAIR-RENAME-ANCHOR-MASS]` left it.
///
/// The result is monotone: completing a rename can only raise
/// `consistency` and add anchors, so certification can only switch on
/// (`rename_literal_monotonicity.rs`). Byte agreement and certified
/// rename evidence remain separate axes ([FUSED-CONTENT-GATE]).
fn evidence_weight(consistency: f64, anchors: usize, scope: PairScope) -> f64 {
    let weight = anchor_weight(anchors, scope);
    if consistency >= 1.0 && weight >= CONTENT_SUPPORT_FLOOR {
        return 1.0;
    }
    weight
}

/// The bidirectionally-modal substitution test shared by the substance
/// and rename measures: a position is explained when its pair is the
/// modal partner in both directions. A genuine rename maps every
/// occurrence of a name to one new name; scattergun similarity does
/// not. The caller chooses the population: [`mapping_consistency`]
/// maps over every identifier position (identity included), while
/// [`rename_mapping`] maps over the substituted pairs alone.
pub(super) struct ModalBijection {
    /// Modal partner of each left key.
    forward: BTreeMap<u64, u64>,
    /// Modal partner of each right key.
    backward: BTreeMap<u64, u64>,
}

impl ModalBijection {
    /// Builds the two modal maps over one pair's identifier positions.
    pub(super) fn over(identifiers: &[(u64, u64)]) -> Self {
        Self {
            forward: modal_partners(identifiers.iter().map(|(left, right)| (*left, *right))),
            backward: modal_partners(identifiers.iter().map(|(left, right)| (*right, *left))),
        }
    }

    /// True when the pair is the modal partner in both directions.
    pub(super) fn explains(&self, (left, right): &(u64, u64)) -> bool {
        self.forward.get(left) == Some(right) && self.backward.get(right) == Some(left)
    }
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
