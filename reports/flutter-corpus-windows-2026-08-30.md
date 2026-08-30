# Flutter corpus run — Windows performance investigation

Run date: 2026-08-30 (Australia/Sydney). Continues [flutter-corpus-2026-08-23.md](flutter-corpus-2026-08-23.md) and [flutter-performance-fix-status-2026-08-23.md](flutter-performance-fix-status-2026-08-23.md). Updated the same day after the bottleneck was profiled, fixed, and re-measured — see "Fixes and re-measurement" below.

## What ran first (baseline, before fixes)

| Metric | Result | Notes |
|---|---:|---|
| Corpus | Flutter | Pinned checkout `67323de285b0`, already present; 6,231 source files (6,153 Dart), 210 MB working tree |
| Binary | Clean release build | `cargo clean` first (1,506 files, 624.4 MiB removed) |
| Release test-target compile | 1m 43s | vs 1m 29s on 2026-08-23 |
| Corpus test run 1 | Killed at 20m 00s cap | No result line emitted |
| Corpus test run 2 | Killed at 20m 00s cap | No result line emitted; the spawned `deslop.exe` scan child survived the kill and was still running at **~40 minutes with 6.9 GB RSS** when manually terminated — it had not completed either |
| Files analysed / clusters / dup % | Not emitted | No run completed |

Command run (both timed-out runs):

```text
cargo clean
node scripts/corpus/fetch-corpus.mjs flutter
cargo test --release -p deslop --test corpus_repos corpus_flutter_dart -- --ignored --exact --nocapture --test-threads=1
```

A correction to the 2026-08-23 report's measurement setup: the corpus harness does not scan inside the test process. `scan()` spawns `target/release/deslop.exe` as a child and measures that child's `PeakWorkingSet64` from a PowerShell monitor. Any CPU/RSS probe attached to the test process itself observes an idle waiter (~21 MB), not the scan.

The gate's real ceilings come from `corpus/flutter.json`: **`max_wall_seconds: 900`, `max_peak_rss_mb: 7168`** (the reference note in the test says "roughly 9.5 GB peak and 9m44s on a laptop"). The 20-minute kill caps used in the baseline runs here were this host's own observation budget, not the gate's.

## Stage attribution sample (baseline)

A direct scan of the pinned checkout (`deslop.exe <root> --no-incremental --embeddings off --no-fail-over --no-color --notext --nohtml --log-level info`) was probed every 10 s and killed mid-corpus-build. The engine's own observability counters (`--log-level info`, written to `target/logs/deslop-<ts>.log`) gave:

| Checkpoint | Cumulative corpus-build time | Interval rate |
|---|---:|---|
| Discovery complete | 1.8 s | 6,231 files found |
| 250 files | 6.6 s | 38 files/s |
| 500 files | 16.7 s | 24 files/s |
| 1,000 files | 34.2 s | 15 files/s |
| 1,750 files | 91.6 s | 17 files/s |
| 3,000 files | 160.3 s | 22 files/s |
| 3,500 files | 183.4 s | 11 files/s |
| 4,000 files | 323.7 s | 3.7 files/s |
| 4,250 files | 422.7 s | **2.5 files/s** |

Killed at ~460 s with 4,250/6,231 files done; the `fingerprint corpus built` completion record (which carries the `read_ms`/`parse_ms`/`fingerprint_ms`/`signature_ms` split) was never reached.

Process probe over the same window:

| Observation | Value |
|---|---|
| CPU | 80–100 % of one logical core, every interval |
| Disk read operations | 0 (source set cache-warm) |
| Defender (`MsMpEng`) | idle (0 %) throughout |
| RSS growth | 137 MB @ 10 s → 630 MB @ 50 s → 2.3 GB @ 281 s → 3.1 GB @ 461 s, monotonic |

## Baseline findings

1. **The corpus build is CPU-bound on one core, not I/O-bound.** Zero disk reads during the sampled window, Defender idle, discovery finished in 1.8 s. Anti-virus interference and filesystem walking are ruled out.
2. **Per-file throughput degrades superlinearly on large files** (38 files/s → 2.5 files/s as the walk reaches the big framework files).
3. **Memory grows without bound during corpus build** — 3.1 GB at 68 % of files.
4. **No custom allocator was configured** in the workspace at baseline.

## Fixes and re-measurement (same day)

All fixes are **value-preserving**: every one was A/B-verified by byte-comparing the report JSON of a fixed 198-file subtree scan (`packages/flutter/lib/src/material`, 285,510 fingerprints, 3,493,596-byte report) against the pre-fix binary. All comparisons were identical, and the full `deslop-core` suite (153 lib + 176 integration tests) shows no new failures against the c6f75c3 baseline (the 5 suite failures that exist are pre-existing there).

### 1. MinHash gram-expansion cache (corpus build)

`DESLOP_SIG_TIMING=1` sub-stage attribution on the material subtree showed signature stage 34.8 s of which **minhash = 31.2 s (90 %)**: every k-gram occurrence paid a `kgram_bytes` Vec plus a 1,024-byte blake3 XOF expansion (~16 chained compressions), ~48 M gram occurrences per scan. Corpus-wide gram repetition is 99.94 % (28.5 M hits vs 18.5 k misses on material).

Fix: `GramExpansions` memo in `lsh.rs` keyed by gram digest, capped at 300 k entries (~300 MiB), hashing grams incrementally so the byte stream fed to the hash is unchanged.

