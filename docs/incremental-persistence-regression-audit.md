# Incremental persisted-processing regression audit

Audit date: 2026-08-17

Scope: the persisted per-fingerprint MinHash work in `docs/plans/incremental-analysis-plan.md`, including blob integrity, warm/cold report equivalence, signature reuse, the workspace opt-out, and the resource regressions introduced by storing full signatures. Cross-run report diffing and issue #364 remain out of scope.

## Verdict

**The original cache-correctness defects are fixed, but this branch is not merge-verifiable yet.** Findings 1–4, 7, and 8 from the previous audit now have matching implementation and focused unit coverage. The cache binds every blob to the lookup address, rejects malformed or stale data before serving it, and persists architecture-independent fallback signatures.

## Resolution — 2026-08-17, same day

Every required next step below is closed, with one deliberate red pin left in the tree per the accuracy rules.

1. **Compile break fixed; all five suites rerun green.** `corpora.rs` imports `cluster_file_set`; the unused `corpora::*` re-export is gone and the seven consuming suites import `corpora::*` explicitly, matching every other `common` submodule. Actual counts: `signature_reuse` 4/4 · `cache_blob_integrity` 4/4 · `incremental_equivalence` 6/6 · `incremental_multilang_golden` 3/3 · `incremental_multilang_matrix` 4/4. Workspace `cargo check --tests` and `cargo clippy --workspace --all-targets` are clean.
2. **The LSH-only fixture exposed a live false negative — quarantined.** The authored embeddings-off Python pair (`structural=0.0`, `token_jaccard=0.9297`, 43+-node endpoints, survived every LSH-only floor) rendered **zero** duplication: `buckets::classify_signals` never implemented [CLONE-BUCKETS-ROUTING] row 4 (`structural ≤ 0.01 ∧ token_jaccard ≥ 0.90 → NearlyIdentical`); the triple fell to `LooselySimilar`, which the renderer hides, and `report_render::is_csharp_lsh_type3_near_miss` patches the row for **C# members only**. Per the strict accuracy rule the wrong routing is replaced by a quarantine `panic!` and pinned red by `lsh_only_nearmiss_recall.rs`, watched failing first for the real reason (`clusters_total=0`, `clusters_hidden=1`) and then on the quarantine. The fix must implement the spec row for every language, dissolve the C#-only carve-out, and land the full warm/cold LSH-only equivalence matrix on top — recall equivalence across store states cannot be asserted while the recall path is quarantined.
3. **RSS and disk: fixed, not merely accepted.** The per-render flatten (an owned copy of every signature — ~157 MiB on tokio — plus fingerprints, trees, and the source map) is deleted; the session owns one canonical flat store (`pipeline/session/store.rs`), spliced per change, borrowed by renders. Retention landed as [PIPELINE-INCREMENTAL-RETENTION]: stale tool-version partitions removed after every full pass, provable orphans **kept** under a 2 GiB budget (they are the revert-reuse set the equivalence suite asserts full-hits), orphans-first then oldest-first eviction over it. Pinned by `cache_retention.rs` 3/3 and `fpcache/retention/tests.rs` 6/6. Re-measured (release, pinned tokio): warm 3.31 s / **1,495 MB** (was 1,609 MB), revert full-hits through the retained orphan; full table in the plan's Phase 4 checklist.
4. **Status surface fixed.** `session_config().incremental` now reports the *effective* mode — the request gated by the live config through `PipelineSession::effective_incremental` — and MCP forwards it unchanged. Pinned by `live_session_status.rs` 3/3, including the toggle-under-opt-out transition and the store never being created.
5. **Benchmark rerun and recorded** (plan Phase 4); `deslop-core` lib tests 36/36. The one open accuracy item is the deliberate red pin from step 2.

Two material gaps remain:

1. The targeted integration suites do not currently compile. `crates/deslop/tests/common/corpora.rs:104` calls `cluster_file_set` without importing it; `occurrence_files` in that file and the `corpora::*` re-export in `common/mod.rs` are then rejected as unused. As a result, none of `signature_reuse`, `cache_blob_integrity`, `incremental_equivalence`, `incremental_multilang_golden`, or `incremental_multilang_matrix` ran in this audit.
2. The equivalence corpus still does not force candidate discovery through persisted MinHash signatures. The current incremental fixtures are structurally identical Type-1 pairs, so the structural candidate route can preserve the cluster even if warm signatures stop producing the required LSH collision. The requested embeddings-off, `structural = 0`, LSH-only cold/warm/edit fixture is still absent.

