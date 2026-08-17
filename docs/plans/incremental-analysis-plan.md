# Making analysis actually incremental

Turns `deslop` from "re-analyse everything, but skip re-parsing" into an analyser whose cost tracks the size of the change. Tracked by [gh #383](https://github.com/Nimblesite/Deslop/issues/383); the disk economics that motivate it are [gh #379](https://github.com/Nimblesite/Deslop/issues/379), and the CI surface that inherits the ceiling is [gh #381](https://github.com/Nimblesite/Deslop/issues/381). Specified by [`pipeline.md §PIPELINE-INCREMENTAL-ANALYSIS`](../specs/pipeline.md).

**The whole migration is behaviour-preserving.** An incremental pass must produce the report a cold pass produces, field for field. No threshold moves, no bucket moves, no ranking change. Any report difference is a defect in the incremental path, never an accepted cost of it.

## Where the time actually goes

400-file / 2.9 MB Rust corpus, warm cache (400 hits / 0 misses), three consecutive runs:

| stage | run 1 | run 2 | run 3 | share |
|---|---|---|---|---|
| discovery | 16 ms | 18 ms | 18 ms | 0.5% |
| parse + normalise + fingerprint — **the only persisted stage** | 302 ms | 326 ms | 304 ms | ~9% |
| LSH + candidate pair evaluation | 2486 ms | 2618 ms | 2598 ms | **~78%** |
| cluster + rank | 23 ms | 13 ms | 13 ms | 0.5% |
| render | 7 ms | 12 ms | 7 ms | 0.3% |
| buckets + metrics | 317 ms | 314 ms | 338 ms | ~10% |
| write JSON | 69 ms | 88 ms | 84 ms | 2.5% |

Wall clock corroborates: `--no-incremental` 7.10 s, fully warm 6.39 s. The store saves 10% because it is attached to a stage worth 9%.

**Two targets, in order: the 78% and the 10%.** Everything else in the table is noise, and optimising it would be theatre.

## What licenses reuse

[PIPELINE-DETERMINISM] already guarantees that identical corpus state produces identical MinHash signatures, identical candidate sets, identical fused scores, and identical cluster ids — and explicitly that this holds over corpus *state*, not edit history. That is the whole basis for reusing downstream work: a value computed from content that has not changed is still correct.

It also bounds what may **not** be reused. The embedding/ANN layer is the one approximate stage ([FUSION-EMBED-PROVIDER]); reuse there trades recall in a way the rest of this plan does not, and is out of scope below.

## Phase 0 — break down the 2.5 seconds

**Attributed.** The existing `debug` spans around signature construction, band collision enumeration, and candidate scoring already carried the split — no new instrumentation was needed. Release build over `crates/` (92,973 fingerprints), stable across three consecutive runs:

| sub-stage | time | share of the LSH block |
|---|---|---|
| MinHash signature construction (one blake3 XOF fill per k-gram) | ~1,656–1,708 ms | **~69%** |
| band collision enumeration | ~710–773 ms | ~30% |
| candidate pair scoring (`estimate_jaccard`) | ~30 ms | ~1% |
| closure + rank | ~30 ms | — |

The "pair scoring is almost certainly not the cost" hunch is confirmed and quantified. **Signature construction dominates**, which selects the first Phase 2 design below. Banding is the secondary target: `band_key` hashes the 4 row values through blake3, and identity concatenation of the rows has identical collision semantics — recorded as a follow-up optimisation, separate from the reuse work.

**Benchmark corpus and baseline — recorded.** The benchmark corpus is the pinned tokio clone ([`corpus/tokio.json`](../../corpus/tokio.json), tag `tokio-1.49.0`, sha-verified by `scripts/fetch-corpus.mjs`) — committed as a manifest rather than vendored source, deterministic by pin, and the corpus the Makefile already names fastest and most stable. Release binary, `--embeddings off`, 758 files / 1,779 clusters:

| run | wall | peak RSS | `cache_stats` |
|---|---|---|---|
| `--no-incremental` | 5.97 s | 1,412 MB | 0 / 0 |
| cold, store on | 5.96 s | 1,411 MB | 0 hit / 758 miss |
| fully warm | 5.58 s | 1,368 MB | 758 hit / 0 miss |

The parse store costs 29 MB for this corpus. All three reports are field-for-field identical with `cache_stats` removed — verified by direct JSON comparison, corroborating [PIPELINE-DETERMINISM] and the warm/cold agreement invariant on a real corpus. The cold golden report is committed at `crates/deslop/tests/fixtures/report-golden/` and enforced byte-for-byte by `report_golden.rs`, whose second half independently re-derives the golden's occurrence slices, ranking, and metrics arithmetic from the authored fixture sources so a wrongly-blessed golden cannot self-certify.

## Phase 1 — write the contract before the code

[PIPELINE-INCREMENTAL-ANALYSIS] states what an incremental pass may reuse and what equivalence it owes. It exists now as a specification of intent; Phase 1 is where it stops being aspirational:

- An equivalence test that runs a corpus cold, then runs it again after touching one file, and asserts the two reports are identical field for field — cluster ids, occurrence byte ranges, bucket, signals, ranking order, and `metrics`. This is the test the whole plan is judged by, and it must exist and pass **before** any reuse is implemented, so that it is proven to be a real assertion rather than one that happens to hold.
- The same equivalence asserted across a file add, a file delete, a rename, and a revert-to-previous-content — the last one specifically, because content-addressed reuse makes "back to a state we have seen" a distinct code path.

Exit: equivalence tests exist, pass against today's non-incremental behaviour, and would fail if a later phase served stale downstream state.

## Phase 2 — make the dominant sub-stage incremental

Phase 0 attributed ~69% of the block to signature construction, selecting the first design. The second stays recorded against the banding ~30% but is not this phase's deliverable.

**Selected and landed — full signatures in the parse blob.** A signature is a pure function of one subtree's normalised token k-grams, so it is content-addressed by the existing blob key exactly as the tree is — and inherits [PIPELINE-INCREMENTAL-INVALIDATION] unchanged: a stale signature is unaddressable, not merely unused. Each file's signatures are built once at parse/load time (`signatures_for_file`), persisted beside the fingerprints (blob magic bumped; decode enforces signature count == fingerprint count), and attached on a validated hit — the hit path re-derives fingerprints from the cached tree and any disagreement with the stored records voids the blob and takes the miss path, self-healing on the store that follows. Reuse is observable as `signatures_built` / `signatures_reused` on the `fingerprint corpus built` event, pinned by `signature_reuse.rs`.

**Why not the cheaper formats.** Persisting only the 32 band hashes per fingerprint (256 B vs 1 KB) fails on consumption: `estimate_jaccard` consumes *full* signatures for every candidate pair that reaches scoring and for every cluster's signal means, so band hashes alone would force full-signature reconstruction for exactly the fingerprints that matter — the cost the persistence exists to elide. The in-memory band index avoids disk but dies with the process, which is the wrong shape for CLI runs and CI (#381). Full signatures grow the store (measured below) and that lands squarely in the #379 economics question, which Phase 4 answers with numbers — including whether the normalised tree still earns its share of the blob now that signatures ride along.

**Recorded alternative — if banding or collision enumeration had dominated.** Persist the band index (band → bucket → fingerprint hash) rather than the signatures. An incremental pass evicts the changed files' fingerprints from their buckets, inserts the new ones, and reads off only the collisions involving them — which is the O(k·N) rather than O(N²) win, and the one that makes cost track change size.

Whichever lands, the surviving pair set is small enough to persist outright: 5,960 pairs at ~80 B is under 500 KB, three orders of magnitude below the parse store.

Exit: a one-file change on the benchmark corpus is measurably cheaper, with the Phase 1 equivalence tests green.

## Phase 3 — buckets and metrics

~10%, and second-largest once Phase 2 lands. Not worth touching before then, and worth re-measuring after — a stage that is 10% of 3.3 s is a very different proposition at 10% of 800 ms.

## Phase 4 — decide what the parse store is for

Once cost tracks change size, revisit the parse store on its own merits rather than as the only persistence there is. It costs ~40× the source it describes on this repository ([gh #379](https://github.com/Nimblesite/Deslop/issues/379)) and buys 9%. The honest options are that it earns its disk under the new economics, that it shrinks (store fingerprints, drop the normalised tree, re-derive), or that it goes. This plan does not pre-judge which; it does insist the question is asked with numbers rather than left to inertia.

## Non-goals

- **Embedding/ANN reuse.** Approximate stage, different risk profile, bounded separately by [FUSION-EMBED-PROVIDER]. The embedding cache already handles the expensive part (inference).
- **Any accuracy change.** If a phase makes a report better, that is a bug in the phase — the change belongs in its own test-first work stream with its own corpus measurement.
- **Cross-run report diffing.** Persisting reports to answer "what is new since the base commit" is [gh #381](https://github.com/Nimblesite/Deslop/issues/381) and [gh #364](https://github.com/Nimblesite/Deslop/issues/364), and rides on `ReportDelta` and stable cluster ids, both of which already exist. Unrelated to making a run cheaper.

## Spec IDs

| ID | Section | Status |
|---|---|---|
| [PIPELINE-INCREMENTAL] | The persisted parse store and its content addressing | ✅ implemented |
| [PIPELINE-INCREMENTAL-INTEGRITY] | Blob binding digest, bounded decode, size-bounded reads | ✅ implemented, pinned by `cache_blob_integrity.rs` + `fpcache/tests.rs` |
| [PIPELINE-INCREMENTAL-RETENTION] | Store pruning: stale-version partitions, orphan policy, 2 GiB budget | ✅ implemented, pinned by `cache_retention.rs` + `fpcache/retention/tests.rs` |
| [PIPELINE-INCREMENTAL-ANALYSIS] | What an incremental pass may reuse, and the equivalence it owes | ⏳ signature reuse implemented and pinned; downstream stages open |
| [CONFIG-INCREMENTAL-OPTOUT] | `[analysis] incremental = false` escape hatch | ✅ implemented, pinned by `signature_reuse.rs` |
| [PIPELINE-DETERMINISM] | The property every reuse rests on | ✅ implemented |

## Checklist

The live TODO for this plan. Every work session updates this list in the same change as the work it records.

### Phase 0 — attribution and baseline
- [x] Attribute the LSH block: signature construction ~69%, band enumeration ~30%, pair scoring ~1% (release, `crates/`, 92,973 fingerprints)
- [x] Record the attribution and the selected Phase 2 design in this plan
- [x] Commit a benchmark corpus that later phases measure against — the pinned tokio manifest (`corpus/tokio.json`, sha-verified clone)
- [x] Record the cold and warm baseline for that corpus in this plan — 5.97 s / 5.96 s / 5.58 s, reports identical modulo `cache_stats`
- [x] Commit cold golden reports that every later phase must reproduce byte-identically — `report_golden.rs` + `tests/fixtures/report-golden/` (byte-equality half plus an independent contract half derived from the authored sources)
- [x] Extend the golden to a mixed-language corpus — `incremental_multilang_golden.rs` + `tests/fixtures/incremental-multilang/` (Rust, Python, TypeScript, Dart, C#, Go; one authored Type-1 pair each, twelve byte-distinct files sharing one store). `expected-report.json` blessed and reviewed: exactly six `identical` clusters, one per language, weights ranked 52→35. Scanned at `--min-nodes 20` — below 14 the C# pair renders a second signature-line cluster that straddles [PIPELINE-CLUSTER-SUBSUME] containment by 7 bytes (gh #389, filed as its own edge)

### Phase 1 — equivalence contract ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE])
- [x] E2E test: cold run vs warm run — reports identical field for field, `cache_stats` the sole difference (`incremental_equivalence.rs::cold_and_warm_cached_runs_match_the_uncached_cold_report`)
- [x] E2E test: one-file edit — warm report equals a cold run of the edited tree (`editing_one_file_matches_the_cold_report_of_the_post_edit_tree`)
- [x] E2E tests: file add, file delete, rename, revert-to-previous-content (four scenarios in `incremental_equivalence.rs`, each with exact `cache_stats` and cluster-shape assertions)
- [x] Per-language invalidation matrix — `incremental_multilang_matrix.rs`: touch one language (exactly 1 miss / 11 hits, all six clusters unmoved), delete one language (that cluster gone, other five field-for-field identical), revert (content-addressed full-hit restore), a six-step cumulative edit chain, and byte-identical `.ts`/`.js` twins proving the store key's language component
- [x] All equivalence tests green against today's behaviour before any reuse lands — verified 6/6 green; the reuse pin `signature_reuse.rs` is in the tree born-red against the missing `signatures_built`/`signatures_reused` event fields

### Phase 2 — signature persistence
- [x] Decide the persistence format against #379 — full signatures in the parse blob; band hashes rejected because `estimate_jaccard` consumes full signatures for scoring and cluster means (rationale recorded above)
- [x] Blob format bump: signatures persisted beside fingerprints, positionally 1:1; decode rejects a count mismatch; pre-signature magic decodes as a plain miss (unit-pinned in `fpcache.rs`)
- [x] LSH consumes persisted signatures for unchanged files and constructs only for changed files — hit path validates re-derived fingerprints against stored records before attaching (`corpus/tests.rs` pins the reject-and-self-heal path); `signature_reuse.rs` green (4/4, including the store-disabled accounting contract and the `[analysis] incremental = false` config escape hatch)
- [x] Blob trust hardened per the regression audit ([`../incremental-persistence-regression-audit.md`](../incremental-persistence-regression-audit.md)): every blob carries a binding digest over `(magic, semantic epoch, signature width, min_nodes, language, source hash, payload)`; decode bounds every allocation and rejects trailing bytes; the blob file is size-bounded before read ([PIPELINE-INCREMENTAL-INTEGRITY], pinned by `cache_blob_integrity.rs` + `fpcache/tests.rs`)
- [x] One-file change on the benchmark corpus measurably cheaper; Phase 1 equivalence tests green. Release, pinned tokio, `--embeddings off`, binding-digest format: `--no-incremental` 5.88 s / 1,649 MB; cold store-on 6.22 s / 1,665 MB; fully warm 2.91 s / 1,609 MB; **one-file edit (757 hit / 1 miss) 2.92 s** — 2.0× cheaper than the store-off pass; revert restores a full-hit 2.94 s pass. All six states render byte-equal reports modulo `cache_stats`. The edit pass is *not* cheaper than fully-warm because everything downstream of signatures still runs corpus-wide — exactly the remaining phases' target
- [x] Follow-up recorded: `band_key` identity concatenation instead of blake3 (Phase 0 attribution section)

### Phase 3 — re-measure
- [x] Re-run attribution after Phase 2 (release, warm tokio pass, debug spans): discovery 7 ms (~0.2%) · parse-store load (decode + digest verify + fingerprint re-derivation, `signatures_built=0`) ~663 ms (~23%) · **LSH band enumeration ~1,276 ms (~44%) — now the dominant stage** · candidate scoring ~54 ms (~2%) · closure + rank + content ~82 ms (~3%) · buckets + metrics + JSON write ~0.7 s (~25%). Decision with numbers: signature construction is eliminated from the warm path, so the next targets in order are **banding (~44%)** — the already-recorded `band_key` follow-up and/or a persisted band index — then **buckets+metrics (~25%)**, then store-load decode (~23%). Buckets+metrics is now worth touching, but only after banding

### Phase 4 — parse-store economics
- [x] Re-run the #379 disk numbers under the new economics. Store: **185.8 MiB / 759 blobs** for a 7.3 MiB source tree (~25×; signatures are ~85% of blob bytes; +32 bytes/blob for the binding digest is noise). Verdict: **keep** — the disk buys a halved warm wall and the store is the substrate the remaining phases build on. **Shrink path recorded**: if the banding phase persists the band index, the per-fingerprint signatures (~85% of the store) stop earning their bytes and the blob drops back to roughly the pre-signature 29 MB shape
- [x] Retention landed ([PIPELINE-INCREMENTAL-RETENTION]): after every full store-on pass, stale tool-version partitions are removed, provable orphans are **kept** under a 2 GiB budget (they are exactly the revert/branch-switch reuse set the equivalence suite asserts full-hits), and over budget eviction is orphans-first then oldest-first. Pinned by `cache_retention.rs` (stale partition removed, kept orphan full-hits the revert, disabled pass never sweeps) and `fpcache/retention/tests.rs` (eviction order, budget stop, foreign-file safety)
- [x] Warm-RSS regression removed. The render-time flatten — an owned copy of every signature (~157 MiB on tokio), fingerprint, tree, and the whole source map on **every** render — is deleted; the session now owns one canonical flat store (`session/store.rs`), spliced per change and borrowed by renders. Re-measured (release, pinned tokio, default min-nodes, `--embeddings off`, single runs): `--no-incremental` 6.45 s / 1,532 MB · cold 6.48 s / 1,550 MB · **fully warm 3.31 s / 1,495 MB** · one-file edit (757 hit / 1 miss) 3.39 s / 1,497 MB · revert 3.42 s / 1,495 MB full-hit through the retained orphan. Warm peak RSS drops 1,609 → 1,495 MB (−114 MB; +127 MB over the pre-signature 1,368 MB baseline, down from +241 MB — the residue is the persisted signatures themselves, which now exist exactly once). All states verified byte-equal modulo `cache_stats`, the edit state against a cold pass of the edited tree

### Follow-ups
- [ ] Pin persisted-signature recall on the LSH-only route: extend the Python Type-3 fixture to cold, fully warm, one-file edit, and revert states; assert `structural <= 0.01`, the exact token Jaccard, `NearlyIdentical`, exact files/spans/ranking/metrics, byte-equal reports modulo `cache_stats`, and the mixed-pass `signatures_built` / `signatures_reused` split
- [ ] Bind `tool_version` into `BlobBinding` and the length-prefixed binding-digest input, while retaining `SEMANTIC_EPOCH`; add a cross-version blob-relocation test that must miss, rebuild, and self-heal
- [ ] Close the hostile-size gaps in blob loading: open once, check metadata and perform a bounded read through the same handle, use fallible allocation, enforce a global decoded-node/allocation budget, and test a metadata/read replacement plus a digest-valid near-limit count payload
- [ ] Make retention safe across concurrently running tool versions: keep stale-version partitions as budget-ranked candidates or protect them with a lease/grace period; add a two-version concurrent-sweep test and reconcile the coexistence and retention contracts
- [ ] Remove the stale `RED PIN`/unimplemented prose from `lsh_only_nearmiss_recall.rs` and make the `0.90` LSH-only routing threshold share one source of truth with pair admission, or pin their equality explicitly
- [ ] Run the normal full CI gate on the final stable snapshot and record the result here; the isolated post-fix `lsh_only_nearmiss_recall` test is already green 1/1, but that is not a substitute for the workspace gate
