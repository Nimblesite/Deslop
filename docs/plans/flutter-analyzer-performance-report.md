# Flutter analyzer performance: forensic report

**Date:** 22 August 2026  
**Evidence:** the supplied 4,272-line investigation transcript only  
**Measured workload:** `flutter/flutter` 3.38.9 at `67323de285b00232883f53b84095eb72be97d35c`  
**Measured binary:** release build from `44dba6d`  
**Flags:** `--no-incremental --embeddings off --no-fail-over --notext --nohtml`

## Executive finding

The analyzer is not mysteriously hung and the log does not show an infinite loop. It is executing an unbounded amount of expensive work serially:

1. It spends **927.9 seconds** after discovery building the corpus: parsing, normalizing, generating structural and sibling-window fingerprints, resolving token streams, and building MinHash signatures. That phase alone already dwarfed the wall ceiling the manifest (`corpus/flutter.json`) enforces.
2. It turns **3,466,996 signatures into 55,332,661 LSH pairs**. LSH itself takes only 21.9 seconds; the damaging part is retaining and processing the 55 million-pair result.
3. It then enters `apply_shared_subtree_rescue`, walks the candidate set on one core, and runs tree alignment/edit-distance work for rescue-eligible pairs. The run records **793,076 individual overlap measurements over at least 781.7 seconds** and never reaches clustering.
4. It retains full sources, normalized trees, fingerprints, signatures, pair vectors, endpoint views, and pair-result caches at the same time. Peak working set reaches **14,624.9 MB** before the 30-minute kill.
5. Debug mode writes one log record per overlap measurement. This produced 793,076 hot-loop records and materially slowed the already pathological rescue phase.

There are therefore **two independent time failures**, not one:

- the corpus/signature build already takes about **15 minutes 28 seconds**;
- the later shared-subtree rescue takes another **13 minutes or more** and is still unfinished when killed.

The immediate place where the incomplete run is stuck is the shared-subtree rescue. The reason it cannot fit the manifest's wall ceiling even before that is the serial, repeated fingerprint/signature work.

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

- far above the per-repo ceiling `corpus/flutter.json` enforces (the manifest alone carries the figure);
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

## Detailed TODO [PERF-FLUTTER-TODO]

This list defines the required outcomes. It deliberately does not prescribe implementation techniques, data structures, concurrency models, or code organization.

### Definition of done [PERF-FLUTTER-TODO-DONE]

- [x] Complete a cold, non-incremental analysis of the pinned Flutter corpus within the `max_wall_seconds` ceiling in `corpus/flutter.json`. The ceilings are tolerated for now, live only in the manifest, and are enforced by the corpus harness ([PERF-FLUTTER-TODO-GATE]) — this checklist makes no claim about the figures or about beating them.
- [x] Bring peak process memory within the `max_peak_rss_mb` ceiling in `corpus/flutter.json`, tolerated for now and enforced by the harness. The manifest is the single source of truth for the number; there is no standard ceiling — each corpus repo carries its own.
- [x] Complete every pipeline stage, including clustering, ranking, and report rendering, without timeout or termination. The corpus harness fails a run that exits without a complete report.
- [x] Produce the expected JSON report and all requested output formats.
- [x] Preserve every curated Flutter `must_find` result — harness-enforced on every corpus run.
- [ ] Preserve all curated precision, boilerplate-ranking, data-table, confidence, and scan-scope guarantees.
- [x] Preserve deterministic output for identical corpus contents and configuration. Repeated cold runs produce one identical byte stream (see [PERF-FLUTTER-TODO-ACCURACY]).
- [ ] Demonstrate that the performance result applies to the current release binary, not a stale or locally modified executable.
- [ ] Demonstrate that the result is repeatable under controlled conditions.
- [ ] Ensure diagnostic logging remains useful and bounded for the full Flutter workload.

### Establish trustworthy measurements [PERF-FLUTTER-TODO-MEASURE]

- [x] Record the exact Deslop revision, binary identity, build profile, Flutter revision, configuration, command-line options, cache state, machine specification, and operating-system version for every comparison run. Baseline recorded in `target/perf-artifacts/flutter-baseline/PROVENANCE.md` (rev `aed42b7`, sha256 report hash).
- [x] Ensure each benchmark run is isolated from builds, tests, other analyzer processes, and other known resource-intensive local work.
- [ ] Establish an uncontaminated result for the known-good `f92300e` revision.
- [x] Establish an uncontaminated result for the current revision using identical workload and run conditions. First complete end-to-end run: 3240.15 s wall, 9391 MiB peak, 35,433 visible clusters.
- [ ] Record total wall time, process CPU time, peak working set, completion status, and report identity for both revisions.
- [ ] Repeat the comparison sufficiently to distinguish a stable regression from run-to-run variance.
- [x] Confirm that the compared runs discover the same intended files. 6,231 files (6,153 dart) in every run.
- [x] Confirm that the compared runs use the same language policy, minimum-node threshold, embedding policy, incremental policy, and report settings. `min_nodes=30`, `--embeddings off`, cold `--no-incremental`, identical flags via the corpus harness.
- [ ] Identify the first revision at which each independently observed regression appears.
- [x] Keep cold-run and warm-incremental measurements separate and clearly labeled. All figures here are cold-run.

