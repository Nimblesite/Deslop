//! Structured observability helpers for the embedding pass.

use std::time::{Duration, Instant};

/// Millisecond threshold for a slow provider batch warning.
const SLOW_PROVIDER_CALL_MS: u64 = 2_000;
/// Milliseconds in one second for throughput calculations.
const MILLIS_PER_SECOND: u64 = 1_000;

/// Accumulates cache and provider timings for one embedding pass.
#[derive(Debug)]
pub(super) struct EmbeddingObserver {
    /// Whole-pass start time.
    pass_started: Instant,
    /// Cache phase start time.
    cache_started: Instant,
    /// Unique snippets loaded from the embedding cache.
    cache_hits: usize,
    /// Unique snippets queued after a cache miss.
    cache_misses: usize,
    /// Duplicate snippets skipped before cache/provider work.
    duplicate_inputs: usize,
    /// Number of source subtrees seen by the pass.
    total_subtrees: usize,
    /// Per-provider-batch elapsed milliseconds.
    provider_elapsed_ms: Vec<u64>,
    /// Approximate whitespace token count sent to the provider.
    provider_tokens: usize,
}

impl EmbeddingObserver {
    /// Creates an observer for one embedding pass.
    pub(super) fn new(total_subtrees: usize) -> Self {
        let now = Instant::now();
        Self {
            pass_started: now,
            cache_started: now,
            cache_hits: 0,
            cache_misses: 0,
            duplicate_inputs: 0,
            total_subtrees,
            provider_elapsed_ms: Vec::new(),
            provider_tokens: 0,
        }
    }

    /// Records one unique cache hit.
    pub(super) fn record_cache_hit(&mut self) {
        self.cache_hits = self.cache_hits.saturating_add(1);
    }

    /// Records one unique cache miss.
    pub(super) fn record_cache_miss(&mut self) {
        self.cache_misses = self.cache_misses.saturating_add(1);
    }

    /// Records one duplicate subtree skipped before provider dispatch.
    pub(super) fn record_duplicate(&mut self) {
        self.duplicate_inputs = self.duplicate_inputs.saturating_add(1);
    }

    /// Emits the cache phase summary event.
    pub(super) fn log_cache_phase(&self, queued_for_provider: usize) {
        tracing::info!(
            cache_hits = self.cache_hits,
            cache_misses = self.cache_misses,
            queued_for_provider,
            duplicate_inputs = self.duplicate_inputs,
            elapsed_ms = elapsed_millis(self.cache_started.elapsed()),
            "embedding cache phase complete"
        );
    }

    /// Wraps one provider batch call with dispatch, completion, and slow-call logs.
    pub(super) fn provider_batch<T>(
        &mut self,
        batch_index: usize,
        total_batches: usize,
        batch_size: usize,
        tokens: usize,
        call: impl FnOnce() -> T,
    ) -> T {
        let timed = Self::start_provider_batch(batch_index, total_batches, batch_size, tokens);
        let result = call();
        self.finish_provider_batch(&timed);
        result
    }

    /// Emits the final embedding pass summary.
    pub(super) fn log_final(&self, pair_count: usize, embedded: usize, failed: usize) {
        let total_pass_ms = elapsed_millis(self.pass_started.elapsed());
        tracing::info!(
            pair_count,
            embedded,
            failed,
            provider_batches = self.provider_elapsed_ms.len(),
            cache_hit_pct = percent(self.cache_hits, self.cache_accesses()),
            provider_p50_ms = percentile(&self.provider_elapsed_ms, 50),
            provider_p99_ms = percentile(&self.provider_elapsed_ms, 99),
            total_pass_ms,
            subtrees_per_sec = throughput_per_sec(self.total_subtrees, total_pass_ms),
            tokens_per_sec = throughput_per_sec(self.provider_tokens, total_pass_ms),
            "embedding pass complete"
        );
    }

