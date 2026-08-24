# Performance branch review — closing audit

Scope: the `performance` branch against `main`. The static diff audit raised the findings indexed below; this document records each one's disposition and is the reference the pinning tests cite by finding title.

Corpus resource figures are never stated here. The ceilings live only in `corpus/*.json` (tolerated for now, enforced by the corpus harness, [PERF-FLUTTER-TODO-GATE]), and every validation claim below is backed by that gate and by the recorded report hash — never by numbers in prose.

## Findings index

Each heading is the exact title the code and tests cite.

### streamed LSH construction

`band_collisions` materialised every band collision into a `Vec` before a consumer looked at one; the replacement streams them. The risk is that a streaming emitter changes *which* pairs reach the gate. Pinned in `lsh/banding.rs`: identical band keys pair through the star, a run emits star pairs only, a pair colliding in every band emits once per band for the caller to deduplicate, `band_key` is exact identity concatenation, and `truncated_hash_collisions_never_manufacture_pairs` proves the 64-bit sort hash is an accelerator only — two signatures sharing it but differing in the full 32-byte key are never paired.

### admission parity

The insertion-time construction gate is a performance rewrite of `survival_decision` at overlap 0. It must refuse exactly the pairs the closure would drop and keep exactly the ones it would keep. Pinned in `pair/gate_parity_tests.rs`, which drives both functions over a matrix of signal triples, floors, and endpoint shapes, plus `refused_pairs_reenter_only_through_the_rescue_route`.

### first-seen pair deduplication drops stronger evidence

A key set that admits a pair on first sight and refuses every later arrival silently discards the evidence those arrivals carried — a real duplicate can disappear because a weak discovery happened to arrive first. Resolved: `PairBuilder` merges per axis (structural from the Merkle pass, strongest cosine from the embedding pass) and only then materialises. Pinned in `crates/deslop-core/tests/pair_evidence_merge.rs`.

### parallel rescue

The shared-subtree rescue now runs sharded. Every measurement is a pure function of the corpus, so sharding may change which thread computes a value but never the value. Pinned in `overlap/rescue.rs::shard_equivalence_tests` (byte-identical pair outcomes and counters against the serial path) and `a_panicked_shard_poisons_the_whole_rescue` (a dropped `Err` join would report a partial analysis as a complete one).

### mixed-size overlap fallback

A small endpoint that is a subtree nested inside a large endpoint must still be credited. Building alignment entries only for endpoints past `ALIGNMENT_MAX_NODES` leaves the small side empty, the credit at zero, and a real rescue silently dropped. Pinned in `overlap/tests.rs::a_small_endpoint_still_gets_credit_against_a_large_one`.

### segmented-store remove/upsert logic has no changed test

The store holds signatures in segments with fingerprints positionally aligned; a remove or an upsert has to drain and splice inside them. Pinned in `pipeline/session/store.rs`: mutations at the beginning, middle, and end of a multi-segment population, asserting the 1:1 fingerprint/signature alignment after every step.

### Large parallel paths lack black-box parity coverage

Restored. `pipeline/corpus/tests.rs::cold_corpus_is_identical_for_any_worker_count` builds the same corpus at worker counts 1, 2, 5, and 16 — chosen so the shard splits genuinely differ — and asserts fingerprint order, the flattened signature population, per-file entries, sources, analysed line counts, and boilerplate ranges all equal the one-worker output. One worker is the serial construction, so this pins the ordered shard merge end to end. End to end, the Flutter validation run reproduces the accepted byte-identical report hash on the fully parallel path.

### Removed signature-construction performance assertions

Restored as the corpus-scale complexity canary (`pipeline/signatures/tests/canary.rs`). The fold is `O(nodes)` regardless of fingerprint population, so a reversion to per-fingerprint root resolution multiplies the canary's work by the statement count and is unmissable in suite wall time, while sampled byte-parity against the top-down reference keeps the accuracy contract pinned.

### Flutter gate contradiction

The manifest's rationale contradicted its own ceilings and the plan checklist restated figures. Resolved by policy: `corpus/flutter.json` is the single source of truth for the ceilings, its rationale now says exactly that, and `docs/plans/flutter-analyzer-performance-report.md` points at the manifest instead of restating or undercutting the numbers. The branch's validation runs pass the harness gate and reproduce the accepted deterministic report hash (`2562e181…`, recorded in [PERF-FLUTTER-TODO-ACCURACY]).

### Signature-arena I/O error swallowing

`signature_arena.rs` converted read errors into absent similarity evidence, and the module was never connected to the pipeline. Deleted rather than wired: the banding and pair-gate consumers read the signature population on the order of 10⁸ times per corpus-scale run, so a file-backed lookup cannot serve those hot loops without redesigning the consumers. The `SignatureLookup` seam remains for any future backing that can serve reads at memory speed; the arena design lives in git history.

### Public Rust API breaks

Accepted. `band_collisions` removal, the `SignatureLookup`/`LshPairs` signatures, `subtree_at_range` returning an owned node, and the `ReportInputs.parse_cache` field are source-breaking for the internal workspace crates only. Every crate is version `0.0.0-dev` and ships solely inside the VSIX; there are no external API consumers to shim.

### Oversized modules

Resolved for every file this branch pushed past the 500-line limit: `lsh.rs` (→ `lsh/banding.rs`), `pipeline/signatures.rs` (→ `signatures/fold.rs`), `pair/candidates.rs` (→ `candidates/builder.rs`), `pair.rs` (→ `pair/closure.rs`, `pair/gate_parity_tests.rs`), `pipeline/corpus.rs` (→ `corpus/registry.rs`, `corpus/shards.rs`), `cluster_filters/snippets.rs` (→ `snippets/memos.rs`), `cluster_filters/calls.rs` (→ `calls/args.rs`), and `pipeline/signatures/tests.rs` (→ `tests/fold_parity.rs`, `tests/canary.rs`). Files already over the limit on `main` (`cluster.rs`, `lang/shared.rs`, `overlap/tests.rs`, `cluster_filters/mod.rs`, `session/mod.rs`, `tests/common/mod.rs`) received only mandated test and accuracy additions here and stay with the repo-wide oversized-file debt tracked in `docs/plans/fused-score-followups.md`.

## Pre-existing findings, not introduced here

### Empty authored evidence reported as perfect agreement

`content/frontier.rs` and `buckets/gate.rs` treat an empty authored-content union as agreement `1.0`. Both files are untouched by this branch and the behaviour exists on `main`. Tracked as issue #443.

### Near-identical routing overwrites measured token evidence

`buckets/gate.rs` replaces `token_jaccard` for `NearlyIdentical` clusters at structural score ≥ 0.99. Pre-existing on `main`. Tracked as issue #431.

## Conclusion

All branch-introduced findings are resolved, each with a named test pinning it. The two pre-existing accuracy findings are tracked (#431, #443) and unchanged by this branch. The full test suite passes, and the Flutter corpus run completes inside the manifest-enforced ceilings while reproducing the accepted deterministic report byte-for-byte.
