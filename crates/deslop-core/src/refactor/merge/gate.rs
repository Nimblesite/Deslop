//! The mergeability gate ([AUTOFIX-MERGE-GATE]) and the first-order
//! anti-unification walk ([AUTOFIX-MERGE-ANTIUNIFY]).
//!
//! The normalised trees arrive with trivia already stripped and every
//! parameter-position leaf collapsed to `__ident__` / `__literal__`
//! (skeleton step 1 is the pipeline's own normalisation). The walk
//! confirms exact skeleton equality across all sites, records each
//! differing leaf as a hole, and enforces the Bellon/Baxter budgets.
//! The final residual check proves the zero-risk property directly:
//! outside the holes, every site's source bytes are equivalent — so
//! unnamed tokens (operators) and comments can never drift silently.

use crate::{
    ast::{ByteRange, NormalizedNode},
    lang::shared::{IDENTIFIER_KIND, LITERAL_KIND},
    report_render::canonicalise_whitespace,
};

/// Minimum total AST mass of one occurrence's forest (gate step 0).
const MASS_THRESHOLD: usize = 20;

/// Minimum physical lines spanned by one occurrence (gate step 0,
/// Bellon).
const MIN_SPAN_LINES: usize = 6;

/// Maximum differing leaf positions before coalescing (gate 4b).
const MAX_DIFF_LEAVES: usize = 6;

/// Minimum Baxter similarity `2S / (2S + L + R)` against the canonical
/// site (gate 4a).
const SIMILARITY_THRESHOLD: f64 = 0.95;

/// One leaf position where the sites disagree — a hole candidate.
#[derive(Debug, Clone)]
pub struct Hole {
    /// Normalised leaf kind (`__ident__` or `__literal__`).
    pub normalized_kind: &'static str,
    /// Per-site leaf text and byte range, in site order.
    pub per_site: Vec<HoleSite>,
}

/// One site's view of a hole.
#[derive(Debug, Clone)]
pub struct HoleSite {
    /// Verbatim leaf text at this site.
    pub text: String,
    /// Byte range of the leaf at this site.
    pub range: ByteRange,
}

/// Gate output: the holes plus the matched-node count feeding the
/// similarity score.
#[derive(Debug)]
pub struct GateOutcome {
    /// Differing leaf positions in canonical (site 0) textual order.
    pub holes: Vec<Hole>,
    /// Nodes shared by every site (the `S` of the Baxter formula).
    pub matched_nodes: usize,
}

/// Mutable alignment state threaded through the parallel walk.
struct AlignState {
    /// Holes discovered so far.
    holes: Vec<Hole>,
    /// Matching (non-hole) node count.
    matched: usize,
}

/// Runs gate steps 0–4 plus the residual byte proof over the per-site
/// statement forests.
///
/// # Errors
///
/// `Err` carries the human-readable routing reason for `ai_or_human`.
pub fn evaluate(
    forests: &[Vec<&NormalizedNode>],
    source: &[u8],
    spans: &[ByteRange],
) -> Result<GateOutcome, String> {
    size_guard(forests, source, spans)?;
    let mut state = AlignState {
        holes: Vec::new(),
        matched: 0,
    };
    align_forests(forests, source, &mut state)?;
    residual_guard(source, spans, &state.holes)?;
    Ok(GateOutcome {
        holes: state.holes,
        matched_nodes: state.matched,
    })
}

/// Gate step 0: AST mass and physical span floors (Bellon), at least
/// two survivors.
fn size_guard(
    forests: &[Vec<&NormalizedNode>],
    source: &[u8],
    spans: &[ByteRange],
) -> Result<(), String> {
    if forests.len() < 2 {
        return Err("fewer than two merge sites".to_owned());
    }
    let mass: usize = forests
        .first()
        .map(|forest| forest.iter().map(|node| node_count(node)).sum())
        .unwrap_or_default();
    if mass < MASS_THRESHOLD {
        return Err(format!(
            "AST mass {mass} below the merge floor {MASS_THRESHOLD}"
        ));
    }
    let lines = spans
        .first()
        .map(|span| span_lines(source, *span))
        .unwrap_or_default();
    if lines < MIN_SPAN_LINES {
        return Err(format!(
            "span of {lines} lines below the merge floor {MIN_SPAN_LINES}"
        ));
    }
    Ok(())
}

/// Gate steps 4a–4b, run **after** the Baker rename lifting (gate step
/// 3 precedes step 4): the budget and the Baxter similarity count
/// distinct surviving substitutions — the [AUTOFIX-MERGE-ANTIUNIFY]
/// store rule makes repeated occurrences of one substitution a single
/// variable, not many differences.
///
/// # Errors
///
/// `Err` carries the routing reason for `ai_or_human`.
pub fn budget_guard(matched_nodes: usize, substitution_count: usize) -> Result<(), String> {
    if substitution_count > MAX_DIFF_LEAVES {
        return Err(format!(
            "{substitution_count} distinct substitutions exceed the budget of {MAX_DIFF_LEAVES}"
        ));
    }
    let shared = 2_f64 * to_f64(matched_nodes);
    let similarity = shared / (shared + 2_f64 * to_f64(substitution_count));
    if similarity < SIMILARITY_THRESHOLD {
        return Err(format!(
            "similarity {similarity:.3} below the {SIMILARITY_THRESHOLD} threshold"
        ));
    }
    Ok(())
}