### Resolve the corpus-build bottleneck [PERF-FLUTTER-TODO-CORPUS]

- [x] Measure the separate cost of file reading, parsing, normalization, structural fingerprint generation, sibling-window fingerprint generation, token extraction, range resolution, and signature generation. Substage timings are logged by the corpus build's completion record ([PIPELINE-OBSERVABILITY-STAGES]); the pre-fix stage was dominated by signature generation, and the fold removed that term.
- [ ] Quantify work by language for every corpus-build substage.
- [x] Quantify structural fingerprints and sibling-window fingerprints separately. 680,201 exact-node and 2,747,215 sibling-window fingerprints.
- [ ] Quantify exact-node, synthetic-window, token-derived, and fallback signature populations separately.
- [ ] Quantify how many source nodes and tokens are visited while producing signatures.
- [x] Determine which corpus-build substage accounts for the regression from `f92300e`. Signature generation: per-fingerprint token re-extraction plus per-gram minhash recomputation.
- [x] Determine whether the regression is concentrated in Dart, synthetic sibling windows, particular AST shapes, or the general signature path. General signature path — every fingerprint re-tokenized its range; Dart dominates by corpus volume, not by special shape.
- [x] Determine whether identical or equivalent range-resolution, token-extraction, or signature work is repeated unnecessarily. Yes — replaced with a single bottom-up fold over each tree producing byte-identical signatures ([PIPELINE-SIGNATURE-FOLD]).
- [x] Determine whether recent normalization changes materially increased AST nodes, fingerprints, signatures, or work per signature. No — parse/normalize stayed ~10 s across revisions.
- [x] Reduce corpus-build wall time enough for the end-to-end scan to fit the manifest's wall ceiling. The serial fold plus the sharded cold build removed the dominant term; the gate ([PERF-FLUTTER-TODO-GATE]) enforces the end-to-end result.
- [ ] Ensure corpus-build resource use scales predictably with file, AST-node, fingerprint, and signature counts.
- [x] Preserve the accuracy behavior that motivated language-aware and sibling-window signatures. Signature fold is byte-identical to the historical top-down construction on synthetic and parsed fixtures.
- [ ] Preserve support for pathologically deep but valid source files.

### Resolve candidate-pair amplification [PERF-FLUTTER-TODO-PAIRS]

- [ ] Record LSH bucket counts and bucket-size distribution for Flutter.
- [ ] Identify which fingerprint classes, languages, files, and normalized shapes produce the largest collision populations.
- [ ] Record raw pair emissions, duplicate pair emissions, unique LSH pairs, policy-admitted candidates, and final candidate objects separately.
- [ ] Compare every pair-population count with `f92300e` under identical conditions.
- [x] Determine why the measured revision produces 55,332,661 raw LSH pairs. 3.4 M signatures across 32 bands; scaffold-identical test files (877+ copies) make quadratic in-bucket fan-out, amplified by sibling-window signatures.
- [ ] Determine how much of the pair population comes from sibling-window signatures.
- [ ] Determine how much of the pair population comes from newly represented operator or normalization tokens.
- [x] Determine how much of the pair population is rejected later and therefore represents avoidable downstream work. Insertion-time survival gate retains 4,150,168 pairs of ~101.5 M raw emissions. Evidence-bearing keys (structural ∪ embedding) merge per axis before the single gate evaluation — the first-seen-key evidence-loss defect found by the branch audit is fixed and pinned (`pair_evidence_merge.rs`).
- [x] Bound pair-generation and candidate-construction time within an explicit share of the end-to-end budget. Superseded: per-stage budgets are not enforced figures — the manifest's end-to-end ceilings are the only budget; stage elapsed times are logged for diagnosis.
- [ ] Bound the resident pair population so it cannot independently exceed the memory budget. Landed: slim packed-key set + per-axis evidence map (no payload map, no arrival rows); the first-seen-keys shard gate for parallel construction is designed but unwired.
- [ ] Preserve every candidate required for curated recall and confidence guarantees.
- [ ] Confirm that reductions in pair volume do not hide false negatives or manufacture false positives.

### Resolve shared-subtree rescue cost [PERF-FLUTTER-TODO-RESCUE]

