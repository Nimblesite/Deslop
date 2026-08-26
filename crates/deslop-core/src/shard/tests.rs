//! [PERF-FLUTTER-TODO-PAIRS] — what the dynamic hand-out must preserve:
//! input order, exactly-once processing, and a loud panic.

use super::map_chunks;

/// Workers used by the ordering pins — more than one, so the hand-out
/// is genuinely concurrent, and enough that a finish-order result would
/// diverge from an input-order one.
const TEST_WORKERS: usize = 8;

/// A population large enough that uneven chunk costs reorder completion.
const ITEMS: usize = 512;

/// Items per chunk for the ordering pins.
const CHUNK: usize = 8;

/// Results come back in chunk order even when the cheap chunks finish
/// long before the expensive ones — the property the merge relies on to
/// keep the report a stable byte stream ([PIPELINE-DETERMINISM]).
#[test]
fn results_arrive_in_input_order_not_completion_order() {
    let items: Vec<usize> = (0..ITEMS).collect();
    // The first chunk is made the slowest, so a finish-ordered result
    // would put it last. Input order must survive that.
    let (folded, _states) = map_chunks(
        items.chunks(CHUNK),
        TEST_WORKERS,
        || (),
        |(), chunk| {
            let first = chunk.first().copied().unwrap_or_default();
            if first == 0 {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            chunk.to_vec()
        },
    );
    let flattened: Vec<usize> = folded.into_iter().flatten().collect();
    assert_eq!(
        flattened, items,
        "chunk results must reassemble into the input sequence"
    );
}

/// Every item is handed to exactly one worker: none dropped, none run
/// twice. A dropped chunk is a silent false negative; a repeated one
/// double-counts its measurements.
#[test]
fn every_item_is_processed_exactly_once() {
    let items: Vec<usize> = (0..ITEMS).collect();
    let (seen, states) = map_chunks(
        items.chunks(CHUNK),
        TEST_WORKERS,
        || (),
        |(), chunk| chunk.to_vec(),
    );
    assert_eq!(
        states.len(),
        TEST_WORKERS,
        "every worker must return its state for the caller to merge"
    );
    let mut flattened: Vec<usize> = seen.into_iter().flatten().collect();
    let total = flattened.len();
    flattened.sort_unstable();
    flattened.dedup();
    assert_eq!(total, ITEMS, "every item must be processed exactly once");
    assert_eq!(flattened.len(), ITEMS, "no item may be processed twice");
}

/// A single worker still drains the whole queue — the degenerate path
/// the small-population callers take.
#[test]
fn one_worker_still_drains_every_chunk() {
    let items: Vec<usize> = (0..ITEMS).collect();
    let (folded, _states) = map_chunks(
        items.chunks(CHUNK),
        1,
        || (),
        |(), chunk: &[usize]| chunk.len(),
    );
    assert_eq!(
        folded.iter().sum::<usize>(),
        ITEMS,
        "a single worker must still cover the whole population"
    );
    assert_eq!(
        folded.len(),
        ITEMS / CHUNK,
        "every chunk must produce exactly one result"
    );
}

/// A panicking worker poisons the whole run rather than returning the
/// surviving chunks: an incomplete analysis must never render as a
/// complete one.
#[test]
fn a_panicked_worker_poisons_the_whole_run() {
    let items: Vec<usize> = (0..ITEMS).collect();
    let result = std::panic::catch_unwind(|| {
        map_chunks(
            items.chunks(CHUNK),
            TEST_WORKERS,
            || (),
            |(), chunk: &[usize]| {
                // The codebase's sanctioned deliberate-panic spelling:
                // an assert that can never hold.
                assert_ne!(
                    chunk.first().copied(),
                    Some(0),
                    "poison the chunk on purpose"
                );
                chunk.len()
            },
        )
    });
    assert!(
        result.is_err(),
        "a panicked worker's payload must propagate — a swallowed join \
         would report a partial scan as complete"
    );
}