/// The zero-risk residual proof: cutting every hole out of every
/// site's span must leave whitespace-canonically equal bytes. Unnamed
/// tokens (operators) and comments are invisible to the normalised
/// skeleton, so only this byte-level check can rule their drift out.
fn residual_guard(source: &[u8], spans: &[ByteRange], holes: &[Hole]) -> Result<(), String> {
    let residuals: Option<Vec<Vec<u8>>> = spans
        .iter()
        .enumerate()
        .map(|(site, span)| residual_bytes(source, *span, holes, site))
        .collect();
    let Some(residuals) = residuals else {
        return Err("occurrence bytes unavailable for the residual proof".to_owned());
    };
    let equal = residuals
        .windows(2)
        .all(|pair| matches!(pair, [left, right] if left == right));
    equal.then_some(()).ok_or_else(|| {
        "code outside the differing leaves is not byte-equivalent (comment or operator drift)"
            .to_owned()
    })
}

/// One site's span bytes with its hole ranges cut out and whitespace
/// canonicalised.
fn residual_bytes(source: &[u8], span: ByteRange, holes: &[Hole], site: usize) -> Option<Vec<u8>> {
    let mut cuts: Vec<ByteRange> = holes
        .iter()
        .filter_map(|hole| hole.per_site.get(site).map(|entry| entry.range))
        .collect();
    cuts.sort_unstable_by_key(|range| range.start);
    let mut output = Vec::new();
    let mut cursor = span.start;
    for cut in cuts {
        output.extend_from_slice(source.get(cursor..cut.start)?);
        cursor = cut.end;
    }
    output.extend_from_slice(source.get(cursor..span.end)?);
    Some(canonicalise_whitespace(&output))
}

/// Lossless-enough usize → f64 for similarity math on small counts.
fn to_f64(value: usize) -> f64 {
    u32::try_from(value).map_or(f64::MAX, f64::from)
}

/// Aligns N same-index statement forests (gate step 2 skeleton
/// equality, [AUTOFIX-MERGE-ANTIUNIFY] decompose rule).
fn align_forests(
    forests: &[Vec<&NormalizedNode>],
    source: &[u8],
    state: &mut AlignState,
) -> Result<(), String> {
    let widths: Vec<usize> = forests.iter().map(Vec::len).collect();
    if widths
        .windows(2)
        .any(|pair| matches!(pair, [left, right] if left != right))
    {
        return Err("statement counts differ across occurrences".to_owned());
    }
    for index in 0..widths.first().copied().unwrap_or_default() {
        let nodes: Vec<&NormalizedNode> = forests
            .iter()
            .filter_map(|forest| forest.get(index).copied())
            .collect();
        align_nodes(&nodes, source, state)?;
    }
    Ok(())
}

/// Aligns one node position across all sites: agreeing heads decompose
/// ([AUTOFIX-MERGE-ANTIUNIFY] DECOMPOSE); disagreeing positions must be
/// parameter-position leaves ([AUTOFIX-MERGE-GATE] 4d) and become holes
/// (SOLVE).
fn align_nodes(
    nodes: &[&NormalizedNode],
    source: &[u8],
    state: &mut AlignState,
) -> Result<(), String> {
    let first = nodes
        .first()
        .ok_or_else(|| "empty alignment position".to_owned())?;
    let heads_agree = nodes
        .iter()
        .all(|node| node.kind == first.kind && node.children.len() == first.children.len());
    if !heads_agree {
        return Err(
            "structural drift between occurrences (statements added or reshaped)".to_owned(),
        );
    }
    // Collapsed parameter-position nodes align atomically over their
    // full span — a C# `string_literal` keeps a content child, an
    // interpolated string keeps expression children, yet the whole
    // literal is the exchangeable unit.
    if first.children.is_empty() || first.kind == IDENTIFIER_KIND || first.kind == LITERAL_KIND {
        return align_leaves(nodes, source, state, first.kind);
    }
    state.matched = state.matched.saturating_add(1);
    for index in 0..first.children.len() {
        let children: Vec<&NormalizedNode> = nodes
            .iter()
            .filter_map(|node| node.children.get(index))
            .collect();
        align_nodes(&children, source, state)?;
    }
    Ok(())
}

/// Compares one leaf position: equal texts match, differing texts in a
/// parameter position (identifier / literal) become a hole, differing
/// texts anywhere else refuse (gate 4d).
fn align_leaves(
    nodes: &[&NormalizedNode],
    source: &[u8],
    state: &mut AlignState,
    kind: &'static str,
) -> Result<(), String> {
    let entries: Option<Vec<HoleSite>> = nodes
        .iter()
        .map(|node| {
            source
                .get(node.byte_range.start..node.byte_range.end)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map(|text| HoleSite {
                    text: text.to_owned(),
                    range: node.byte_range,
                })
        })
        .collect();
    let Some(entries) = entries else {
        return Err("occurrence bytes unavailable".to_owned());
    };
    if entries
        .windows(2)
        .all(|pair| matches!(pair, [left, right] if left.text == right.text))
    {
        state.matched = state.matched.saturating_add(1);
        return Ok(());
    }
    if kind != IDENTIFIER_KIND && kind != LITERAL_KIND {
        return Err(format!(
            "a non-parameter position (`{kind}`) differs across occurrences"
        ));
    }
    state.holes.push(Hole {
        normalized_kind: kind,
        per_site: entries,
    });
    Ok(())
}

/// Total node count of a normalised subtree.
fn node_count(node: &NormalizedNode) -> usize {
    1_usize.saturating_add(node.children.iter().map(node_count).sum::<usize>())
}

/// Physical lines covered by `span` in `source`.
fn span_lines(source: &[u8], span: ByteRange) -> usize {
    source
        .get(span.start..span.end)
        .map(|slice| slice.split(|byte| *byte == b'\n').count())
        .unwrap_or_default()
}