Result (material): minhash 31.2 s → 9.3 s; signature stage 39.0 s → 17.2 s. Full-corpus bounded window (300 s): 4,250 files in 422.7 s → 162.8 s (**2.6×**); corpus build now completes (~4 min whole clone, was unbounded before).

### 2. Zhang–Shasha DP: scratch reuse + hoisted inner loop

Overlap sub-stage counters (`view_build`/`bound`/`align`/`credit`, same env gate) attributed the post-corpus stages: rescue 79.3 s and signals 87.8 s on material were **~97 % tree-alignment DP** (`align_ms`), 4.9–18 ms per alignment through bounds-checked grid accessors.

Fix: one scratch buffer reused across keyroot pairs, and the cell loop rewritten on flat row-major indexing with node lookups hoisted. Same three-way min fold — bit-identical values; the alignment unit tests pass unchanged (and run 2.6× faster).

### 3. Parallel rescue and signal measurement

The rescue's cost is `distinct structural pairs × DP cost`, and both stages ran single-threaded. Each measurement is a pure function of its two endpoint views, so both stages now use a two-pass scheme: a sequential pre-pass resolves views, memoises bounds, and collects the *distinct* missing structural pairs; the measurements run under `rayon` across all cores; a replay pass walks the pairs in the original order and reads every value from the memo. Accumulation order is the original order, so report bytes are unchanged; only observability counters shift (they now count replay memo hits).

Result (material): rescue **79.3 s → 0.045 s**; signals **87.8 s → 12.9 s**. The `align_ms` counter reads as CPU-time summed across threads (276 s of align work done in ~13 s wall on 24 threads).

### 4. Never-survive candidate pre-filter (pair generation)

The pairs vector carried every LSH/candidate collision (1.24 M on material; tens of millions on the clone) even though most can never survive clustering: a pair below its fused floor that is not rescue-eligible keeps `shared_subtree_overlap` at its initial `0.0` and is dropped later by `survival_decision`. That same compound predicate now runs inside `finalise_pairs` (same-language pipelines only — cross-language audit mode lowers floors after generation, so it defers). Result (material): 1.24 M → 479 k pairs carried (**2.6×**), identical rescue/cluster outcomes, byte-identical report. The "pair survival outcome" observability record now counts kept pairs rather than all generated ones.

### 5. Endpoint-view memo cap; mimalloc

- The endpoint-view memo is a pure cache, so it is now capped (`ENDPOINT_VIEW_CAP = 400,000`): at cap the table clears wholesale; eviction can only cost a rebuild, never change a value.
- Windows-only mimalloc global allocator A/B on material earlier gave only ~5–6 % (stage times 297 s → 281 s), so it is retained but was **not** the fix — the cost was algorithmic.

### Net effect (material subtree, 198 files, whole scan)

| Stage | Before | After |
|---|---:|---:|
| Corpus build (incl. signatures) | 43.4 s | ~17 s |
| Shared-subtree rescue | 105.9 s | **0.05 s** |
| Cluster signals | 103.0 s | **12.9 s** |
| Other cluster stages + rank/render | ~20 s | ~14 s |

## Gate status after fixes

| Metric | Ceiling | Result |
|---|---:|---|
| Wall | 900 s | **> 2,040 s, killed** (36 min; the post-corpus stages grind single-core on mega-clusters) |
| Peak RSS | 7,168 MB | **~15.8 GB** (was ~18 GB before the pair pre-filter) |

The gate still fails both ceilings, but the failure has moved: the baseline run never finished the corpus build inside 40 minutes; the fixed run finishes the corpus build in ~4 minutes, parallelises rescue and signal measurement, and spends its remaining half hour in the pair-heavy stages the RSS blocker below names. The 2026-08-23 "Mac completes in minutes" observation remains unexplained by any platform-conditional code — there is none in the scan pipeline — and is consistent only with a warm incremental cache or a different scan scope.

## Remaining blocker: peak RSS

The wall-time problem is fixed in structure (corpus build ~4 min; rescue and signals now parallel). What still fails the gate is **peak working set**, dominated at clone scale by:

1. **Candidate-pair materialisation for identical structures.** LSH band buckets over the framework's duplicated boilerplate hold thousands of members; every within-bucket pair legitimately survives (jaccard 1.0 ⇒ fused ≥ threshold), so the pre-filter cannot drop them and the pipeline materialises tens of millions of 88-byte `CandidatePair`s plus the `(usize, usize)`-keyed score map that precedes them. This is quadratic in occurrence count and is the structural fix needed: collapse equal-signature bucket members into one group with multiplicity, or stream pair generation.
2. **Per-fingerprint signature copies.** `Vec<Signature>` holds 1 KiB per fingerprint (~3.5 M fingerprints on the clone ⇒ ~3.5 GB) even though distinct token streams are far fewer; sharing via `Arc` from the existing `SignatureMemo` is the value-preserving fix.

## Tooling added

- `crates/deslop/tests/perf_sample.rs`: bounded-duration sample runner (exact corpus `SCAN_FLAGS`), now polls child exit instead of sleeping the full window.
- Env-gated sub-stage counters (`DESLOP_SIG_TIMING=1`): signature split (collect/resolve/digest/minhash) and overlap split (view_build/bound/align/credit), emitted as `… sub-stage timing` records alongside the stage records.
- `target/launch-full-test.ps1`, `target/launch-full-attr.ps1`: detached full-gate and attribution launches for background monitoring.
