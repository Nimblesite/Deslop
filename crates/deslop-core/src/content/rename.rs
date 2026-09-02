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
    frontier::{
        frontiers_aligned, leaf_bytes, member_count, population, MemberContent, Population,
    },
    vacuous_share,
};

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

/// Type-2 rename evidence between two members ([TECH-PMATCH-BAKER]): one
/// pooled coverage over the pair's constrained identifier positions and
/// every aligned literal position, scaled by the smooth anchor-mass
/// weight. On a cross-file pair a drifted literal that echoes nothing
/// stays in the denominator, weakening the proof in proportion to the
/// evidence around it instead of vetoing an otherwise fully-anchored
/// rename. A same-file pair keeps the stricter min of the
/// literal-affirmation share and identifier coverage, matching the
/// promote floor's conservatism: a same-file rename family is the #197
/// sibling shape, and its literal axis must vouch on its own.
/// The pool opens only where the literal population affirms at all:
/// aligned literals with zero preservation and zero echoes are the
/// #134 stride family — every substantive byte disagrees and nothing
/// outside the substitution vouches, so the axis is `0.0`.
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
    let echoes = literal_echoes(canonical, member, sources);
    let mapping = rename_mapping(
        &population(&canonical.keys, &member.keys, Population::Identifier),
        &echoes.per_substitution,
    );
    let literal_total = population(&canonical.keys, &member.keys, Population::Literal).len();
    let affirming_literals = affirming_literal_count(canonical, member, &echoes);
    if affirming_literals == 0 && literal_total > 0 {
        return 0.0;
    }
    // A window carved from inside a function that carries no literal at
    // all offers the substitution nothing to contradict — the literal
    // that would is on the line the window left out — so a substitution
    // corroborated only by its own repetition cannot anchor it. Its
    // anchors are the positions the rename did not supply: identity
    // identifiers ([FUSED-CONTENT-GATE-INTERIOR]). A whole authored
    // function or module with no literal is judged as before.
    let identifier_anchors = if scope.interior && literal_total == 0 {
        mapping.identity
    } else {
        mapping.explained
    };
    let anchors = affirming_literals.saturating_add(identifier_anchors);
    let coverage = if scope.same_file {
        vacuous_share(affirming_literals, literal_total)
            .min(vacuous_share(mapping.explained, mapping.constrained))
    } else {
        let explained = mapping.explained.saturating_add(affirming_literals);
        let constrained = mapping.constrained.saturating_add(literal_total);
        vacuous_share(explained, constrained)
    };
    coverage * evidence_weight(coverage, anchors)
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
fn evidence_weight(consistency: f64, anchors: usize) -> f64 {
    let weight = anchor_weight(anchors);
    if consistency >= 1.0 && weight >= CONTENT_SUPPORT_FLOOR {
        return 1.0;
    }
    weight
}

/// Literal echoes of the bijection's identifier substitutions (#409), as a
/// per-substitution count: an aligned literal position whose bytes
/// transform into the partner's bytes exactly by one bijection-explained
/// identifier substitution. The transform is byte-exact replacement of
/// every occurrence — content measurement over the leaf's raw bytes,
/// the same bytes the keys hash — so `"OrderService"` echoes the
/// `OrderService -> UserService` symbol substitution while a data
/// table's `"GET"` against `"POST"` echoes nothing.
fn literal_echoes<S: BuildHasher>(
    canonical: &MemberContent,
    member: &MemberContent,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> LiteralEchoes {
    let identifiers = population(&canonical.keys, &member.keys, Population::Identifier);
    let bijection = ModalBijection::over(&substituted_pairs(&identifiers));
    let substitutions = explained_substitution_bytes(canonical, member, &bijection, sources);
    let mut echoes = LiteralEchoes::default();
    for index in substituted_literal_positions(canonical, member) {
        let bytes = leaf_bytes(canonical, index, sources).zip(leaf_bytes(member, index, sources));
        let Some((left, right)) = bytes else {
            continue;
        };
        let explained_by = substitutions
            .iter()
            .find(|(_, (from, to))| replaced_matches(left, from, to, right));
        if let Some((keys, _)) = explained_by {
            let slot = echoes.per_substitution.entry(*keys).or_insert(0_usize);
            *slot = slot.saturating_add(1);
            let _newly = echoes.positions.insert(index);
        }
    }
    echoes
}

/// Aligned literal positions that affirm the copy: positions whose raw
/// bytes are preserved or whose bytes an echo explains. Every collapsed
/// literal position counts on its own, the fragments of an interpolated
/// string included — the frontier is positional, and
/// [FUSED-CONTENT-GATE] pools each aligned literal position into the
/// same coverage as the identifier positions. A preserved fragment is a
/// preserved literal; the drifted fragment beside it is a drifted one,
/// and weakens the proof in proportion like any other.
fn affirming_literal_count(
    canonical: &MemberContent,
    member: &MemberContent,
    echoes: &LiteralEchoes,
) -> usize {
    canonical
        .keys
        .iter()
        .zip(member.keys.iter())
        .enumerate()
        .filter(|(_, (left, right))| {
            left.population == Population::Literal && right.population == Population::Literal
        })
        .filter(|(index, (left, right))| left.key == right.key || echoes.positions.contains(index))
        .count()
}

/// The echo evidence of one pair (#409): per-substitution counts for
/// mapping corroboration, plus the frontier positions the echoes
/// affirmed, for the authored-literal group discipline.
#[derive(Default)]
struct LiteralEchoes {
    /// Echo count per bijection-explained substitution.
    per_substitution: BTreeMap<(u64, u64), usize>,
    /// Frontier indices whose literal bytes an echo explained.
    positions: std::collections::BTreeSet<usize>,
}

/// Frontier indices of aligned positions where both members carry a
/// literal and the raw bytes differ — the candidates an echo can
/// explain.
fn substituted_literal_positions(canonical: &MemberContent, member: &MemberContent) -> Vec<usize> {
    canonical
        .keys
        .iter()
        .zip(member.keys.iter())
        .enumerate()
        .filter(|(_, (left, right))| {
            left.population == Population::Literal
                && right.population == Population::Literal
                && left.key != right.key
        })
        .map(|(index, _)| index)
        .collect()
}

/// One bijection-explained substitution: the aligned key pair plus the
/// raw bytes on each side.
type SubstitutionBytes<'src> = ((u64, u64), (&'src [u8], &'src [u8]));

/// The distinct bijection-explained identifier substitutions of one
/// pair, with the raw bytes on each side — the substitution vocabulary
/// [`literal_echoes`] tests candidates against.
fn explained_substitution_bytes<'src, S: BuildHasher>(
    canonical: &MemberContent,
    member: &MemberContent,
    bijection: &ModalBijection,
    sources: &'src HashMap<FileId, Vec<u8>, S>,
) -> Vec<SubstitutionBytes<'src>> {
    let mut out: Vec<SubstitutionBytes<'src>> = Vec::new();
    for (index, (left, right)) in canonical.keys.iter().zip(member.keys.iter()).enumerate() {
        let keys = (left.key, right.key);
        if left.population != Population::Identifier
            || right.population != Population::Identifier
            || left.key == right.key
            || !bijection.explains(&keys)
        {
            continue;
        }
        if out.iter().any(|(seen, _)| *seen == keys) {
            continue;
        }
        let bytes = leaf_bytes(canonical, index, sources).zip(leaf_bytes(member, index, sources));
        if let Some(pair_bytes) = bytes {
            out.push((keys, pair_bytes));
        }
    }
    out
}