    /// Emits one provider dispatch-start event.
    fn start_provider_batch(
        batch_index: usize,
        total_batches: usize,
        batch_size: usize,
        tokens: usize,
    ) -> TimedProviderBatch {
        tracing::info!(
            batch_index,
            total_batches,
            batch_size,
            tokens,
            "embedding provider batch dispatch starting"
        );
        TimedProviderBatch::new(batch_index, total_batches, batch_size, tokens)
    }

    /// Emits completion and slow-call events for one provider batch.
    fn finish_provider_batch(&mut self, batch: &TimedProviderBatch) {
        let elapsed_ms = elapsed_millis(batch.started.elapsed());
        self.provider_elapsed_ms.push(elapsed_ms);
        self.provider_tokens = self.provider_tokens.saturating_add(batch.tokens);
        log_provider_complete(batch, elapsed_ms);
        log_slow_provider_batch(batch, elapsed_ms);
    }

    /// Returns the count of cache lookups with a hit/miss outcome.
    fn cache_accesses(&self) -> usize {
        self.cache_hits.saturating_add(self.cache_misses)
    }
}

/// Timed metadata for one provider batch call.
#[derive(Debug)]
struct TimedProviderBatch {
    /// One-based provider batch index.
    batch_index: usize,
    /// Total top-level provider batches.
    total_batches: usize,
    /// Number of inputs in this provider call.
    batch_size: usize,
    /// Approximate whitespace token count in this provider call.
    tokens: usize,
    /// Provider call start time.
    started: Instant,
}

impl TimedProviderBatch {
    /// Creates timed metadata for one provider call.
    fn new(batch_index: usize, total_batches: usize, batch_size: usize, tokens: usize) -> Self {
        Self {
            batch_index,
            total_batches,
            batch_size,
            tokens,
            started: Instant::now(),
        }
    }
}

/// Counts approximate whitespace-separated tokens in a provider input.
pub(super) fn token_count(input: &str) -> usize {
    input.split_whitespace().count()
}

/// Emits one provider completion event.
fn log_provider_complete(batch: &TimedProviderBatch, elapsed_ms: u64) {
    tracing::info!(
        batch_index = batch.batch_index,
        total_batches = batch.total_batches,
        batch_size = batch.batch_size,
        tokens = batch.tokens,
        elapsed_ms,
        "embedding provider batch complete"
    );
}

/// Emits a warning for provider calls over the slow-call threshold.
fn log_slow_provider_batch(batch: &TimedProviderBatch, elapsed_ms: u64) {
    if elapsed_ms > SLOW_PROVIDER_CALL_MS {
        tracing::warn!(
            batch_index = batch.batch_index,
            total_batches = batch.total_batches,
            batch_size = batch.batch_size,
            tokens = batch.tokens,
            elapsed_ms,
            "embedding provider batch slow"
        );
    }
}

/// Converts a duration into saturated whole milliseconds.
fn elapsed_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Calculates an integer percentage without floating-point casts.
fn percent(part: usize, total: usize) -> u64 {
    let Some(total) = u64::try_from(total).ok().filter(|value| *value > 0) else {
        return 0;
    };
    u64::try_from(part)
        .unwrap_or(u64::MAX)
        .saturating_mul(100)
        .checked_div(total)
        .unwrap_or(0)
}

/// Returns integer items/sec over a whole-pass duration.
fn throughput_per_sec(items: usize, elapsed_ms: u64) -> u64 {
    let Some(elapsed_ms) = non_zero(elapsed_ms) else {
        return 0;
    };
    u64::try_from(items)
        .unwrap_or(u64::MAX)
        .saturating_mul(MILLIS_PER_SECOND)
        .checked_div(elapsed_ms)
        .unwrap_or(0)
}

/// Returns a non-zero elapsed duration.
fn non_zero(value: u64) -> Option<u64> {
    (value > 0).then_some(value)
}

/// Returns a nearest-rank percentile from provider batch durations.
fn percentile(values: &[u64], pct: usize) -> u64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = sorted
        .len()
        .saturating_sub(1)
        .saturating_mul(pct)
        .checked_div(100)
        .unwrap_or(0);
    sorted.get(rank).copied().unwrap_or_default()
}
