# Flutter analyzer performance: forensic report

**Date:** 22 August 2026  
**Evidence:** the supplied 4,272-line investigation transcript only  
**Measured workload:** `flutter/flutter` 3.38.9 at `67323de285b00232883f53b84095eb72be97d35c`  
**Measured binary:** release build from `44dba6d`  
**Flags:** `--no-incremental --embeddings off --no-fail-over --notext --nohtml`

## Executive finding

The analyzer is not mysteriously hung and the log does not show an infinite loop. It is executing an unbounded amount of expensive work serially:

1. It spends **927.9 seconds** after discovery building the corpus: parsing, normalizing, generating structural and sibling-window fingerprints, resolving token streams, and building MinHash signatures. That phase alone already makes a sub-10-minute run impossible.
2. It turns **3,466,996 signatures into 55,332,661 LSH pairs**. LSH itself takes only 21.9 seconds; the damaging part is retaining and processing the 55 million-pair result.
3. It then enters `apply_shared_subtree_rescue`, walks the candidate set on one core, and runs tree alignment/edit-distance work for rescue-eligible pairs. The run records **793,076 individual overlap measurements over at least 781.7 seconds** and never reaches clustering.
4. It retains full sources, normalized trees, fingerprints, signatures, pair vectors, endpoint views, and pair-result caches at the same time. Peak working set reaches **14,624.9 MB** before the 30-minute kill.
5. Debug mode writes one log record per overlap measurement. This produced 793,076 hot-loop records and materially slowed the already pathological rescue phase.

There are therefore **two independent time failures**, not one:

- the corpus/signature build already takes about **15 minutes 28 seconds**;
- the later shared-subtree rescue takes another **13 minutes or more** and is still unfinished when killed.

The immediate place where the incomplete run is stuck is the shared-subtree rescue. The reason it cannot meet 10 minutes even before that is the serial, repeated fingerprint/signature work.

## What the measured run actually did

The transcript's stage timestamps give this timeline:

| Elapsed | Event | Output / state |
|---:|---|---|
| 0.0 s | Process invoked | Cold run; incremental cache disabled; embeddings disabled |
| 1.2 s | Discovery completed | 6,231 files: 6,153 Dart, 56 Python, 21 JavaScript, 1 TypeScript |
| 929.1 s | `fingerprint corpus built` | 3,466,996 fingerprints and 3,466,996 signatures |
| 932.3 s | LSH started | 3,466,996 signatures |
| 954.2 s | LSH completed | 55,332,661 candidate pairs |
| 954.2 s | Candidate construction started | Embedding contribution was zero |
| 1,019.9 s | First rescue overlap record | Candidate objects had been built and rescue processing was underway |
| 1,801.6 s | Last rescue overlap record | 793,076 overlap records emitted; rescue still incomplete |
| 1,801.7 s | Process killed | No clustering, ranking, rendering, or report |

The phrase **“parse stage” is inaccurate** for the 927.9-second interval. In the measured revision, `build_cached_file` performs all of the following before `fingerprint corpus built` is emitted:

- Tree-sitter parse and normalization;
- structural fingerprint collection;
- sibling-window fingerprint collection;
- token-range resolution and token extraction;
- MinHash signature construction for every fingerprint.

The existing log has no timing boundaries inside that combined operation, so it cannot say how much of the 927.9 seconds belongs to Tree-sitter itself. Any report claiming “parsing took 929 seconds” is overstating the evidence.

## CPU behavior

The analyzer is compute-serial on a 24-logical-core machine.

- Mean process CPU time divided by wall time was **0.983** during the sampled run.
- That is approximately one fully occupied core, not 24-core utilization.
- The process usually reported one thread and never more than four, but CPU-time/wall-time is the stronger evidence: useful computation stayed at roughly one core.
- Both the per-file corpus loop and the candidate rescue loop shown in the transcript are ordinary sequential loops.

This is not merely a parallelism problem, however. Parallelizing 55 million retained pairs and hundreds of thousands of tree edit distances would reduce wall time at the cost of even greater memory pressure. The work first needs to be bounded or avoided.

## The first bottleneck: corpus and signature construction

The combined corpus stage processes only **6.7 files per second** and builds about 3.7 thousand signatures per second. It performs the work file by file in one loop.

The strongest post-`f92300e` regression candidate visible in the transcript is the signature-path change:

- At `f92300e`, non-Python fingerprints normally used the plain exact-node token path. The language-aware range resolver ran only for Python or when an exact range was known to contain boilerplate.
- In the inspected current code, every fingerprint with a known language uses `token_stream_for_fingerprint_with_language`.
- That path recursively resolves the fingerprint range from the file root and supports synthetic sibling windows, then walks the resolved nodes and builds a real token signature.
- Synthetic sibling-window fingerprints that previously missed exact-node lookup and received a cheap deterministic fallback can now cause range resolution, token traversal, allocation, and MinHash work.

That is an accuracy-motivated behavior change, but it changes the amount and nature of work for potentially millions of fingerprints. It can also make formerly fallback/offset-scoped signatures collide on real token content, increasing the number of LSH pairs downstream.

