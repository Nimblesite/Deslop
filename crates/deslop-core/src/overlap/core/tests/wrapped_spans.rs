//! [FUSED-SHARED-SUBTREE-CORE] The aligned core where one grammar spells
//! a single span twice.
//!
//! Python gives an `expression_statement` and the expression it holds the
//! same byte range, so a docstring line normalises to two nodes over
//! identical bytes. The core resolves a sibling's Merkle hash and its
//! emitted [`Fingerprint`] previously used byte range, so both nodes
//! competed for one lookup key. The index now retains node identity.
//! These pins hold the module's stated invariant — every
//! emitted pair is one normalised shape — and the accuracy consequence of
//! losing it.

use std::collections::HashMap;

use crate::{
    ast::NormalizedNode,
    content::{measure_aligned_core, tree_index_of, PairScope},
    fingerprint::{collect_fingerprints, Fingerprint},
    lang::{python::PythonParser, LanguageParser},
    overlap::{judge_core, OverlapMeasurer},
    registry_fixtures::python_pair_ids,
    state::FileId,
};

/// The language every fixture here is written in.
const PYTHON: &str = "python";

/// Every subtree is fingerprinted, leaves included — the same setting
/// [`super::super::resolve`] uses to index an endpoint.
const EVERY_SUBTREE: usize = 1;

/// The scan's default node floor ([DECISION-MIN-NODES]), the size below
/// which no subtree is reported and so the least an aligned core may
/// carry.
const CORE_FLOOR: usize = 30;

/// Whole-file endpoints use the cross-file core policy, not an interior window.
const WHOLE_FILE_CORE: PairScope = PairScope {
    same_file: false,
    interior: false,
    core: true,
};

/// A posting routine that opens with a docstring.
const SETTLE_WITH_DOCSTRING: &str = "\
def settle(ledger, invoice):
    \"\"\"Post one invoice to the ledger.\"\"\"
    total = 0
    for line in invoice.lines:
        total = total + line.amount
    ledger.record(invoice.id, total)
    return total
";

/// The same routine copied under a full rename, with the docstring
/// dropped and an early literal return put in its place.
const APPLY_WITH_EARLY_RETURN: &str = "\
def apply(book, bill):
    return 0
    subtotal = 0
    for row in bill.lines:
        subtotal = subtotal + row.amount
    book.record(bill.id, subtotal)
    return subtotal
";

/// The same copy with the leading statement removed altogether, so the
/// only difference from [`SETTLE_WITH_DOCSTRING`] is the docstring and
/// the rename.
const APPLY_WITHOUT_LEADING_STATEMENT: &str = "\
def apply(book, bill):
    subtotal = 0
    for row in bill.lines:
        subtotal = subtotal + row.amount
    book.record(bill.id, subtotal)
    return subtotal
";

/// A posting routine whose loop body holds exactly one statement, so the
/// body `block` and the statement inside it cover identical bytes.
const SETTLE_ONE_STATEMENT_LOOP: &str = "\
def settle(ledger, invoice):
    total = 0
    for line in invoice.lines:
        total = total + line.amount
    ledger.record(invoice.id, total)
    return total
";

/// The same routine copied under a full rename with one statement
/// inserted into the loop — the Type-3 near-miss the rescue exists for.
const APPLY_TWO_STATEMENT_LOOP: &str = "\
def apply(book, bill):
    subtotal = 0
    for row in bill.lines:
        subtotal = subtotal + row.amount
        subtotal = subtotal + 1
    book.record(bill.id, subtotal)
    return subtotal
";

/// The same near-miss one statement wider on both sides, so no body
/// `block` shares its range with the statement it holds.
const SETTLE_TWO_STATEMENT_LOOP: &str = "\
def settle(ledger, invoice):
    total = 0
    for line in invoice.lines:
        total = total + line.amount
        total = total + line.tax
    ledger.record(invoice.id, total)
    return total
";

/// The wider near-miss's right-hand copy, one statement inserted.
const APPLY_THREE_STATEMENT_LOOP: &str = "\
def apply(book, bill):
    subtotal = 0
    for row in bill.lines:
        subtotal = subtotal + row.amount
        subtotal = subtotal + row.tax
        subtotal = subtotal + 1
    book.record(bill.id, subtotal)
    return subtotal
";

/// The normalised kind of a Python loop.
const FOR_STATEMENT: &str = "for_statement";

/// The normalised kind of a Python indented body.
const BLOCK: &str = "block";

