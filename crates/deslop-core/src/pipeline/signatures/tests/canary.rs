//! The corpus-scale complexity canary for the bottom-up fold
//! ([PERF-FLUTTER-TODO-CORPUS], [PIPELINE-SIGNATURE-FOLD]): an
//! 80k-statement population the `O(nodes)` fold serves in milliseconds,
//! with sampled byte parity against the top-down reference. Shared
//! fixture helpers live in the parent test module.

use super::*;

/// Statements in the corpus-shaped canary tree. Sized so a reversion to
/// the historical per-fingerprint root-resolving construction —
/// `O(fingerprints × tree)`, here billions of node visits — is glaring
/// in suite wall time, while the `O(nodes)` fold stays interactive.
const CANARY_STATEMENTS: usize = 80_000;

/// Byte span each canary statement owns.
const CANARY_STATEMENT_SPAN: usize = 64;

/// Stride between sampled exact-node parity checks (prime, so samples
/// spread across shard-like regions instead of aligning to any period).
const CANARY_SAMPLE_STRIDE: usize = 7_919;

/// Stride between synthetic two-statement sibling-window fingerprints.
const CANARY_WINDOW_STRIDE: usize = 997;

/// Byte offsets inside one canary statement, relative to its start. The
/// statement's shape is fixed, so every node boundary is a named offset
/// rather than a literal at the construction site.
const CALL_START: usize = 2;
/// End of the callee identifier.
const CALLEE_END: usize = 10;
/// Start of the argument list.
const ARGS_START: usize = 12;
/// Start of the first argument identifier.
const FIRST_ARG_START: usize = 13;
/// End of the first argument identifier.
const FIRST_ARG_END: usize = 20;
/// Start of the second argument identifier.
const SECOND_ARG_START: usize = 22;
/// End of the second argument identifier.
const SECOND_ARG_END: usize = 30;
/// End of the call, and of its argument list.
const CALL_END: usize = 54;
/// End of the whole statement.
const STATEMENT_END: usize = 56;

/// The argument list of one canary statement starting at `start`.
fn canary_arguments(file_id: FileId, start: usize) -> NormalizedNode {
    let first_start = start.saturating_add(FIRST_ARG_START);
    let second_start = start.saturating_add(SECOND_ARG_START);
    node(
        file_id,
        "argument_list",
        start.saturating_add(ARGS_START),
        start.saturating_add(CALL_END),
        vec![
            node(
                file_id,
                "__ident__",
                first_start,
                start.saturating_add(FIRST_ARG_END),
                vec![],
            ),
            node(
                file_id,
                "__ident__",
                second_start,
                start.saturating_add(SECOND_ARG_END),
                vec![],
            ),
        ],
    )
}

/// The call subtree of one canary statement starting at `start`.
fn canary_call(file_id: FileId, start: usize) -> NormalizedNode {
    let call_start = start.saturating_add(CALL_START);
    node(
        file_id,
        "call",
        call_start,
        start.saturating_add(CALL_END),
        vec![
            node(
                file_id,
                "__ident__",
                call_start,
                start.saturating_add(CALLEE_END),
                vec![],
            ),
            canary_arguments(file_id, start),
        ],
    )
}

/// One canary statement: 6 named nodes, so its kind stream passes
/// `KGRAM_WIDTH` and produces a token-derived (non-fallback) signature.
fn canary_statement(file_id: FileId, index: usize) -> NormalizedNode {
    let start = index.saturating_mul(CANARY_STATEMENT_SPAN);
    node(
        file_id,
        "expression_statement",
        start,
        start.saturating_add(STATEMENT_END),
        vec![canary_call(file_id, start)],
    )
}

/// [PERF-FLUTTER-TODO-CORPUS] The fold at corpus scale: one pass over a
/// tree of ~half a million nodes serving an 80k-fingerprint population
/// (exact nodes plus sparse sibling windows), byte-faithful to the
/// top-down reference at every sampled position.
///
/// This is also the complexity canary that replaces the deleted
/// memo work-count assertions: the fold is `O(nodes)` regardless of the
/// fingerprint population, so this test runs in milliseconds — a
/// reversion to per-fingerprint root resolution multiplies its work by
/// the statement count and is unmissable in the suite's wall time.
#[test]
fn fold_scales_to_a_corpus_shaped_population_and_stays_byte_faithful() {
    let mut registry = FileRegistry::new();
    let file_id = registry.register(PathBuf::from("canary.py"));
    let statements: Vec<NormalizedNode> = (0..CANARY_STATEMENTS)
        .map(|index| canary_statement(file_id, index))
        .collect();
    let tree = file_root(file_id, statements);

    // One exact-node fingerprint per statement, plus a sparse set of
    // two-statement sibling windows — the range shape no exact node owns.
    let mut fingerprints: Vec<Fingerprint> = (0..CANARY_STATEMENTS)
        .map(|index| {
            let start = index.saturating_mul(CANARY_STATEMENT_SPAN);
            Fingerprint {
                hash: [1; 32],
                file_id,
                byte_range: ByteRange {
                    start,
                    end: start.saturating_add(STATEMENT_END),
                },
                node_count: 6,
            }
        })
        .collect();
    let exact_population = fingerprints.len();
    for index in (0..CANARY_STATEMENTS.saturating_sub(1)).step_by(CANARY_WINDOW_STRIDE) {
        let start = index.saturating_mul(CANARY_STATEMENT_SPAN);
        let end = index
            .saturating_add(1)
            .saturating_mul(CANARY_STATEMENT_SPAN)
            .saturating_add(STATEMENT_END);
        fingerprints.push(Fingerprint {
            hash: [2; 32],
            file_id,
            byte_range: ByteRange { start, end },
            node_count: 12,
        });
    }

    let folded = signatures_for_file(&tree, &fingerprints, Some("python"));
    assert_eq!(
        folded.len(),
        fingerprints.len(),
        "one signature per fingerprint across the whole population"
    );

    // Every statement is the same normalised kind stream at a different
    // offset, so every exact-node signature must equal the first — the
    // offset invariance that makes token evidence measure tokens.
    let first = folded
        .first()
        .copied()
        .unwrap_or(crate::lsh::ZEROED_SIGNATURE);
    let first_fallback = fingerprints
        .first()
        .map_or(crate::lsh::ZEROED_SIGNATURE, fallback_signature);
    assert_ne!(
        first, first_fallback,
        "the canary statement stream passes KGRAM_WIDTH, so its \
         signature must be token-derived, never the fallback"
    );
    for index in (0..exact_population).step_by(CANARY_SAMPLE_STRIDE) {
        assert_eq!(
            folded.get(index),
            Some(&first),
            "statement {index}: identical kind streams must produce \
             identical signatures at any offset"
        );
    }

    // Sampled byte parity against the historical top-down reference,
    // exact nodes and sibling windows both — the final fingerprint is
    // always sampled, so the window family is never skipped.
    let last_window = fingerprints.len().saturating_sub(1);
    for (index, fingerprint) in fingerprints
        .iter()
        .enumerate()
        .filter(|(index, _)| index % CANARY_SAMPLE_STRIDE == 0 || *index == last_window)
    {
        let reference = top_down_signature(&tree, fingerprint, Some("python"));
        assert_eq!(
            folded.get(index),
            Some(&reference),
            "fold diverged from the top-down construction at fingerprint \
             {index} (range {:?})",
            fingerprint.byte_range
        );
    }
    assert_ne!(
        folded.get(last_window),
        folded.first(),
        "a two-statement window carries a longer kind stream than one \
         statement, so its signature must differ"
    );
}
