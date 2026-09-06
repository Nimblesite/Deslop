//! Literal echoes of a rename ([REPAIR-RENAME-LITERAL-ECHO], gh #409).
//!
//! A literal renamed *alongside the symbol it names* is part of the
//! rename, not evidence against it: `"OrderService"` renamed to
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

use crate::state::FileId;

use super::super::frontier::{leaf_bytes, population, MemberContent, Population};
use super::{substituted_pairs, LiteralPosition, ModalBijection};

/// Literal echoes of the bijection's identifier substitutions (#409), as a
/// per-substitution count: an aligned literal position whose bytes
/// transform into the partner's bytes exactly by one bijection-explained
/// identifier substitution. The transform is byte-exact replacement of
/// every occurrence — content measurement over the leaf's raw bytes,
/// the same bytes the keys hash — so `"OrderService"` echoes the
/// `OrderService -> UserService` symbol substitution while a data
/// table's `"GET"` against `"POST"` echoes nothing.
pub(super) fn literal_echoes<S: BuildHasher>(
    canonical: &MemberContent,
    member: &MemberContent,
    sources: &HashMap<FileId, Vec<u8>, S>,
    positions: &[LiteralPosition],
) -> LiteralEchoes {
    let identifiers = population(&canonical.keys, &member.keys, Population::Identifier);
    let bijection = ModalBijection::over(&substituted_pairs(&identifiers));
    let substitutions = explained_substitution_bytes(canonical, member, &bijection, sources);
    let mut echoes = LiteralEchoes::default();
    for index in substituted_literal_positions(positions) {
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
pub(super) fn affirming_literal_count(
    positions: &[LiteralPosition],
    echoes: &LiteralEchoes,
) -> usize {
    positions
        .iter()
        .filter(|(index, (left, right))| left == right || echoes.positions.contains(index))
        .count()
}

/// The echo evidence of one pair (#409): per-substitution counts for
/// mapping corroboration, plus the frontier positions the echoes
/// affirmed, for the authored-literal group discipline.
#[derive(Default)]
pub(super) struct LiteralEchoes {
    /// Echo count per bijection-explained substitution.
    pub(super) per_substitution: BTreeMap<(u64, u64), usize>,
    /// Frontier indices whose literal bytes an echo explained.
    pub(super) positions: std::collections::BTreeSet<usize>,
}

/// Frontier indices of the aligned literal positions whose raw bytes
/// differ — the candidates an echo can explain.
fn substituted_literal_positions(positions: &[LiteralPosition]) -> Vec<usize> {
    positions
        .iter()
        .filter(|(_, (left, right))| left != right)
        .map(|(index, _)| *index)
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