This is a **high-probability regression mechanism, not a completed proof**. The log lacks counts for exact-node versus sibling-window fingerprints, fallback versus token-derived signatures, nodes visited during range resolution, and the same counts from `f92300e`.

The transcript also shows operator-leaf normalization changes after `f92300e`. Those can increase AST node and fingerprint counts and alter signature collision density, but no before/after counts were captured. Treat that as a secondary hypothesis, not the finding.

## The second bottleneck: 55 million pairs and shared-subtree rescue

LSH band calculation is not where the run spends its time. It produces its 55,332,661-pair output in 21.9 seconds. The problem is the cardinality of that output and what happens next.

After candidate construction, the analyzer calls `apply_shared_subtree_rescue`. The transcript shows that it:

1. walks the candidate-pair slice sequentially;
2. selects cross-file pairs whose structural score is zero, fused score is below admission, token Jaccard is at least 0.65, and endpoint size is at least 30 nodes;
3. resolves and caches endpoint tree views;
4. for views of at most 768 nodes, runs Zhang-Shasha tree edit distance through `aligned_shared_nodes`;
5. caches each pair result and writes it back to the candidate.

The exact alignment is the decisive hot path. The transcript identifies its cost as up to `O(n²·d²)` per pair at the configured cap. It is being applied to a large, data-dependent population, serially.

Direct evidence that this is where the process is stuck:

- `collecting candidate pairs` is logged at `07:49:38.293139Z`.
- The first `shared-subtree overlap measured` event is at `07:50:43.970349Z`.
- The last is at `08:03:45.636623Z`, immediately before the kill.
- There are 793,076 such events.
- The pipeline never emits `shared-subtree rescue overlaps measured` and never emits `clustering by transitive closure`, both of which occur after this loop.

The shared-subtree rescue was added after the stated known-good commit as part of the work identified in the transcript as `42b2c928 ... (#408)`. Unlike the incomplete signature-path hypothesis, this is a **directly demonstrated post-`f92300e` source of new runtime**. The observed rescue work alone exceeds 13 minutes.

The cache does not make the pair loop bounded. Endpoint-view caching may reuse a view when one endpoint appears in many pairs, but pair-result keys are normally unique candidate pairs. The log contains no cache hit/miss counts, so any stronger claim would be speculation.

## Memory behavior

Measured checkpoints:

| Elapsed | Working set | Peak | Pipeline state |
|---:|---:|---:|---|
| 60 s | 443 MB | 443 MB | Corpus/signature build |
| 300 s | 2,064 MB | 2,064 MB | Corpus/signature build |
| 600 s | 3,672 MB | 3,673 MB | Corpus/signature build |
| 900 s | 5,334 MB | 5,334 MB | Corpus/signature build |
| 929 s | 5,497 MB | 5,497 MB | Corpus/signature build ends |
| 960 s | 8,192 MB | 9,936 MB | LSH output retained / candidates being built |
| 1,010 s | 13,034 MB | 14,625 MB | Candidate/rescue processing |
| 1,800 s | 12,976 MB | 14,625 MB | Rescue still running |

The 5.9 MB/s near-linear rise during corpus construction is consistent with deliberate accumulation. The code quoted in the transcript retains each file's source plus its normalized tree, fingerprints, and signatures. Later phases add the LSH-pair vector, richer candidate-pair objects, endpoint views, and pair-result caches before the earlier structures can be released.

This is not evidence of a conventional leak or an accidental infinite allocation loop. It is an **unbounded bulk-retention design** whose working set scales with corpus size, fingerprint count, pair count, and rescue population.

The measured 14,624.9 MB peak is:

- 2.04 times the manifest ceiling of 7,168 MB;
- too large for the standard 7 GB runner budget described in the manifest;
- reached before clustering and report rendering begin.

## Why repeated attempts look like “going in circles”

The measured command explicitly uses `--no-incremental`. Every attempt therefore discards the only mechanism intended to reuse parsing, normalized trees, fingerprints, and signatures. Each retry repeats the entire 15-minute cold corpus build before reaching the failing pair path.

That flag is appropriate for a cold-path corpus ceiling test; it is not itself the defect. It does mean these logs say nothing about warm incremental performance, and it explains why repeatedly launching the same diagnostic scan starts the same expensive work from zero.

Within one run, the repeated work is also real:

- millions of fingerprint ranges are independently resolved from a file root for token/signature construction;
- tens of millions of LSH pairs are materialized and revisited as candidate objects;
- hundreds of thousands of rescue-eligible endpoints are compared with an exact tree-distance algorithm;
- all of this occurs serially while the earlier corpus representation remains resident.

## Debug logging is itself defective for this workload

`measure_onto` emits `shared-subtree overlap measured` once per eligible pair at debug level. The killed run produced 793,076 of these records. At an earlier checkpoint, 504,149 records had already produced a 72 MB log.

Consequences:

- the debug run's rescue-stage wall time is contaminated by formatting and writing one event per measurement;
- the log becomes dominated by repetitive records instead of progress information;
- the final aggregate rescue count is never reached when the process is killed;
- default `info` runs are not affected by this specific I/O cost.

The clean info-level run tracked the debug run within roughly 2% through 700 seconds and was still in corpus construction when deliberately stopped. That corroborates the 15-minute first bottleneck. It does **not** provide a clean total for the rescue stage.

## What the transcript's investigation got wrong

Several claims in the earlier report are too strong or methodologically invalid:

1. **It called the 927.9-second block “parsing.”** The event covers parsing, normalization, two fingerprint families, token resolution, and signature generation. The log cannot isolate Tree-sitter.
2. **It called the first measured run an honest total.** Debug logging wrote hundreds of thousands of hot-loop events, and the run was killed before completion.
3. **It never obtained a clean current total.** The info-level run was stopped at about 725 seconds while still in corpus construction.
4. **It never obtained a controlled `f92300e` total in the supplied log.** That process was still running at the end of the transcript.
5. **It contaminated the `f92300e` comparison.** While that full-corpus run was active, it launched a release build and then a second HEAD analyzer on the `material` subset. Its wall time, CPU availability, memory pressure, and I/O are therefore not controlled.
6. **It inferred `f92300e` stage state without live stage logs.** The comparison script buffers stderr in memory and only writes `f92300e-stderr.log` after process exit. “Still parsing” and later “past parse” were inferred from memory shape, not observed stage events.
7. **It mixed revisions.** The measured full current run used `44dba6d`; the branch advanced through `585f103`, `be2a21f`, and `8cc9553`; code comparisons were then made against moving HEAD. The unfinished subset run was the first run using the rebuilt HEAD binary.

Accordingly, the transcript proves what is wrong with the measured `44dba6d` run, but it does **not** establish an exact `f92300e`-to-HEAD slowdown factor or identify a single guilty commit by measurement.

## What is and is not proven about `f92300e`

The known-good process reached 10,929 MB peak by 270.7 seconds and was still running. The memory jump suggests it had left some earlier corpus work and entered a high-allocation pair phase, but the stage log was unavailable and the run did not complete in the supplied transcript.

What can be said safely:

- `f92300e` was processing the same checkout and command shape much faster than the measured current run reached comparable memory pressure.
- It still used roughly one core and already consumed more than 10 GB, so whole-corpus retention and high memory use predate the current regression.
- The current shared-subtree rescue is post-`f92300e` and demonstrably adds a large new serial cost.
- The current all-language sibling-window signature path is a strong candidate for the earlier preprocessing and pair-volume regression.

What cannot be said from this log:

- the exact `f92300e` corpus-build duration;
- its fingerprint/signature count;
- its LSH pair count;
- its final wall time and peak memory;
- the exact speedup/slowdown ratio;
- whether one commit alone accounts for both the corpus-build and rescue regressions.

## Missing logging needed for a conclusive attribution

No code was changed as part of this report. The smallest useful future instrumentation would be aggregate, bounded logging—not per-file paths or per-pair events.

### Corpus construction

- separate elapsed time for read, parse, normalize, structural fingerprints, sibling fingerprints, token resolution, and MinHash;
- progress every fixed time interval with files completed and fingerprints/signatures produced;
- exact-node versus sibling-window fingerprint counts;
- token-derived versus fallback signature counts;
- range-resolver calls, nodes visited, and token totals;
- per-language aggregate totals.

### LSH and candidate construction

- bucket count and largest/percentile bucket sizes;
- raw pair emissions versus unique pairs;
- candidate objects before and after each cheap gate;
- time and memory at the end of each allocation-heavy step.

### Shared-subtree rescue

- candidates scanned, eligible, resolved, aligned, and rescued;
- exact-alignment versus large-tree-fallback counts;
- endpoint-view and pair-result cache hits/misses;
- node-count histograms for aligned pairs;
- one progress record every fixed interval, with throughput and elapsed time;
- no per-pair logging in a production-scale debug run.

Those counters would determine whether the dominant first-stage cost is sibling-window range resolution, token emission, MinHash construction, normalization, or some combination. The current log cannot split them.

## Final diagnosis

The analyzer misses the 10-minute requirement for two concrete reasons:

1. **Before matching starts, it serially constructs 3.47 million fingerprint/signature records using a path that repeatedly resolves and tokenizes ranges—including synthetic sibling windows—and retains the entire corpus. This takes 15.5 minutes.**
2. **It then materializes 55.3 million LSH pairs and runs a post-`f92300e`, serial shared-subtree rescue that applies expensive tree alignment to at least 793,076 candidates. That adds more than 13 minutes and does not finish.**

The memory failure is the same amplification in space: retained corpus state plus enormous pair/candidate structures plus rescue caches reaches 14.6 GB.

The shared-subtree rescue is the proven immediate post-`f92300e` runtime regression. The unconditional language-aware sibling-window signature path is the strongest explanation in the transcript for the earlier corpus-build slowdown and the 55-million-pair fan-out, but a clean A/B with the aggregate counters above is still required to call that second attribution proven.