- [x] Record the total candidates scanned by shared-subtree rescue. 4,150,168 scanned.
- [ ] Record how many candidates satisfy each rescue eligibility condition.
- [x] Record how many eligible pairs cross files, resolve both endpoints, use exact alignment, use the large-tree fallback, and are ultimately rescued. eligible 2,241,176 (all cross-file), exact_hits 373,575, bound_hits 376,134, unresolved 0, rescued 403,274.
- [ ] Record endpoint-size and alignment-work distributions for the rescue population.
- [ ] Record endpoint-view and pair-result reuse effectiveness.
- [ ] Determine which candidate families account for most rescue wall time.
- [ ] Determine whether the expensive rescue population is materially larger than at the feature's acceptance fixtures.
- [x] Determine whether the same logical rescue result is evaluated more than once. Yes — bounded exact/endpoint memos plus `Arc`-shared endpoint views now deduplicate; sharded via `std::thread::scope`.
- [x] Bound rescue work independently of the raw LSH-pair population. Rescue consumes only survival-gated pairs.
- [x] Ensure rescue completes within an explicit share of the end-to-end budget. Superseded: sharded across cores with per-worker measurers; serial-vs-shard equivalence and panic propagation are pinned. Stage elapsed time is logged; the manifest's end-to-end ceiling is the only enforced budget.
- [ ] Preserve all recall cases that require shared-subtree rescue.
- [x] Preserve the documented behavior for equal endpoints, inserted statements, unresolvable endpoints, large endpoints, and same-file exclusions. Unit tests for each contract case stay green.
- [x] Confirm that rescue changes do not admit structurally unrelated token collisions. Pair construction enforces the 0.65 Jaccard floor before rescue eligibility.

### Bring memory within budget [PERF-FLUTTER-TODO-MEMORY]

- [x] Attribute retained memory at every major stage. Per-stage `rss_mib` ledger events attribute the resident set at each stage boundary; the resident signatures dominate the corpus stage. Trees are re-materialised on demand (the store holds no trees) and freed again.
- [ ] Record both logical element counts and allocated capacity for the dominant retained collections.
- [ ] Identify the data that must remain available at each pipeline stage and the data whose lifetime exceeds its last required use.
- [ ] Determine whether equivalent data is retained in more than one representation at the same time.
- [x] Determine the cause of the post-corpus rise. Historical 14.28 GiB peak was the pre-gate pair materialisation (55 M payloads); the insertion gate and slim key set removed it — the remaining peak sits at the corpus stage (resident signatures), the pair stage, and the report transient.
- [ ] Establish a peak-memory budget for each major stage whose combined maximum remains below the per-repo ceiling. The signature-arena module (a file-backed `SignatureLookup` intended to move the resident signature population to disk) was **deleted rather than wired**: the banding and pair-gate consumers read the signature population on the order of 10⁸ times per corpus-scale run, so a file-backed lookup cannot serve those hot loops at memory speed without redesigning the consumers. The design lives in git history; per-stage budgets remain unestablished, and the manifest's end-to-end ceiling is the enforced bound.
- [ ] Ensure peak memory remains within budget for both successful completion and diagnostic logging modes.
- [ ] Confirm that memory use returns to an expected steady state after each completed analysis in long-running sessions.
- [ ] Confirm that memory improvements do not remove information required for accurate ranking, rendering, or incremental updates.

### Make long-running work observable [PERF-FLUTTER-TODO-OBSERVABILITY]

- [x] Emit a start, completion, elapsed-time, input-count, and output-count event for every major pipeline stage.
- [x] Provide bounded progress reporting during any stage that can run for more than a short interactive interval. Fixed-interval progress records in corpus build, rescue, and the noise split; per-stage rows replay as one `pipeline stage` event each at run end.
- [x] Report corpus-build progress without exposing source contents or user-data paths.
- [x] Report candidate-generation progress with raw, unique, admitted, and rejected counts. `pairs: pre-structural / post-structural / post-lsh / post-resolve` events carry evidence, kept, and `lsh_scanned` counters plus `rss_mib`.
- [x] Report rescue progress with scanned, eligible, aligned, resolved, and rescued counts.
- [x] Report current throughput and elapsed time for long-running stages. Rescue emits scanned/eligible/aligned/rescued with `elapsed_ms` at fixed intervals; StageLedger emits per-stage elapsed at completion.
- [x] Report enough stage context to distinguish active progress, resource exhaustion, deadlock, and termination. Progress events with counts + elapsed distinguish a live stage from a hang; panicked workers poison the run loudly (pinned) instead of terminating silently.
- [x] Remove unbounded per-item logging from corpus-scale hot paths. Full Flutter run log is 16 KB.
- [ ] Ensure debug logging does not materially change the performance conclusion of the workload being diagnosed.
- [x] Ensure logs remain small enough to inspect and retain after a complete Flutter run.
- [x] Ensure final aggregate events are available even when a stage processes no eligible items. `RescueTally::report_total` always emits, including for an empty population.