/// [FUSED-SHARED-SUBTREE-CORE] The premise the byte-range index rests on.
///
/// A byte-range index cannot retain two normalised nodes over one span. Python's
/// `expression_statement` and the docstring it holds are exactly that,
/// and both are fingerprint-worthy: neither is boilerplate, and
/// `is_viewless_root` fires on the file root alone. Both must survive indexing.
#[test]
fn a_python_statement_and_its_only_child_claim_one_index_key() -> Result<(), String> {
    let (file_id, _unused) = python_pair_ids();
    let tree = parse_python(SETTLE_WITH_DOCSTRING, file_id)?;
    let collided = colliding_ranges(&tree);
    assert!(
        !collided.is_empty(),
        "the docstring fixture must normalise to a parent and child over one byte range — without that the byte-range index has nothing to lose"
    );
    let fingerprints = collect_fingerprints(&tree, EVERY_SUBTREE);
    let indexed: Vec<((usize, usize), [&'static str; 2], usize)> = collided
        .iter()
        .map(|(range, kinds)| (*range, *kinds, distinct_hashes(&fingerprints, *range)))
        .collect();
    assert!(
        indexed.iter().any(|(_, _, hashes)| *hashes >= 2),
        "some collided range must carry two fingerprint-worthy nodes with different Merkle hashes, or one index key can always answer for both; found {indexed:?}"
    );
    Ok(())
}

/// [FUSED-SHARED-SUBTREE-CORE] The module's stated invariant, held over a
/// Python near-copy whose two sides wrap their leading statement
/// differently: "Every pair emitted is Merkle-equal or a same-kind leaf
/// pair, so the two content frontiers over the core align position for
/// position". A pair whose two sides carry different normalised shapes
/// breaks the frontier alignment `joined_content` requires, and the whole
/// core is then discarded — the pair is refused on a span it shares.
#[test]
fn every_emitted_core_pair_is_one_normalised_shape() -> Result<(), String> {
    for (left_source, right_source) in near_copy_pairs() {
        let (left, right) = parse_pair(left_source, right_source)?;
        let trees = [left.tree, right.tree];
        let core = OverlapMeasurer::new(&trees).aligned_core(&left.whole, &right.whole);
        assert!(
            !core.is_empty(),
            "the two copies share whole statements, so the core cannot be empty"
        );
        for (span, partner) in &core {
            assert_eq!(
                span.hash, partner.hash,
                "every core pair is one normalised shape; {:?} against {:?} is not",
                span.byte_range, partner.byte_range
            );
        }
    }
    Ok(())
}

/// [FUSED-SHARED-SUBTREE-CORE] The accuracy consequence, stated as the
/// user sees it: a docstring on one copy must not change the verdict on
/// the code both copies share. The two right-hand sources below are the
/// same renamed copy; only the statement the docstring lines up against
/// differs, and every statement the pair actually shares is identical in
/// both.
#[test]
fn a_docstring_on_one_copy_does_not_refuse_the_shared_core() -> Result<(), String> {
    let against_early_return = core_is_a_copy(SETTLE_WITH_DOCSTRING, APPLY_WITH_EARLY_RETURN)?;
    let against_no_statement =
        core_is_a_copy(SETTLE_WITH_DOCSTRING, APPLY_WITHOUT_LEADING_STATEMENT)?;
    assert!(
        against_no_statement,
        "the renamed copy shares its whole body with the docstring fixture, so its \
         core must read as a copy"
    );
    assert_eq!(
        against_early_return, against_no_statement,
        "the two right-hand copies share the same statements with the left; the \
         statement the docstring lines up against must not decide the verdict on \
         the code they share"
    );
    Ok(())
}

/// [FUSED-SHARED-SUBTREE-CORE] The statement two copies verbatim share
/// must be one pair of the core, whatever wraps it.
///
/// A Python loop body holding one statement covers exactly the bytes of
/// that statement, so the body `block` and the statement claim one
/// byte-range key. The same near-miss one statement wider is the control:
/// nothing about the copy changed except how many statements sit under
/// the body the grammar wrapped them in.
#[test]
fn the_core_pairs_the_statement_a_single_statement_body_wraps() -> Result<(), String> {
    let cases = [
        (SETTLE_TWO_STATEMENT_LOOP, APPLY_THREE_STATEMENT_LOOP),
        (SETTLE_ONE_STATEMENT_LOOP, APPLY_TWO_STATEMENT_LOOP),
    ];
    for (left_source, right_source) in cases {
        assert_loop_pair(left_source, right_source)?;
    }
    Ok(())
}

/// Checks the shared statement's pairing and credited mass in one near-copy.
fn assert_loop_pair(left_source: &str, right_source: &str) -> Result<(), String> {
    let (left, right) = parse_pair(left_source, right_source)?;
    let trees = [left.tree, right.tree];
    let roots = split_pair(&trees)?;
    let statements = shared_loop_statements(roots)?;
    let core = OverlapMeasurer::new(&trees).aligned_core(&left.whole, &right.whole);
    assert_paired_statement(&core, roots, statements);
    let shared = count_nodes(statements.0).min(count_nodes(statements.1));
    let credited = super::super::core_node_count(&core);
    assert!(
        credited >= shared,
        "the copies share that statement verbatim, so the core must credit at least its {shared} nodes; it credits {credited} over the whole pair"
    );
    Ok(())
}

/// Resolves both statements and verifies the fixture's shared shape first.
fn shared_loop_statements<'tree>(
    (left, right): (&'tree NormalizedNode, &'tree NormalizedNode),
) -> Result<(&'tree NormalizedNode, &'tree NormalizedNode), String> {
    let ours = first_loop_body_statement(left)
        .ok_or_else(|| "the left fixture must hold a loop body statement".to_owned())?;
    let theirs = first_loop_body_statement(right)
        .ok_or_else(|| "the right fixture must hold a loop body statement".to_owned())?;
    assert_eq!(
        span_hash(left, ours),
        span_hash(right, theirs),
        "fixture guard: the first loop statement must normalise to one shape on both sides, or there is nothing for the core to pair"
    );
    Ok((ours, theirs))
}

/// Requires the whole shared statement, not leaves paired to an inserted one.
fn assert_paired_statement(
    core: &[(Fingerprint, Fingerprint)],
    roots: (&NormalizedNode, &NormalizedNode),
    (ours, theirs): (&NormalizedNode, &NormalizedNode),
) {
    assert!(
        core.iter().any(|(span, partner)| span.byte_range == ours.byte_range
            && partner.byte_range == theirs.byte_range),
        "the loop statement both copies share verbatim must be one core pair; core: {:?}; shared statements: {ours:?} against {theirs:?}",
        describe_core(core, roots),
    );
}

/// A parsed fixture: its normalised tree and the whole-file fingerprint.
struct Parsed {
    /// Normalised root.
    tree: NormalizedNode,
    /// Fingerprint spanning the tree's own byte range.
    whole: Fingerprint,
}

/// The near-copy pairs every invariant above is held over.
fn near_copy_pairs() -> [(&'static str, &'static str); 2] {
    [
        (SETTLE_WITH_DOCSTRING, APPLY_WITH_EARLY_RETURN),
        (SETTLE_WITH_DOCSTRING, APPLY_WITHOUT_LEADING_STATEMENT),
    ]
}

/// Whether the aligned core of two whole-file endpoints reads as a copy,
/// through the same two calls the rescue makes ([`judge_core`] over
/// [`measure_aligned_core`]).
fn core_is_a_copy(left_source: &str, right_source: &str) -> Result<bool, String> {
    let (left, right) = parse_pair(left_source, right_source)?;
    let trees = [left.tree, right.tree];
    let core = OverlapMeasurer::new(&trees).aligned_core(&left.whole, &right.whole);
    let sources = HashMap::from([
        (left.whole.file_id, left_source.as_bytes().to_vec()),
        (right.whole.file_id, right_source.as_bytes().to_vec()),
    ]);
    let languages = HashMap::from([(left.whole.file_id, PYTHON), (right.whole.file_id, PYTHON)]);
    let verdict = judge_core(&core, CORE_FLOOR, || {
        measure_aligned_core(
            (&left.whole, &right.whole),
            &core,
            &tree_index_of(&trees),
            &sources,
            &languages,
            WHOLE_FILE_CORE,
        )
    });
    Ok(verdict.copy)
}

/// Parses two Python fixtures into two registered files.
fn parse_pair(left_source: &str, right_source: &str) -> Result<(Parsed, Parsed), String> {
    let (left_id, right_id) = python_pair_ids();
    Ok((
        parse_whole(left_source, left_id)?,
        parse_whole(right_source, right_id)?,
    ))
}

/// Parses `source` as Python and fingerprints its whole normalised root.
fn parse_whole(source: &str, file_id: FileId) -> Result<Parsed, String> {
    let tree = parse_python(source, file_id)?;
    let whole = collect_fingerprints(&tree, EVERY_SUBTREE)
        .into_iter()
        .find(|print| print.byte_range == tree.byte_range)
        .ok_or_else(|| "the whole-file span must be fingerprinted".to_owned())?;
    Ok(Parsed { tree, whole })
}

/// Parses `source` through the real Python plug-in.
fn parse_python(source: &str, file_id: FileId) -> Result<NormalizedNode, String> {
    PythonParser
        .parse_and_normalize(source.as_bytes(), file_id)
        .map_err(|error| format!("the Python fixture must parse: {error}"))
}

/// Each core pair as the kinds and spans it claims on both sides, so a
/// failure names the code rather than raw offsets.
fn describe_core(
    core: &[(Fingerprint, Fingerprint)],
    trees: (&NormalizedNode, &NormalizedNode),
) -> Vec<String> {
    core.iter()
        .map(|(span, partner)| {
            format!(
                "{}@{}..{} = {}@{}..{}",
                kind_at(trees.0, span).unwrap_or("?"),
                span.byte_range.start,
                span.byte_range.end,
                kind_at(trees.1, partner).unwrap_or("?"),
                partner.byte_range.start,
                partner.byte_range.end,
            )
        })
        .collect()
}

/// The outermost normalised kind covering exactly `span`'s byte range.
fn kind_at(tree: &NormalizedNode, span: &Fingerprint) -> Option<&'static str> {
    let mut stack = vec![tree];
    while let Some(node) = stack.pop() {
        if node.byte_range == span.byte_range {
            return Some(node.kind);
        }
        stack.extend(
            node.children
                .iter()
                .filter(|child| child.byte_range.covers(span.byte_range)),
        );
    }
    None
}