The memory and disk regressions are also still present. They are economics rather than a demonstrated report-correctness defect, but they need an explicit acceptance decision rather than being described as fixed.

## Current status of the previous findings

| Finding | Current status | Evidence |
|---|---|---|
| 1. Corrupted signature payloads served as hits | **Fixed in code; unit-pinned** | Blob format v3 carries a BLAKE3 binding digest over the complete payload. `fpcache::tests::a_flipped_signature_byte_fails_the_binding_digest` passed. The E2E self-heal test exists but could not compile in this audit. |
| 2. Blob not bound to its content address | **Fixed in code; unit-pinned** | The digest is recomputed from the lookup's language, `min_nodes`, source hash, magic, semantic epoch, signature width, and payload before decode. Wrong-address and copied-address unit tests passed. Same-partition swap and cross-language E2E tests exist but could not compile. |
| 3. Store-off accounting made `signature_reuse` red | **Fixed in source; execution blocked** | `ReuseCounters::assert_store_disabled` now asserts `{hits: 0, misses: 0}`, builds every signature, and deliberately skips store-on conservation. `signature_reuse.rs` now contains four scenarios, not the stale “3/3” recorded in the plan, but the suite did not compile. |
| 4. Malformed lengths could panic | **Fixed and unit-pinned** | Blob size is checked before `fs::read`; record, kind, and child counts are proven against remaining bytes before allocation; AST depth and trailing bytes are bounded. All focused malformed-blob unit tests passed. |
| 5. RSS and disk regression | **Partially fixed; still open** | Exact payload capacity removes encoder reallocations, but rendering still clones every per-file signature into a second flat `Vec<Signature>`, alongside cloned fingerprints, trees, sources, and boilerplate ranges. No cache budget, eviction, or orphan GC exists. |
| 6. LSH-only equivalence coverage | **Partially improved; still open** | Cache-integrity assertions and a six-language cold/warm golden now pin exact spans, signals, ranking, and metrics. Those corpora use Type-1 clones and do not prove that persisted signatures preserve LSH-only candidate recall. |
| 7. Semantic invalidation under `0.0.0-dev` | **Fixed and unit-pinned** | `SEMANTIC_EPOCH`, `MAGIC`, and `SIGNATURE_LEN` are bound into the digest; superseded magic and trailing data are rejected. Revision pins passed. |
| 8. Architecture-dependent fallback signatures | **Fixed and unit-pinned** | Byte offsets are widened to `u64` before hashing. `fallback_signature_slots_are_architecture_independent` passed against fixed slots. |
| Workspace opt-out | **Implemented; execution blocked** | `[analysis] incremental = false` gates the effective pipeline setting and store creation. Two config opt-out scenarios exist in `signature_reuse.rs`, including an already-warm store, but could not run because of the shared test-helper compile failure. |

## Why the fixed blob path is now trustworthy

`crates/deslop-core/src/fpcache/blob.rs` now makes trust a prerequisite to decode:

- `MAGIC = 0xC0DE_D180` identifies the digest-bearing layout.
- `SEMANTIC_EPOCH` invalidates meaning changes independently of the permanently reused development package version.
- `binding_digest` covers `(magic, semantic epoch, signature width, min_nodes, language id, source hash, payload)`.
- `FingerprintCache::get` derives the source hash from the bytes supplied by the lookup, checks the file-size ceiling, reads the blob, and passes that lookup address into `decode`.
- `decode` verifies the stored digest before decoding any payload field.
- Decode-side counts are checked against the bytes remaining before `Vec::with_capacity` or `vec![...]`; the recursive tree decoder also enforces `MAX_AST_DEPTH`.
- The decoded payload must consume the blob exactly, and the rehydrated fingerprints are still re-derived from the cached tree before the hit is served.

That closes both demonstrated report-drift paths from the original audit: a changed signature payload cannot alter `token_jaccard` while remaining a hit, and a valid blob moved under another address cannot exchange file spans or parser partitions.

## What the implementation now saves

The implementation still solves the narrow Phase 2 bottleneck it targeted:

- A miss parses, fingerprints, builds signatures once in `build_cached_file`, and persists the bundle.
- A validated hit attaches the stored signatures and increments `signatures_reused` without calling `signatures_for_file`.
- Rendering consumes the attached signatures and builds no per-language signatures.