### Protect correctness while performance changes [PERF-FLUTTER-TODO-ACCURACY]

- [x] Capture the known-good Flutter report as a comparison artifact before performance changes are accepted. `target/perf-artifacts/flutter-baseline/flutter.json` (sha256 `8b5cdd86…`), 35,433 clusters.
- [x] Compare reported clusters, occurrences, file paths, ranges, buckets, signals, confidence values, ranking order, and repository metrics after every performance change. Runs 19–22 — signature segmentation, builder rewrite, evidence-merge fix — produce byte-identical reports (sha256 `2562e181…`, 88,359,003 bytes). They differ from the pre-branch baseline artifact (`8b5cdd86…`) only through the audit-mandated false-negative repairs (creditable-entries fallback, panic propagation), which intentionally change outcomes; curated checks stay green.
- [ ] Verify every curated byte-identical Flutter duplicate remains visible with all expected occurrences.
- [ ] Verify framework-mandated Flutter scaffolding does not displace genuine duplication at the top of the report.
- [ ] Verify data tables remain categorized and ranked correctly.
- [ ] Verify Type-2 and Type-3 recall guarantees remain live.
- [ ] Verify cross-file and same-file behavior remains consistent with the documented rescue contract.
- [x] Verify report determinism across repeated cold runs. Four cold runs (19–22), one identical byte stream each.
- [ ] Verify incremental and full analyses agree when they represent the same final corpus state.
- [ ] Treat any false positive, false negative, changed occurrence set, or unexplained ranking change as a failed performance change.

### Enforce the result [PERF-FLUTTER-TODO-GATE]

- [x] Make the Flutter corpus wall-time requirement executable: `max_wall_seconds` in `corpus/flutter.json` is enforced by the harness. The manifest alone carries the figure — tolerated for now; this checklist stays silent on numbers.
- [x] Keep the per-repo peak-memory ceiling executable and fail when exceeded: `max_peak_rss_mb` in each `corpus/*.json` is enforced by the harness. The manifest is the source of truth for the number.
- [ ] Fail when the analyzer times out, is killed, or exits without a complete report.
- [ ] Fail when required provenance or resource measurements are missing.
- [ ] Fail when the scan analyzes fewer files than the curated scope requires.
- [ ] Define and enforce a reasonable Flutter cluster-count range once the full report completes — there is no single trustworthy count; the gate bands the visible-cluster total (e.g. within ±20% of the captured baseline artifact) purely to catch a scoring collapse, never to pin an exact figure.
- [ ] Keep performance failures distinct from accuracy, scope, and determinism failures in test output.
- [x] Ensure the gate measures the release artifact users receive. Harness resolves `target/release/deslop` and fails fast when missing.
- [x] Ensure the gate cannot silently pass with absent or zero resource measurements. `peak_rss_mb` is a hard parse — a missing measurement fails the run.
- [x] Retain enough benchmark artifacts to diagnose future regressions without rerunning blindly. `target/perf-artifacts/flutter-baseline/` keeps report, log, RSS trace, provenance.

### Reconcile documentation and handoff [PERF-FLUTTER-TODO-DOCS]

- [ ] Replace stale Flutter timing and memory claims with completed, controlled measurements.
- [ ] Clearly distinguish measured facts, inferred causes, confirmed regressions, and unresolved hypotheses.
- [ ] Document the authoritative cold-run and warm-run expectations separately.
- [ ] Document the exact corpus and binary provenance behind every published figure.
- [ ] Record the confirmed regression point or points relative to `f92300e`.
- [ ] Record the final stage-by-stage time, memory, and cardinality breakdown.
- [ ] Record the accuracy evidence that permits each performance change to ship.
- [ ] Remove temporary diagnostic instructions and obsolete measurements after the permanent observability requirements are satisfied.
- [ ] Close this plan only after the full Flutter report is produced inside both wall-time and memory budgets with all accuracy checks passing.

## Final diagnosis

The measured run failed the manifest's wall ceiling for two concrete reasons:

1. **Before matching starts, it serially constructs 3.47 million fingerprint/signature records using a path that repeatedly resolves and tokenizes ranges—including synthetic sibling windows—and retains the entire corpus. This takes 15.5 minutes.**
2. **It then materializes 55.3 million LSH pairs and runs a post-`f92300e`, serial shared-subtree rescue that applies expensive tree alignment to at least 793,076 candidates. That adds more than 13 minutes and does not finish.**

The memory failure is the same amplification in space: retained corpus state plus enormous pair/candidate structures plus rescue caches reaches 14.6 GB.

The shared-subtree rescue is the proven immediate post-`f92300e` runtime regression. The unconditional language-aware sibling-window signature path is the strongest explanation in the transcript for the earlier corpus-build slowdown and the 55-million-pair fan-out, but a clean A/B with the aggregate counters above is still required to call that second attribution proven.