/// The two trees of a parsed pair, in order.
fn split_pair(trees: &[NormalizedNode]) -> Result<(&NormalizedNode, &NormalizedNode), String> {
    match trees {
        [left, right] => Ok((left, right)),
        _ => Err("a parsed pair holds exactly two trees".to_owned()),
    }
}

/// The first statement of the first loop body in `tree`, located through
/// the normalised AST.
fn first_loop_body_statement(tree: &NormalizedNode) -> Option<&NormalizedNode> {
    let loop_node = find_kind(tree, FOR_STATEMENT)?;
    loop_node
        .children
        .iter()
        .rev()
        .find(|child| child.kind == BLOCK)?
        .children
        .first()
}

/// The first node of `kind` under `root`, in pre-order.
fn find_kind<'tree>(root: &'tree NormalizedNode, kind: &str) -> Option<&'tree NormalizedNode> {
    if root.kind == kind {
        return Some(root);
    }
    root.children
        .iter()
        .find_map(|child| find_kind(child, kind))
}

/// The Merkle hash fingerprinted for `node`'s own span, taking the
/// innermost claimant so a wrapper over the same bytes cannot answer for
/// it. `None` when the span carries no fingerprint at all.
fn span_hash(tree: &NormalizedNode, node: &NormalizedNode) -> Option<[u8; 32]> {
    collect_fingerprints(node, EVERY_SUBTREE)
        .into_iter()
        .find(|print| print.byte_range == node.byte_range && print.node_count == count_nodes(node))
        .map(|print| print.hash)
        .or_else(|| {
            collect_fingerprints(tree, EVERY_SUBTREE)
                .into_iter()
                .find(|print| print.byte_range == node.byte_range)
                .map(|print| print.hash)
        })
}

/// Nodes in a subtree, including its root.
fn count_nodes(node: &NormalizedNode) -> usize {
    node.children
        .iter()
        .map(count_nodes)
        .fold(1, usize::saturating_add)
}

/// How many different Merkle hashes are fingerprinted over one byte
/// range — two or more means the byte-range index cannot hold them all.
fn distinct_hashes(fingerprints: &[Fingerprint], range: (usize, usize)) -> usize {
    fingerprints
        .iter()
        .filter(|print| (print.byte_range.start, print.byte_range.end) == range)
        .map(|print| print.hash)
        .collect::<std::collections::BTreeSet<[u8; 32]>>()
        .len()
}

/// Every byte range a node shares with one of its own children, with the
/// two kinds that claim it. Walks the normalised tree — no text search.
fn colliding_ranges(root: &NormalizedNode) -> Vec<((usize, usize), [&'static str; 2])> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        for child in &node.children {
            if child.byte_range == node.byte_range {
                found.push((
                    (node.byte_range.start, node.byte_range.end),
                    [node.kind, child.kind],
                ));
            }
            stack.push(child);
        }
    }
    found
}
