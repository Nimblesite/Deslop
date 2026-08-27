//! [PERF-FLUTTER-TODO-PAIRS] Dynamic chunk hand-out for the sharded
//! pipeline stages.
//!
//! The rescue measurement and the cluster signal build both split a
//! long list across worker threads, and both used to cut it into one
//! contiguous chunk per worker. That is only balanced when every item
//! costs the same, and neither stage's items do: a rescue measurement
//! is a tree edit distance whose cost grows with endpoint size, and a
//! cluster's signal build is quadratic in its member count, so one
//! 877-member scaffold cluster outweighs thousands of ordinary ones.
//! Cutting a *sorted* list into contiguous blocks then lands the
//! expensive neighbours in the same block: measured on the Flutter
//! framework slice, the slowest rescue shard ran 20.8 s against a
//! 5.9 s balanced ideal, and the slowest signal shard 13.6 s against
//! 3.9 s — the other workers sat idle for most of both stages.
//!
//! Handing out many small chunks on demand fixes the balance without
//! touching what is computed. Every item is still processed exactly
//! once, by exactly one worker, and the results are reassembled in
//! chunk order, so the output is the same byte stream whatever order
//! the workers happen to claim their chunks in ([PIPELINE-DETERMINISM]).
//!
//! Each worker keeps one long-lived `state` value across every chunk it
//! claims. Both callers pay a real setup cost per state — an
//! [`crate::overlap::OverlapMeasurer`] carries the memos that stop a
//! repeated alignment being recomputed — so building one per *chunk*
//! would trade the balance win straight back for lost reuse.

use std::sync::Mutex;

/// How many worker threads a sharded stage runs for `items` units of
/// work, given the fewest units that make a worker worth spawning.
///
/// Below `min_per_worker` the thread spawn — and, where a worker owns a
/// cache, its cold start — costs more than the work itself, so the
/// stage stays on the calling thread. Above it the count is capped so
/// every worker still carries a full share rather than racing for
/// scraps. Both sharded stages ask the same question and only differ in
/// where their floor sits, so the answer lives here once: a second copy
/// drifts, and a stage that silently stops sharding is invisible in the
/// output it produces.
pub(crate) fn worker_count(items: usize, min_per_worker: usize) -> usize {
    if min_per_worker == 0 || items < min_per_worker {
        return 1;
    }
    let available = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    available.min(items / min_per_worker).max(1)
}

/// The per-chunk results in input order, paired with each worker's
/// final state in worker order. Both sequences are independent of the
/// order the workers happened to finish in.
type Sharded<R, S> = (Vec<R>, Vec<S>);

/// Runs `work` over `chunks` on `workers` threads, handing each worker
/// the next unclaimed chunk as it frees up.
///
/// `init` builds one state per worker, reused across every chunk that
/// worker claims and returned for the caller to merge.
///
/// A panicking worker is re-raised on the caller rather than dropped: a
/// swallowed join would silently omit that chunk's items while the run
/// still reported itself complete.
pub(crate) fn map_chunks<C, R, S>(
    chunks: impl Iterator<Item = C> + Send,
    workers: usize,
    init: impl Fn() -> S + Sync,
    work: impl Fn(&mut S, C) -> R + Sync,
) -> Sharded<R, S>
where
    C: Send,
    R: Send,
    S: Send,
{
    let queue = Mutex::new(chunks.enumerate());
    let mut claimed: Vec<(usize, R)> = Vec::new();
    let mut states: Vec<S> = Vec::with_capacity(workers);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers.max(1))
            .map(|_| scope.spawn(|| drain(&queue, &init, &work)))
            .collect();
        for handle in handles {
            match handle.join() {
                Ok((done, state)) => {
                    claimed.extend(done);
                    states.push(state);
                }
                Err(panic) => std::panic::resume_unwind(panic),
            }
        }
    });
    claimed.sort_by_key(|(position, _)| *position);
    (
        claimed.into_iter().map(|(_, result)| result).collect(),
        states,
    )
}

/// One worker's loop: build its state, then claim and run chunks until
/// the queue is empty.
fn drain<C, R, S>(
    queue: &Mutex<impl Iterator<Item = (usize, C)> + Send>,
    init: &(impl Fn() -> S + Sync),
    work: &(impl Fn(&mut S, C) -> R + Sync),
) -> (Vec<(usize, R)>, S) {
    let mut state = init();
    let mut done = Vec::new();
    while let Some((position, chunk)) = claim(queue) {
        done.push((position, work(&mut state, chunk)));
    }
    (done, state)
}

/// Pops the next chunk off the shared queue.
///
/// A poisoned lock is recovered rather than propagated here: the only
/// way to poison it is a panic inside this function, and the panic that
/// caused it already travels to the caller through the join above.
fn claim<C>(queue: &Mutex<impl Iterator<Item = (usize, C)> + Send>) -> Option<(usize, C)> {
    let mut guard = match queue.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.next()
}

#[cfg(test)]
mod tests;