It does **not** make total run cost proportional to the changed file. Band collision enumeration, candidate scoring, clustering, ranking, content evidence, metrics, and rendering still process a corpus-wide flattened snapshot. The renderer also creates a second owned copy of the signature set on every pass.

The latest numbers recorded in the plan are useful historical evidence, but they were **not re-measured in this audit**:

| Recorded run | Wall time | Peak RSS | Store accounting |
|---|---:|---:|---:|
| `--no-incremental` | 5.88 s | 1,649 MB | 0 hit / 0 miss |
| cold store-on | 6.22 s | 1,665 MB | 0 hit / 758 miss |
| fully warm | 2.91 s | 1,609 MB | 758 hit / 0 miss |
| one-file edit | 2.92 s | not separately recorded | 757 hit / 1 miss |
| revert | 2.94 s | not separately recorded | 758 hit / 0 miss |

The plan records a 185.8 MiB store for 759 blobs, versus the pre-signature 29 MB baseline, and a warm peak about 241 MB above the pre-signature 1,368 MB baseline. One edit/revert cycle leaves one orphaned content-addressed blob. Nothing in the current implementation bounds or collects that growth.

## Remaining correctness-assurance gap: the LSH-only route

The new tests are much stronger than the original suite: damaged stores must render exact truth, and the mixed-language golden pins cluster ids, spans, all signals, ranking, and metric arithmetic across cold and warm passes. That is valuable coverage, but it is not the route-specific proof finding 6 requested.

For every current incremental fixture, an exact structural hash can add the candidate pair independently of LSH. A corrupted or useless warm signature may therefore move `token_jaccard` and still be caught by the exact signal assertions, but a signature defect that only suppresses an LSH collision is not guaranteed to remove the cluster because structural discovery already supplied the pair.

The missing test must use an embeddings-off pair that:

- has `structural = 0` or otherwise cannot enter through the structural candidate path;
- exceeds the LSH node floor and clears the token-Jaccard-only threshold;
- is asserted across cold store fill, fully warm reuse, one-file edit, and revert;
- pins the exact cluster id, files, byte/line spans, bucket, signals, ranking, and metrics;
- proves mixed hit/miss telemetry builds signatures only for the changed file; and
- includes the Python and boilerplate-aware token paths if those are claimed by the equivalence contract.

Until that exists, “warm signatures preserve candidate recall” remains an inference from implementation, not an enforced regression contract.

## Status-surface mismatch still present

Operational behaviour honors the config opt-out, but the live session status does not expose the effective setting. `PipelineSession::effective_incremental` combines the requested session mode with the live config. `AnalysisSession::session_config`, however, returns `self.incremental`, and MCP forwards that field unchanged.

Consequently, a session requested with incremental processing enabled can run uncached under `[analysis] incremental = false` while `session-config.incremental` still reports `true`. `cache_stats` exposes the effect after a pass, but the configuration surface itself is ambiguous. The clean fix is to expose requested and effective values separately, or make the existing field explicitly effective and pin the live config-reload transition.

## Verification performed in this audit

- `cargo test -p deslop-core --lib`: **26 passed, 0 failed**.
- The 26 passing tests include all 12 `fpcache` tests, the architecture-independent fallback-signature vector, and fingerprint-tamper invalidation in the corpus layer.
- Attempted: `cargo test -p deslop --test signature_reuse --test cache_blob_integrity --test incremental_equivalence --test incremental_multilang_golden --test incremental_multilang_matrix`.
- Result: **compile failure before test execution** at `crates/deslop/tests/common/corpora.rs:104`, plus denied unused imports at `corpora.rs:13` and `common/mod.rs:60`.
- The pinned Tokio release benchmark was not rerun because the correctness integration gate did not compile. The table above is explicitly the last measurement recorded in the plan.

## Required next steps

1. Repair the shared integration-test helper so the targeted suites compile, then rerun all five suites and record their actual counts.
2. Add the LSH-only incremental equivalence fixture and watch it prove the cold/warm/edit/revert route.
3. Decide and document whether the measured RSS and disk costs are accepted for this phase. If not, remove the second owned signature copy and add a cache budget plus orphan eviction/GC.
4. Make the status surface distinguish requested incremental mode from the config-gated effective mode.
5. Only then replace this audit's verdict with a merge-ready statement and, if performance claims matter to that decision, rerun the pinned release benchmark.