/// True when replacing the *symbol-boundary* occurrences of `from` in
/// `left` with `to` yields exactly `right`, with at least one occurrence
/// replaced. Pure byte-content equality under one substitution — no
/// pattern language, no tokenisation; the leaves being compared were
/// already isolated by the AST.
///
/// Replacing every raw byte occurrence instead accepted arbitrary data
/// as rename proof: under an explained `a -> x` substitution, the literal
/// `"banana"` transforms into `"bxnxnx"`, so a string whose payload
/// merely *contains* the substituted bytes corroborated the rename it
/// contradicts. Repeated across enough identifier positions that cleared
/// [`CONTENT_SUPPORT_FLOOR`], it certified `rename_consistency = 1.0`
/// for code whose literal data had changed. An echo is a *symbol* echo:
/// the bytes have to occupy a place a symbol reference could occupy —
/// `"OrderService"`, a name inside a path or a message — never the
/// inside of a longer word ([REPAIR-RENAME-LITERAL-ECHO], gh #409).
fn replaced_matches(left: &[u8], from: &[u8], to: &[u8], right: &[u8]) -> bool {
    let mut expected: Vec<u8> = Vec::with_capacity(right.len());
    let mut cursor = 0_usize;
    let mut replaced = false;
    while let Some(start) = next_occurrence(left, from, cursor) {
        let Some(head) = left.get(cursor..start) else {
            break;
        };
        expected.extend_from_slice(head);
        let boundary = at_symbol_boundary(left, start, from.len());
        expected.extend_from_slice(if boundary { to } else { from });
        replaced = replaced || boundary;
        cursor = start.saturating_add(from.len());
    }
    expected.extend_from_slice(left.get(cursor..).unwrap_or_default());
    replaced && expected == right
}

/// First offset at or after `from_index` where `needle` occurs in
/// `haystack`, `None` when there is none left.
fn next_occurrence(haystack: &[u8], needle: &[u8], from_index: usize) -> Option<usize> {
    let offset = find_bytes(haystack.get(from_index..)?, needle)?;
    Some(from_index.saturating_add(offset))
}

/// True when the window `[start, start + len)` is delimited on both
/// sides by a byte that cannot continue an identifier — the only place
/// inside a literal payload where a symbol *reference* can sit. The
/// quote characters that bound a string leaf count as delimiters, so a
/// literal that is exactly the renamed symbol still echoes it.
fn at_symbol_boundary(bytes: &[u8], start: usize, len: usize) -> bool {
    let before = start.checked_sub(1).and_then(|index| bytes.get(index));
    let after = bytes.get(start.saturating_add(len));
    !before.is_some_and(|byte| is_word_byte(*byte))
        && !after.is_some_and(|byte| is_word_byte(*byte))
}

/// True for a byte that continues an identifier-like word: ASCII
/// alphanumerics and `_`, plus every non-ASCII byte, since a UTF-8 word
/// continues through its lead and continuation bytes.
fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || !byte.is_ascii()
}

/// First byte offset of `needle` in `haystack`, `None` when absent or
/// empty.
fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
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
